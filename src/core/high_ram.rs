use crate::core::Addressable;

pub struct HighRam {
    ram: [u8; 127],
}

impl Default for HighRam {
    fn default() -> Self {
        Self { ram: [0; 127] }
    }
}

impl Addressable for HighRam {
    fn read(&self, address: u16) -> Option<u8> {
        let a = address as usize;
        match address {
            0xFF80..=0xFFFE => Some(self.ram[a - 0xFF80]),
            // TODO Echo RAM, trace log somehow?
            _ => None,
        }
    }

    fn write(&mut self, address: u16, value: u8) -> Option<()> {
        let a = address as usize;
        match address {
            0xFF80..=0xFFFE => {
                self.ram[a - 0xFF80] = value;
                Some(())
            }
            _ => None,
        }
    }
}
