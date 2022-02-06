use crate::components::Memory;
use crate::KIB;

// TODO mappers
pub struct CartridgeRom {
    rom: [u8; 2 * 16 * KIB],
}

impl CartridgeRom {
    pub fn new(rom: [u8; 2 * 16 * KIB]) -> Self {
        Self { rom }
    }
}

impl Memory for CartridgeRom {
    fn read_byte(&self, address: u16) -> u8 {
        self.rom[address as usize]
    }

    fn write_byte(&mut self, _address: u16, _byte: u8) {
        // ignore
    }
}
