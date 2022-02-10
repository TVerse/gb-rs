use crate::{Cartridge, GameBoyError, RawResult, KIB};

#[derive(Debug)]
pub struct Mbc1Cartridge {
    rom: Vec<[u8; 16 * KIB]>,
    rom_bank: u8,
}

impl Mbc1Cartridge {
    pub fn new(rom: Vec<u8>) -> Self {
        let chunks = rom.chunks_exact(16 * KIB);
        assert!(chunks.remainder().is_empty());
        let rom: Vec<_> = chunks.map(|c| c.try_into().unwrap()).collect();
        Self { rom, rom_bank: 1 }
    }
}

impl Cartridge for Mbc1Cartridge {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        match address {
            0x0000..=0x3FFF => Ok(self.rom[0][address as usize]),
            0x4000..=0x7FFF => Ok(self.rom[self.rom_bank as usize][(address as usize) - 0x4000]),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "RomOnlyCartridge ROM read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()> {
        match address {
            0x2000..=0x3FFF => {
                log::trace!("Swapping ROM bank from {} to {}", self.rom_bank, byte);
                self.rom_bank = byte;
                Ok(())
            }
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "RomOnlyCartridge write",
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_32k() {
        let bank_0 = [0x55; 16 * KIB];
        let bank_1 = [0xAA; 16 * KIB];
        let rom = {
            let mut tmp = bank_0.to_vec();
            tmp.extend(bank_1.to_vec());
            tmp
        };
        let rom = Mbc1Cartridge::new(rom);

        assert_eq!(rom.rom_bank, 1);
        assert_eq!(&rom.rom[0], &bank_0);
        assert_eq!(&rom.rom[1], &bank_1);
    }
}
