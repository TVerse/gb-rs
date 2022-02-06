use crate::{AddressError, ByteAddressable};

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

impl ByteAddressable for HighRam {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError> {
        let a = address as usize;
        match address {
            0xFF80..=0xFFFE => Ok(self.ram[a - 0xFF80]),
            _ => Err(AddressError::NonMappedAddress {
                address,
                description: "HighRam read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError> {
        let a = address as usize;
        match address {
            0xFF80..=0xFFFE => Ok(self.ram[a - 0xFF80] = byte),
            _ => Err(AddressError::NonMappedAddress {
                address,
                description: "HighRam write",
            }),
        }
    }
}
