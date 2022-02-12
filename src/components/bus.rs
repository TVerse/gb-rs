use crate::components::cartridge::Cartridge;
use crate::components::high_ram::HighRam;
use crate::components::interrupt_controller::InterruptController;
use crate::components::ppu::Ppu;
use crate::components::serial::Serial;
use crate::components::sound::Sound;
use crate::components::timer::Timer;
use crate::components::work_ram::WorkRam;
use crate::components::ByteAddressable;

use crate::components::controller::Controller;
use crate::{GameBoyError, RawResult, KIB};

pub trait Bus {
    fn read_byte(&self, address: u16) -> RawResult<u8>;
    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()>;

    fn read_word(&self, address: u16) -> RawResult<u16> {
        let lower = self.read_byte(address)? as u16;
        let higher = self.read_byte(address.wrapping_add(1))? as u16;

        Ok((higher << 8) | lower)
    }

    fn write_word(&mut self, address: u16, word: u16) -> RawResult<()> {
        let lower = word as u8;
        let higher = (word >> 8) as u8;

        self.write_byte(address, lower)?;
        self.write_byte(address.wrapping_add(1), higher)
    }

    fn memory_dump(&self) -> [u8; 64 * KIB] {
        let v: Vec<u8> = (0..=0xFFFF)
            .map(|addr| self.read_byte(addr).unwrap_or(0xFF))
            .collect();
        // This vec is always 64 KIB long
        v.try_into().unwrap()
    }
}

pub struct RealBus {
    pub cartridge: Box<dyn Cartridge>,
    pub ppu: Ppu,
    pub serial: Serial,
    pub work_ram: WorkRam,
    pub interrupt_controller: InterruptController,
    pub high_ram: HighRam,
    pub timer: Timer,
    pub sound: Sound,
    pub controller: Controller,
}

impl RealBus {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        Self {
            cartridge,
            ppu: Ppu::new(),
            serial: Serial::new(),
            work_ram: WorkRam::new(),
            interrupt_controller: InterruptController::new(),
            high_ram: HighRam::new(),
            timer: Timer::new(),
            sound: Sound::new(),
            controller: Controller::new(),
        }
    }

    pub fn step(&mut self, cycles: usize) -> Option<u8> {
        self.serial.step(cycles, &mut self.interrupt_controller)
    }
}

impl Bus for RealBus {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        self.cartridge
            .read_byte(address)
            .or_else(|_e| self.ppu.read_byte(address))
            .or_else(|_e| self.serial.read_byte(address))
            .or_else(|_e| self.work_ram.read_byte(address))
            .or_else(|_e| self.interrupt_controller.read_byte(address))
            .or_else(|_e| self.high_ram.read_byte(address))
            .or_else(|_e| self.timer.read_byte(address))
            .or_else(|_e| self.sound.read_byte(address))
            .or_else(|_e| self.controller.read_byte(address))
            .map_err(|_e| GameBoyError::NonMappedAddress {
                address,
                description: "RealBus read",
            })
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()> {
        self.cartridge
            .write_byte(address, byte)
            .or_else(|_e| self.ppu.write_byte(address, byte))
            .or_else(|_e| self.serial.write_byte(address, byte))
            .or_else(|_e| self.work_ram.write_byte(address, byte))
            .or_else(|_e| self.interrupt_controller.write_byte(address, byte))
            .or_else(|_e| self.high_ram.write_byte(address, byte))
            .or_else(|_e| self.timer.write_byte(address, byte))
            .or_else(|_e| self.sound.write_byte(address, byte))
            .or_else(|_e| self.controller.write_byte(address, byte))
            .map_err(|_e| GameBoyError::NonMappedAddress {
                address,
                description: "RealBus write",
            })
    }
}

#[cfg(test)]
#[derive(Debug)]
pub struct FlatBus {
    pub mem: Vec<u8>,
}

#[cfg(test)]
impl Bus for FlatBus {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        self.mem
            .get(address as usize)
            .copied()
            .ok_or(GameBoyError::NonMappedAddress {
                address,
                description: "FlatBus read",
            })
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()> {
        self.mem.get_mut(address as usize).map(|b| *b = byte).ok_or(
            GameBoyError::NonMappedAddress {
                address,
                description: "FlatBus read",
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn little_endian_reads() {
        let bus = FlatBus {
            mem: vec![0x34, 0x12],
        };

        assert_eq!(bus.read_word(0x0000).unwrap(), 0x1234);
    }
}
