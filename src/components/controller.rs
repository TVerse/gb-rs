use crate::{ByteAddressable, GameBoyError};

pub struct Controller {}

impl Controller {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteAddressable for Controller {
    fn read_byte(&self, address: u16) -> crate::RawResult<u8> {
        match address {
            0xFF00 => Ok(0x00),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "Controller read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, _byte: u8) -> crate::RawResult<()> {
        match address {
            0xFF00 => Ok(()),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "Controller write",
            }),
        }
    }
}
