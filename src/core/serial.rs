use crate::core::interrupt_controller::Interrupt;
use crate::core::{Addressable, EventContext, HexByte, InterruptContext};
use crate::ExecutionEvent;
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
    clock_counter: u16,
    bit_counter: u8,
    data_copy: u8,
}

impl Serial {
    pub fn tick<C: InterruptContext, E: EventContext>(&mut self, ctx: &mut C, e: &mut E) {
        self.clock_counter += 1;
        if self.clock_counter == 512 {
            self.clock_counter = 0;
        }

        if self.clock_counter == 0 && self.control.contains(Control::TRANSFER_START) {
            if !self.control.contains(Control::IS_INTERNAL_CLOCK) {
                unimplemented!("Serial external clock")
            }
            // No other gameboy, just shift in 1s
            self.data <<= 1;
            self.data |= 1;
            self.bit_counter += 1;
            if self.bit_counter == 8 {
                self.control.remove(Control::TRANSFER_START);
                e.push_event(ExecutionEvent::SerialOut(HexByte(self.data_copy)));
                ctx.raise_interrupt(Interrupt::Serial);
                self.bit_counter = 0;
            }
        }
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
                self.data_copy = value;
                Some(())
            }
            0xFF02 => {
                self.control = Control::from_bits_truncate(value);
                if self.control.contains(Control::TRANSFER_START) {
                    self.bit_counter = 0;
                }
                Some(())
            }
            _ => None,
        }
    }
}
