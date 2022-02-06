use crate::{AddressError, ByteAddressable};

pub struct Sound {}

impl Sound {
    pub fn new() -> Self {
        Self {}
    }
}

impl ByteAddressable for Sound {
    // TODO
    fn read_byte(&self, address: u16) -> Result<u8, AddressError> {
        match address {
            0xFF10..=0xFF3F => Ok(0xFF),
            _ => Err(AddressError::NonMappedAddress {
                address,
                description: "Sound read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, _byte: u8) -> Result<(), AddressError> {
        match address {
            0xFF10..=0xFF3F => Ok(()),
            _ => Err(AddressError::NonMappedAddress {
                address,
                description: "Sound write",
            }),
        }
    }
}
