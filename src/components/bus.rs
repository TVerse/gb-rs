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
use crate::{GameBoyError, RawResult};

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
}

pub struct RealBus<'a> {
    pub cartridge: &'a mut dyn Cartridge,
    pub ppu: &'a mut Ppu,
    pub serial: &'a mut Serial,
    pub work_ram: &'a mut WorkRam,
    pub interrupt_controller: &'a mut InterruptController,
    pub high_ram: &'a mut HighRam,
    pub timer: &'a mut Timer,
    pub sound: &'a mut Sound,
    pub controller: &'a mut Controller,
}

impl<'a> Bus for RealBus<'a> {
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
