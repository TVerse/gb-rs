use crate::components::cartridge::Cartridge;
use crate::components::ppu::Ppu;
use crate::components::{AddressError, ByteAddressable};
use crate::Serial;

pub trait Bus {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError>;
    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError>;

    fn read_word(&self, address: u16) -> Result<u16, AddressError> {
        let low = self.read_byte(address)? as u16;
        let high = self.read_byte(address + 1)? as u16;

        Ok((high << 8) | low)
    }
}

impl<T> ByteAddressable for T
where
    T: Bus,
{
    fn read_byte(&self, address: u16) -> Result<u8, AddressError> {
        Bus::read_byte(self, address)
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError> {
        Bus::write_byte(self, address, byte)
    }
}

pub struct RealBus<'a> {
    pub cartridge: &'a mut dyn Cartridge,
    pub ppu: &'a mut Ppu,
    pub serial: &'a mut Serial,
}

impl<'a> Bus for RealBus<'a> {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError> {
        self.cartridge
            .read_byte(address)
            .or_else(|_e| self.ppu.read_byte(address))
            .or_else(|_e| self.serial.read_byte(address))
            .or_else(|_e| {
                Err(AddressError::NonMappedAddress {
                    address,
                    description: "RealBus read",
                })
            })
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError> {
        self.cartridge
            .write_byte(address, byte)
            .or_else(|_e| self.ppu.write_byte(address, byte))
            .or_else(|_e| self.serial.write_byte(address, byte))
            .or_else(|_e| {
                Err(AddressError::NonMappedAddress {
                    address,
                    description: "RealBus write",
                })
            })
    }
}

#[cfg(test)]
pub struct FlatBus {
    pub mem: Vec<u8>,
}

#[cfg(test)]
impl Bus for FlatBus {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError> {
        self.mem
            .get(address as usize)
            .copied()
            .ok_or(AddressError::NonMappedAddress {
                address,
                description: "FlatBus read",
            })
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError> {
        self.mem.get_mut(address as usize).map(|b| *b = byte).ok_or(
            AddressError::NonMappedAddress {
                address,
                description: "FlatBus read",
            },
        )
    }
}
