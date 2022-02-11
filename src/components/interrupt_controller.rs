use crate::RawResult;
use crate::{ByteAddressable, GameBoyError};

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

    pub fn set_serial_interrupt(&mut self) {
        self.interrupt_flags |= 0b00000100
    }
}

impl Default for InterruptController {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteAddressable for InterruptController {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        match address {
            0xFF0F => Ok(self.interrupt_flags),
            0xFFFF => Ok(self.interrupt_enable),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "InterruptController read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()> {
        match address {
            0xFF0F => {
                self.interrupt_flags = byte;
            }
            0xFFFF => {
                self.interrupt_enable = byte;
            }
            _ => {
                return Err(GameBoyError::NonMappedAddress {
                    address,
                    description: "InterruptController write",
                })
            }
        };
        Ok(())
    }
}
