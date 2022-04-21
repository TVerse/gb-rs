use crate::core::cartridge::Cartridge;
use crate::core::cpu::instructions::Instruction;
use crate::core::cpu::registers::Registers;
use crate::core::cpu::{Cpu, CpuError};
use crate::core::serial::Serial;
use crate::core::wram::WorkRam;
use crate::core::ExecutionEvent::ReadFromNonMappedAddress;
use crate::ExecutionEvent::{MemoryRead, MemoryWritten};
use std::path::Path;
use std::{fs, mem};

pub mod cartridge;
pub mod cpu;
mod serial;
#[cfg(test)]
mod testsupport;
mod wram;

const KIB: usize = 1024;

pub struct HexAddress(pub u16);

impl std::fmt::Debug for HexAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for HexAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#06x}", self.0)
    }
}

pub struct HexByte(pub u8);

impl std::fmt::Debug for HexByte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for HexByte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#04x}", self.0)
    }
}

pub trait ExecuteContext {
    fn tick(&mut self);
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

pub trait EventContext {
    fn push_event(&mut self, event: ExecutionEvent);
}

#[derive(Debug)]
pub enum ExecutionEvent {
    MemoryRead(HexAddress),
    MemoryWritten(HexAddress),
    ReadFromNonMappedAddress(HexAddress),
    WriteToNonMappedAddress(HexAddress),
    InstructionExecuted {
        opcode: HexByte,
        instruction: Instruction,
        new_pc: HexAddress,
        registers: Registers,
    },
    DebugTrigger,
}

impl std::fmt::Display for ExecutionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadFromNonMappedAddress(a) => write!(f, "ReadFromNonMappedAddress({})", a),
            ExecutionEvent::WriteToNonMappedAddress(a) => {
                write!(f, "WriteToNonMappedAddress({})", a)
            }
            ExecutionEvent::InstructionExecuted {
                opcode,
                instruction,
                new_pc,
                registers,
            } => {
                writeln!(f, "InstructionExecuted")?;
                writeln!(f, "Opcode: {}", opcode)?;
                writeln!(f, "{}", instruction)?;
                writeln!(f, "PC after instruction: {}", new_pc)?;
                writeln!(f, "Registers:")?;
                write!(f, "{}", registers)
            }
            ExecutionEvent::DebugTrigger => write!(f, "DebugTrigger"),
            MemoryRead(addr) => write!(f, "MemoryRead({})", addr),
            MemoryWritten(addr) => write!(f, "MemoryWritten({})", addr),
        }
    }
}

trait Addressable {
    #[must_use]
    fn read(&self, address: u16) -> Option<u8>;

    #[must_use]
    fn write(&mut self, address: u16, value: u8) -> Option<()>;
}

pub struct GameboyContext {
    clock_counter: u64,
    cartridge: Box<dyn Cartridge>,
    wram: WorkRam,
    serial: Serial,
    events: Vec<ExecutionEvent>,
}

impl GameboyContext {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        Self {
            clock_counter: 0,
            cartridge,
            wram: WorkRam::default(),
            serial: Serial::default(),
            events: Vec::with_capacity(100),
        }
    }
}

impl ExecuteContext for GameboyContext {
    fn tick(&mut self) {
        self.clock_counter += 1;
    }

    fn read(&mut self, addr: u16) -> u8 {
        self.push_event(MemoryRead(HexAddress(addr)));
        self.wram
            .read(addr)
            .or_else(|| self.serial.read(addr))
            .or_else(|| self.cartridge.read(addr))
            .unwrap_or_else(|| {
                self.push_event(ReadFromNonMappedAddress(HexAddress(addr)));
                0xFF
            })
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.push_event(MemoryWritten(HexAddress(addr)));
        self.wram
            .write(addr, value)
            .or_else(|| self.serial.write(addr, value))
            .unwrap_or_else(|| {
                self.push_event(ReadFromNonMappedAddress(HexAddress(addr)));
            });
    }
}

impl EventContext for Vec<ExecutionEvent> {
    fn push_event(&mut self, event: ExecutionEvent) {
        self.push(event)
    }
}

impl EventContext for GameboyContext {
    fn push_event(&mut self, event: ExecutionEvent) {
        self.events.push_event(event)
    }
}

pub struct GameBoy {
    cpu: Cpu,
    context: GameboyContext,
    next_opcode: u8,
}

impl GameBoy {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        let mut cpu = Cpu::after_boot_rom();
        let mut context = GameboyContext::new(cartridge);
        let initial_opcode = cpu.get_first_opcode(&mut context);
        Self {
            cpu,
            context,
            next_opcode: initial_opcode,
        }
    }

    pub fn get_elapsed_cycles(&self) -> u64 {
        self.context.clock_counter
    }

    pub fn take_events(&mut self) -> Vec<ExecutionEvent> {
        mem::replace(&mut self.context.events, Vec::with_capacity(100))
    }

    pub fn execute_instruction(&mut self) -> Result<(), CpuError> {
        let new_opcode = self
            .cpu
            .decode_execute_fetch(self.next_opcode, &mut self.context)?;
        self.next_opcode = new_opcode;
        Ok(())
    }

    pub fn get_serial_out(&mut self) -> Option<u8> {
        self.context.serial.get_data()
    }

    pub fn dump(&mut self, base: &str) {
        let p = Path::new(base);
        if !p.exists() {
            fs::create_dir(p).unwrap();
        }
        log::info!("Dumping...");
        let mut v = Vec::with_capacity(64 * KIB);
        for i in 0..64 * KIB {
            v.push(self.context.read(i as u16));
        }
        fs::write(format!("{}/cpu.txt", base), format!("{}", self.cpu)).unwrap();
        fs::write(format!("{}/address_space.bin", base), v).unwrap();
        log::info!("Dump done!")
    }
}
