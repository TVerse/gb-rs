mod components;
mod execution;

use crate::components::bus::{Bus, RealBus};
use crate::components::cartridge::Cartridge;
use crate::components::ppu::Ppu;
use crate::components::serial::Serial;
use crate::components::{AddressError, ByteAddressable};
use crate::execution::{ExecutingCpu, ExecutionError};
use components::cpu::Cpu;
use thiserror::Error;

use crate::components::interrupt_controller::InterruptController;
use crate::components::wram::WorkRam;
pub use components::cartridge::RomOnlyCartridge;
use crate::components::high_ram::HighRam;

pub type Result<T> = std::result::Result<T, GameBoyError>;

const KIB: usize = 1024;

#[derive(Error, Debug)]
pub enum GameBoyError {
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error(transparent)]
    Address(#[from] AddressError),
}

pub struct GameBoy {
    cpu: Cpu,
    ppu: Ppu,
    cartridge: Box<dyn Cartridge>,
    serial: Serial,
    work_ram: WorkRam,
    interrupt_controller: InterruptController,
    high_ram: HighRam
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
            high_ram: HighRam::new()
        }
    }

    pub fn step(&mut self) -> Result<()> {
        let mut bus = RealBus {
            cartridge: self.cartridge.as_mut(),
            ppu: &mut self.ppu,
            serial: &mut self.serial,
            work_ram: &mut self.work_ram,
            interrupt_controller: &mut self.interrupt_controller,
            high_ram: &mut self.high_ram,
        };
        let mut executing_cpu = ExecutingCpu::new(&mut self.cpu, &mut bus);

        executing_cpu.step()?;
        Ok(())
    }

    pub fn get_serial(&mut self) -> Result<Option<u8>> {
        if self.serial.read_byte(0xFF02)? == 81 {
            self.serial.write_byte(0xFF02, 0x01)?;
            self.serial
                .write_byte(0xFF0F, self.serial.read_byte(0xFF04)? | 0x04)?;

            Ok(Some(self.serial.read_byte(0xFF01)?))
        } else {
            Ok(None)
        }
    }
}
