use crate::core::cartridge::Cartridge;
use crate::core::execution::{get_first_opcode, ExecutionError, NextOperation};
use crate::core::interrupt_controller::{Interrupt, InterruptController};
use crate::core::timer::Timer;
use cpu::Cpu;
use execution::instructions::Instruction;
use high_ram::HighRam;
use serial::Serial;
use std::path::Path;
use std::{fs, mem};
use wram::WorkRam;

pub mod cartridge;
pub mod cpu;
pub mod execution;
pub mod high_ram;
mod interrupt_controller;
pub mod serial;
#[cfg(test)]
mod testsupport;
mod timer;
pub mod wram;

const KIB: usize = 1024;

pub struct HexWord(pub u16);

impl std::fmt::Debug for HexWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for HexWord {
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

pub trait MemoryContext {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

pub trait EventContext {
    fn push_event(&mut self, event: ExecutionEvent);
}

pub trait ClockContext {
    fn tick(&mut self);
}

pub trait InterruptContext {
    fn raise_interrupt(&mut self, interrupt: Interrupt);
}

pub trait HandleInterruptContext {
    fn unraise_interrupt(&mut self, interrupt: Interrupt);

    fn should_start_interrupt_routine(&self) -> bool;

    fn get_highest_priority_interrupt(&self) -> Option<Interrupt>;

    fn should_cancel_halt(&self) -> bool;

    fn schedule_ime_enable(&mut self);

    fn enable_interrupts(&mut self);

    fn disable_interrupts(&mut self);
}

#[derive(Debug)]
pub enum ExecutionEvent {
    MemoryRead {
        address: HexWord,
        value: HexByte,
    },
    MemoryWritten {
        address: HexWord,
        value: HexByte,
    },
    ReadFromNonMappedAddress(HexWord),
    WriteToNonMappedAddress(HexWord),
    InstructionExecuted {
        opcode: HexByte,
        instruction: Instruction,
        new_pc: HexWord,
        cpu: Cpu,
    },
    InterruptRoutineStarted,
    InterruptRoutineFinished(Interrupt),
    Halted,
    DebugTrigger,
}

impl std::fmt::Display for ExecutionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFromNonMappedAddress(a) => {
                write!(f, "ReadFromNonMappedAddress({})", a)
            }
            Self::WriteToNonMappedAddress(a) => {
                write!(f, "WriteToNonMappedAddress({})", a)
            }
            Self::InstructionExecuted {
                opcode,
                instruction,
                new_pc,
                cpu,
            } => {
                writeln!(f, "InstructionExecuted")?;
                writeln!(f, "Opcode: {}", opcode)?;
                writeln!(f, "{}", instruction)?;
                writeln!(f, "PC after instruction: {}", new_pc)?;
                writeln!(f, "Registers:")?;
                write!(f, "{}", cpu)
            }
            Self::DebugTrigger => write!(f, "DebugTrigger"),
            Self::MemoryRead { address, value } => {
                write!(f, "MemoryRead{{address: {}, value: {}}}", address, value)
            }
            Self::MemoryWritten { address, value } => {
                write!(f, "MemoryWritten{{address: {}, value: {}}}", address, value)
            }
            Self::InterruptRoutineStarted => write!(f, "InterruptRoutineStarted"),
            Self::InterruptRoutineFinished(interrupt) => {
                write!(f, "InterruptRoutineFinished({})", interrupt)
            }
            Self::Halted => write!(f, "Halted"),
        }
    }
}

pub struct GameboyContext {
    clock_counter: u64,
    cartridge: Box<dyn Cartridge>,
    wram: WorkRam,
    serial: Serial,
    high_ram: HighRam,
    interrupt_controller: InterruptController,
    timer: Timer,
    events: Vec<ExecutionEvent>,
}

impl GameboyContext {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        Self {
            clock_counter: 0,
            cartridge,
            wram: WorkRam::default(),
            serial: Serial::default(),
            high_ram: HighRam::default(),
            interrupt_controller: InterruptController::default(),
            timer: Timer::default(),
            events: Vec::with_capacity(100),
        }
    }
}

