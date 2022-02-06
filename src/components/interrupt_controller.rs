use crate::{AddressError, ByteAddressable};

pub struct InterruptController {
    interrupt_flags: u8,
    interrupt_enable: u8,
}

impl InterruptController {
    pub fn new() -> Self {
        Self {
            interrupt_flags: 0,
            interrupt_enable: 0,
        }
    }
}

impl ByteAddressable for InterruptController {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError> {
        match address {
            0xFF0F => Ok(self.interrupt_flags),
            0xFFFF => Ok(self.interrupt_enable),
            _ => Err(AddressError::NonMappedAddress {
                address,
                description: "InterruptController read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError> {
        match address {
            0xFF0F => Ok(self.interrupt_flags = byte),
            0xFFFF => Ok(self.interrupt_enable = byte),
            _ => Err(AddressError::NonMappedAddress {
                address,
                description: "InterruptController write",
            }),
        }
    }
}
