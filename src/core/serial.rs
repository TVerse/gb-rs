use crate::core::{Addressable, ExecutionEvent};
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct Control: u8 {
        const TRANSFER_START = 0b10000000;
        // const CLOCK_SPEED = 0b00000010; // CGB ONLY
        const IS_INTERNAL_CLOCK = 0b00000001;
    }
}

#[derive(Default, Debug, Clone)]
pub struct Serial {
    data: u8,
    control: Control,
}

impl Serial {
    pub fn get_data(&mut self) -> Option<u8>{
        if self.control.contains(Control::TRANSFER_START) && !self.control.contains(Control::IS_INTERNAL_CLOCK) {
            unimplemented!("Serial external clock")
        }

        if self.control.contains(Control::TRANSFER_START | Control::IS_INTERNAL_CLOCK) {
            self.control.remove(Control::TRANSFER_START);
            return Some(self.data)
        }

        None
    }
}

impl Addressable for Serial {
    fn read(&self, address: u16) -> Option<u8> {
        match address {
            0xFF01 => Some(self.data),
            0xFF02 => Some(self.control.bits),
            _ => None,
        }
    }

    fn write(&mut self, address: u16, value: u8) -> Option<()> {
        match address {
            0xFF01 => {
                self.data = value;
                Some(())
            }
            0xFF02 => {
                self.control = Control::from_bits_truncate(value);
                Some(())
            }
            _ => None,
        }
    }
}
