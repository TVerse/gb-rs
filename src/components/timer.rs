use crate::{AddressError, ByteAddressable};

pub struct Timer {
    div: u8,
    tima: u8,
    tma: u8,
    tac: u8,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
        }
    }
}

impl ByteAddressable for Timer {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError> {
        match address {
            0xFF04 => Ok(self.div),
            0xFF05 => Ok(self.tima),
            0xFF06 => Ok(self.tma),
            0xFF07 => Ok(self.tac),
            _ => Err(AddressError::NonMappedAddress {
                address,
                description: "Timer read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError> {
        match address {
            0xFF04 => Ok(self.div = byte),
            0xFF05 => Ok(self.tima = byte),
            0xFF06 => Ok(self.tma = byte),
            0xFF07 => Ok(self.tac = byte),
            _ => Err(AddressError::NonMappedAddress {
                address,
                description: "Timer write",
            }),
        }
    }
}
