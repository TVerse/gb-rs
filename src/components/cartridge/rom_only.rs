use crate::{Cartridge, GameBoyError, RawResult, KIB};

pub struct RomOnlyCartridge {
    rom: [u8; 32 * KIB],
}

impl RomOnlyCartridge {
    pub fn new(rom: [u8; 32 * KIB]) -> Self {
        Self { rom }
    }
}

impl Cartridge for RomOnlyCartridge {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        self.rom
            .get(address as usize)
            .copied()
            .ok_or(GameBoyError::NonMappedAddress {
                address,
                description: "RomOnlyCartridge ROM read",
            })
    }

    fn write_byte(&mut self, address: u16, _byte: u8) -> RawResult<()> {
        Err(GameBoyError::NonMappedAddress {
            address,
            description: "RomOnlyCartridge write",
        })
    }
}
