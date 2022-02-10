use crate::RawResult;
use crate::{ByteAddressable, GameBoyError};

pub struct HighRam {
    ram: [u8; 127],
}

impl HighRam {
    pub fn new() -> Self {
        Self { ram: [0; 127] }
    }

    pub fn raw(&self) -> &[u8] {
        &self.ram
    }
}

impl Default for HighRam {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteAddressable for HighRam {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        let a = address as usize;
        match address {
            0xFF80..=0xFFFE => Ok(self.ram[a - 0xFF80]),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "HighRam read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()> {
        let a = address as usize;
        match address {
            0xFF80..=0xFFFE => {
                self.ram[a - 0xFF80] = byte;
            }
            _ => {
                return Err(GameBoyError::NonMappedAddress {
                    address,
                    description: "HighRam write",
                })
            }
        };
        Ok(())
    }
}
