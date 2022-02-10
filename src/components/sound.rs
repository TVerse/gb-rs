use crate::ByteAddressable;
use crate::GameBoyError;
use crate::RawResult;

pub struct Sound {}

impl Sound {
    pub fn new() -> Self {
        Self {}
    }
}

impl ByteAddressable for Sound {
    // TODO
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        match address {
            0xFF10..=0xFF3F => Ok(0xFF),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "Sound read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, _byte: u8) -> RawResult<()> {
        match address {
            0xFF10..=0xFF3F => Ok(()),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "Sound write",
            }),
        }
    }
}
