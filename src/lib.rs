extern crate core;

mod components;
mod execution;

use crate::components::bus::Bus;
use crate::components::bus::RealBus;
use crate::components::cartridge::Cartridge;
use crate::components::ByteAddressable;
use components::cpu::Cpu;
use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::execution::{execute_instruction, fetch_and_decode, handle_interrupt};
pub use components::cartridge::parse_into_cartridge;
pub use components::cpu::{Register16, Register8};
pub use execution::instructions::{CommonRegister, Instruction, JumpCondition, ResetVector, Immediate8, Immediate16};
use crate::components::cpu::State;
use crate::components::interrupt_controller::Interrupt;
use crate::Instruction::Push;

pub type RawResult<T> = std::result::Result<T, GameBoyError>;

pub type Result<T> = std::result::Result<T, GameBoyExecutionError>;

const KIB: usize = 1024;

#[derive(Error, Debug)]
pub struct GameBoyExecutionError {
    error: GameBoyError,
    execution_context: Option<ExecutionContext>,
}

impl std::fmt::Display for GameBoyExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Error: {}", self.error)?;
        if let Some(ref context) = self.execution_context {
            write!(f, "Context: {}", context)
        } else {
            write!(f, "No context available")
        }
    }
}

impl From<GameBoyError> for GameBoyExecutionError {
    fn from(error: GameBoyError) -> Self {
        Self {
            error,
            execution_context: None,
        }
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

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub instruction: Instruction,
    pub pc: u16,
    pub three_bytes_before_pc: [Option<u8>; 3],
    pub three_bytes_at_pc: [Option<u8>; 3],
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
        write!(
            f,
            "Instruction: {} (bytes: {}, cycles: {})",
            self.instruction,
            self.instruction.bytes(),
            if let Some(c) = self.instruction.cycles_branch_not_taken() {
                format!("{}/{}", self.instruction.cycles(), c)
            } else {
                self.instruction.cycles().to_string()
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_type: StepType,
    pub serial_byte: Option<u8>,
}

#[derive(Debug, Clone)]
pub enum StepType {
    InstructionExecuted(ExecutionContext),
    InterruptStarted,
    Halted,
}

#[derive(Debug)]
pub struct GameBoy {
    pub cpu: Cpu,
    pub bus: RealBus,
}

impl GameBoy {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        Self {
            cpu: Cpu::new(),
            bus: RealBus::new(cartridge),
        }
    }

    pub fn step(&mut self) -> Result<StepResult> {
        let interrupt = self.potential_interrupt()?;


        if let Some(interrupt_vector) = interrupt {
            if self.cpu.get_state() == State::Halted {
                self.cpu.set_state(State::Running);
            }
            if self.cpu.interrupts_enabled() {
                let cycles = handle_interrupt(&mut self.cpu, &mut self.bus, interrupt_vector)?;
                let serial_byte = self.bus.step(cycles);
                return Ok(StepResult {
                    step_type: StepType::InterruptStarted,
                    serial_byte,
                });
            }
        }

        match self.cpu.get_state() {
            State::Running => {
                let decode_context =
                    fetch_and_decode(&self.cpu, &self.bus).map_err(|error| GameBoyExecutionError {
                        error,
                        execution_context: None,
                    })?;

                let cycles = execute_instruction(&mut self.cpu, &mut self.bus, decode_context.instruction)
                    .map_err(|error| GameBoyExecutionError {
                        error,
                        execution_context: Some(ExecutionContext {
                            instruction: decode_context.instruction,
                            pc: decode_context.pc,
                            three_bytes_before_pc: decode_context.three_bytes_before_pc,
                            three_bytes_at_pc: decode_context.three_bytes_at_pc,
                        }),
                    })?;

                let serial_byte = self.bus.step(cycles);

                Ok(StepResult {
                    step_type: StepType::InstructionExecuted(ExecutionContext {
                        instruction: decode_context.instruction,
                        pc: decode_context.pc,
                        three_bytes_before_pc: decode_context.three_bytes_before_pc,
                        three_bytes_at_pc: decode_context.three_bytes_at_pc,
                    }),
                    serial_byte,
                })
            }
            State::Halted => {
                let serial_byte = self.bus.step(1);
                Ok(StepResult {
                    step_type: StepType::Halted,
                    serial_byte
                })
            }
            State::Stopped => todo!("State::Stopped")
        }
    }

    pub fn potential_interrupt(&self) -> RawResult<Option<Interrupt>> {
        let valid_interrupts = self.bus.read_byte(0xFF0F)? & self.bus.read_byte(0xFFFF)?;
        let res = if valid_interrupts & 0x01 != 0 {
            Some(Interrupt::VBlank)
        } else if valid_interrupts & 0x02 != 0 {
            Some(Interrupt::LcdStat)
        } else if valid_interrupts & 0x04 != 0 {
            Some(Interrupt::Timer)
        } else if valid_interrupts & 0x08 != 0 {
            Some(Interrupt::Serial)
        } else if valid_interrupts & 0x10 != 0 {
            Some(Interrupt::Joypad)
        } else {
            None
        };
        Ok(res)
    }

    pub fn dump(&self, base: &str) {
        let p = Path::new(base);
        if !p.exists() {
            fs::create_dir(p).unwrap();
        }

        log::info!("Dumping...");
        fs::write(format!("{}/cpu.txt", base), format!("{}", self.cpu)).unwrap();
        fs::write(
            format!("{}/address_space.bin", base),
            self.bus.memory_dump(),
        )
        .unwrap();
        log::info!("Dump done!")
    }
}
