extern crate core;

mod components;
mod execution;

use crate::components::bus::RealBus;
use crate::components::cartridge::Cartridge;
use crate::components::ppu::Ppu;
use crate::components::serial::Serial;
use crate::components::ByteAddressable;
use components::cpu::Cpu;
use std::fs;
use std::path::Path;
use thiserror::Error;

use crate::components::controller::Controller;
use crate::components::high_ram::HighRam;
use crate::components::interrupt_controller::InterruptController;
use crate::components::sound::Sound;
use crate::components::timer::Timer;
use crate::components::wram::WorkRam;
use crate::execution::instructions::Instruction;
use crate::execution::{execute, fetch_and_decode, DecodeContext, DecodeResult};
pub use components::cartridge::parse_into_cartridge;

pub type RawResult<T> = std::result::Result<T, GameBoyError>;

pub type Result<T> = std::result::Result<T, GameBoyExecutionError>;

const KIB: usize = 1024;

#[derive(Error, Debug)]
pub struct GameBoyExecutionError {
    error: GameBoyError,
    execution_context: ExecutionContext,
}

impl std::fmt::Display for GameBoyExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Error: {}", self.error)?;
        write!(f, "Context: {}", self.execution_context)
    }
}

#[derive(Error, Debug, Clone)]
pub enum GameBoyError {
    #[error("Tried to use nonmapped address {address:#06x}: {description}")]
    NonMappedAddress {
        address: u16,
        description: &'static str,
    },
    #[error("Unknown opcode {opcode:#04x} at pc {pc:#06x}")]
    InvalidOpcode { opcode: u8, pc: u16 },
}

#[derive(Error, Debug)]
pub struct ExecutionContext {
    pub pc: u16,
    pub three_bytes_before_pc: [Option<u8>; 3],
    pub three_bytes_at_pc: [Option<u8>; 3],
    pub cpu: Cpu,
    pub instruction: Option<Instruction>,
}

pub struct BorrowedExecutionContext<'a> {
    pub pc: u16,
    pub three_bytes_before_pc: [Option<u8>; 3],
    pub three_bytes_at_pc: [Option<u8>; 3],
    pub cpu: &'a Cpu,
    pub instruction: Instruction,
}

impl std::fmt::Display for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "pc: {:#06x}", self.pc)?;
        write!(f, "3 bytes before pc:")?;
        for b in self.three_bytes_before_pc {
            if let Some(b) = b {
                write!(f, "{:#04x}", b)?
            } else {
                write!(f, "0xxx")?
            }
            write!(f, " ")?
        }
        writeln!(f)?;
        write!(f, "3 bytes at pc:")?;
        for b in self.three_bytes_at_pc {
            if let Some(b) = b {
                write!(f, "{:#04x}", b)?
            } else {
                write!(f, "0xxx")?
            }
            write!(f, " ")?
        }
        writeln!(f)?;
        if let Some(i) = self.instruction {
            writeln!(
                f,
                "Instruction: {:?} (bytes: {}, cycles: {})",
                i,
                i.bytes(),
                i.cycles()
            )?;
        } else {
            writeln!(f, "Instruction: unknown")?;
        }
        writeln!(f, "CPU state (possibly partially through executing!):")?;
        write!(f, "{}", self.cpu)
    }
}

pub struct GameBoy {
    pub cpu: Cpu,
    pub ppu: Ppu,
    pub cartridge: Box<dyn Cartridge>,
    pub serial: Serial,
    pub work_ram: WorkRam,
    pub interrupt_controller: InterruptController,
    pub high_ram: HighRam,
    pub timer: Timer,
    pub sound: Sound,
    pub controller: Controller,
}

impl GameBoy {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        Self {
            cpu: Cpu::new(),
            ppu: Ppu::new(),
            cartridge,
            serial: Serial::new(),
            work_ram: WorkRam::new(),
            interrupt_controller: InterruptController::new(),
            high_ram: HighRam::new(),
            timer: Timer::new(),
            sound: Sound::new(),
            controller: Controller::new(),
        }
    }

    fn with_context(
        &self,
        error: GameBoyError,
        context: &DecodeContext,
        instruction: RawResult<Instruction>,
    ) -> GameBoyExecutionError {
        GameBoyExecutionError {
            error,
            execution_context: ExecutionContext {
                pc: context.pc,
                three_bytes_before_pc: context.three_bytes_before_pc,
                three_bytes_at_pc: context.three_bytes_at_pc,
                cpu: self.cpu.clone(),
                instruction: instruction.ok(),
            },
        }
    }

    pub fn step(&mut self, verbose: bool) -> Result<u64> {
        let mut bus = RealBus {
            cartridge: self.cartridge.as_mut(),
            ppu: &mut self.ppu,
            serial: &mut self.serial,
            work_ram: &mut self.work_ram,
            interrupt_controller: &mut self.interrupt_controller,
            high_ram: &mut self.high_ram,
            timer: &mut self.timer,
            sound: &mut self.sound,
            controller: &mut self.controller,
        };

        let DecodeResult {
            instruction,
            context,
        } = fetch_and_decode(&self.cpu, &bus);
        // let testing_instruction = self.cpu.get_register16(Register16::PC) == 0xDEF8;
        // if testing_instruction {
        //     log::info!("About to test instruction {:?}", instruction.clone().unwrap());
        //     log::info!("CPU before:\n{}", self.cpu);
        // }
        let _cycles = instruction
            .clone()
            .and_then(|i| execute(&mut self.cpu, &mut bus, i))
            .map_err(|e| self.with_context(e, &context, instruction.clone()))?;
        // if testing_instruction {
        //     log::info!("CPU after:\n{}", self.cpu)
        // }
        if verbose {
            log::info!(
                "Execution context:\n{}",
                ExecutionContext {
                    pc: context.pc,
                    three_bytes_before_pc: context.three_bytes_before_pc,
                    three_bytes_at_pc: context.three_bytes_at_pc,
                    cpu: self.cpu.clone(),
                    instruction: instruction.ok(),
                }
            );
        }
        Ok(self.cpu.get_instruction_counter())
    }

    pub fn get_serial(&mut self) -> RawResult<Option<u8>> {
        if self.serial.read_byte(0xFF02)? == 0x81 {
            self.serial.write_byte(0xFF02, 0x01)?;
            self.interrupt_controller
                .write_byte(0xFF0F, self.interrupt_controller.read_byte(0xFF0F)? | 0x04)?;

            Ok(Some(self.serial.read_byte(0xFF01)?))
        } else {
            Ok(None)
        }
    }

    pub fn dump(&self, base: &str) {
        let p = Path::new(base);
        if !p.exists() {
            fs::create_dir(p).unwrap();
        }

        log::info!(
            "Dumping, instruction {}",
            self.cpu.get_instruction_counter()
        );

        fs::write(format!("{}/cpu.txt", base), format!("{}", self.cpu)).unwrap();
        fs::write(format!("{}/work_ram.bin", base), self.work_ram.raw()).unwrap();
        fs::write(format!("{}/high_ram.bin", base), self.high_ram.raw()).unwrap();
        fs::write(format!("{}/vram.bin", base), self.ppu.vram_raw()).unwrap();
        fs::write(format!("{}/oam.bin", base), self.ppu.oam_raw()).unwrap();
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }
}
