use crate::{Addressable, KIB};

pub struct WorkRam {
    ram: [u8; 8 * KIB],
}

impl Default for WorkRam {
    fn default() -> Self {
        Self { ram: [0; 8 * KIB] }
    }
}

impl Addressable for WorkRam {
    fn read(&self, address: u16) -> Option<u8> {
        let a = address as usize;
        match address {
            0xC000..=0xDFFF => Some(self.ram[a - 0xC000]),
            // TODO Echo RAM, trace log somehow?
            _ => None,
        }
    }

    fn write(&mut self, address: u16, value: u8) -> Option<()> {
        let a = address as usize;
        match address {
            0xC000..=0xDFFF => {
                self.ram[a - 0xC000] = value;
                Some(())
            }
            // TODO Echo RAM, trace log somehow?
            _ => None,
        }
    }
}