impl MemoryContext for GameboyContext {
    fn read(&mut self, addr: u16) -> u8 {
        let result = self
            .wram
            .read(addr)
            .or_else(|| self.serial.read(addr))
            .or_else(|| self.cartridge.read(addr))
            .or_else(|| self.high_ram.read(addr))
            .or_else(|| self.interrupt_controller.read(addr))
            .or_else(|| self.timer.read(addr))
            .unwrap_or_else(|| {
                self.push_event(ExecutionEvent::ReadFromNonMappedAddress(HexWord(addr)));
                0xFF
            });
        self.push_event(ExecutionEvent::MemoryRead {
            address: HexWord(addr),
            value: HexByte(result),
        });
        result
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.wram
            .write(addr, value)
            .or_else(|| self.serial.write(addr, value))
            .or_else(|| self.cartridge.write(addr, value))
            .or_else(|| self.high_ram.write(addr, value))
            .or_else(|| self.interrupt_controller.write(addr, value))
            .or_else(|| self.timer.write(addr, value))
            .unwrap_or_else(|| {
                self.push_event(ExecutionEvent::ReadFromNonMappedAddress(HexWord(addr)));
            });
        self.push_event(ExecutionEvent::MemoryWritten {
            address: HexWord(addr),
            value: HexByte(value),
        })
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

impl ClockContext for GameboyContext {
    fn tick(&mut self) {
        self.clock_counter += 1;
        // Core clock, not CPU clock!
        for _ in 0..4 {
            self.timer.tick(&mut self.interrupt_controller);
        }
        self.interrupt_controller.tick();
    }
}

impl InterruptContext for GameboyContext {
    fn raise_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_controller.raise_interrupt(interrupt)
    }
}

impl HandleInterruptContext for GameboyContext {
    fn unraise_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_controller.unraise_interrupt(interrupt)
    }

    fn should_start_interrupt_routine(&self) -> bool {
        self.interrupt_controller.should_start_interrupt_routine()
    }

    fn get_highest_priority_interrupt(&self) -> Option<Interrupt> {
        self.interrupt_controller.get_highest_priority_interrupt()
    }

    fn should_cancel_halt(&self) -> bool {
        self.interrupt_controller.should_cancel_halt()
    }

    fn schedule_ime_enable(&mut self) {
        self.interrupt_controller.schedule_ime_enable()
    }

    fn enable_interrupts(&mut self) {
        self.interrupt_controller.enable_interrupts()
    }

    fn disable_interrupts(&mut self) {
        self.interrupt_controller.disable_interrupts()
    }
}

pub struct GameBoy {
    cpu: Cpu,
    context: GameboyContext,
    next_operation: NextOperation,
}

impl GameBoy {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        let mut cpu = Cpu::after_boot_rom();
        let mut context = GameboyContext::new(cartridge);
        let initial_opcode = get_first_opcode(&mut cpu, &mut context);
        Self {
            cpu,
            context,
            next_operation: NextOperation::Opcode(initial_opcode),
        }
    }

    pub fn get_elapsed_cycles(&self) -> u64 {
        self.context.clock_counter
    }

    pub fn take_events(&mut self) -> Vec<ExecutionEvent> {
        mem::replace(&mut self.context.events, Vec::with_capacity(100))
    }

    pub fn execute_operation(&mut self) -> Result<(), ExecutionError> {
        let new_opcode =
            execution::handle_next(&mut self.cpu, self.next_operation, &mut self.context)?;
        self.next_operation = new_opcode;
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
        fs::write(
            format!("{}/timer.txt", base),
            format!("{}", self.context.timer),
        )
        .unwrap();
        fs::write(
            format!("{}/interrupt_controller.txt", base),
            format!("{}", self.context.interrupt_controller),
        )
        .unwrap();
        log::info!("Dump done!")
    }
}

pub trait Addressable {
    #[must_use]
    fn read(&self, address: u16) -> Option<u8>;

    #[must_use]
    fn write(&mut self, address: u16, value: u8) -> Option<()>;
}

impl std::fmt::Display for GameBoy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CPU:")?;
        writeln!(f, "{}", self.cpu)?;
        writeln!(f, "Interrupt controller:")?;
        writeln!(f, "{}", self.context.interrupt_controller)?;
        writeln!(f, "Timer:")?;
        writeln!(f, "{}", self.context.timer)
    }
}
