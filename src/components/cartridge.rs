use crate::components::AddressError;
use crate::KIB;

pub trait Cartridge {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError>;
    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError>;
}

pub struct RomOnlyCartridge {
    rom: [u8; 32 * KIB],
}

impl RomOnlyCartridge {
    pub fn new(rom: [u8; 32 * KIB]) -> Self {
        Self { rom }
    }
}

impl Cartridge for RomOnlyCartridge {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError> {
        self.rom
            .get(address as usize)
            .copied()
            .ok_or_else(|| AddressError::NonMappedAddress {
                address,
                description: "RomOnlyCartridge ROM read",
            })
    }

    fn write_byte(&mut self, address: u16, _byte: u8) -> Result<(), AddressError> {
        Err(AddressError::NonMappedAddress {
            address,
            description: "RomOnlyCartridge write",
        })
    }
}
