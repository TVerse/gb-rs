use crate::{ByteAddressable, GameBoyError, RawResult, KIB};

#[derive(Debug)]
pub struct WorkRam {
    ram: [u8; 8 * KIB],
}

impl WorkRam {
    pub fn new() -> Self {
        Self { ram: [0; 8 * KIB] }
    }

    pub fn raw(&self) -> &[u8] {
        &self.ram
    }
}

impl Default for WorkRam {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteAddressable for WorkRam {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        let a = address as usize;
        match address {
            0xC000..=0xDFFF => Ok(self.ram[a - 0xC000]),
            0xE000..=0xFDFF => Ok(self.ram[a - 0xE000]),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "WorkRam read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()> {
        let a = address as usize;
        match address {
            0xC000..=0xDFFF => self.ram[a - 0xC000] = byte,
            0xE000..=0xFDFF => self.ram[a - 0xE000] = byte,
            _ => {
                return Err(GameBoyError::NonMappedAddress {
                    address,
                    description: "WorkRam write",
                })
            }
        };
        Ok(())
    }
}
