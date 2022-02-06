use crate::{AddressError, ByteAddressable, KIB};

pub struct WorkRam {
    ram: [u8; 8 * KIB],
}

impl WorkRam {
    pub fn new() -> Self {
        Self { ram: [0; 8 * KIB] }
    }
}

impl ByteAddressable for WorkRam {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError> {
        let a = address as usize;
        match address {
            0xC000..=0xDFFF => Ok(self.ram[a - 0xC000]),
            _ => Err(AddressError::NonMappedAddress {
                address,
                description: "WorkRam read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError> {
        let a = address as usize;
        match address {
            0xC000..=0xDFFF => Ok(self.ram[a - 0xC000] = byte),
            _ => Err(AddressError::NonMappedAddress {
                address,
                description: "WorkRam write",
            }),
        }
    }
}
