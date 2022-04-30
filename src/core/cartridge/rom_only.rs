use crate::core::cartridge::Cartridge;
use crate::core::{Addressable, KIB};

#[derive(Debug, Clone)]
pub struct RomOnlyCartridge {
    rom: [u8; 32 * KIB],
}

impl RomOnlyCartridge {
    pub fn new(rom: [u8; 32 * KIB]) -> Self {
        Self { rom }
    }
}

impl Addressable for RomOnlyCartridge {
    fn read(&self, address: u16) -> Option<u8> {
        self.rom.get(address as usize).copied()
    }

    fn write(&mut self, _address: u16, _byte: u8) -> Option<()> {
        None
    }
}

impl Cartridge for RomOnlyCartridge {}
