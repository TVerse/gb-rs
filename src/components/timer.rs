use crate::components::interrupt_controller::InterruptController;
use crate::ByteAddressable;
use crate::GameBoyError;
use crate::RawResult;

#[derive(Debug, Clone)]
pub struct Timer {
    div_internal: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    tima_internal: usize,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div_internal: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            tima_internal: 0,
        }
    }

    // TODO breaks if cycles * 4 > u16::MAX
    pub fn step(&mut self, interrupt_controller: &mut InterruptController, cycles: usize) {
        let cycles = cycles * 4; // System clock, not CPU cycles
        self.div_internal = self.div_internal.wrapping_add(cycles as u16);

        if let Some(divider) = self.tac_divider() {
            let tima_internal = self.tima_internal.wrapping_add(cycles);
            self.tima_internal = tima_internal;
            if tima_internal >= divider {
                self.tima_internal = tima_internal % divider;
                let tima = self.tima.wrapping_add(1);
                if tima == 0 {
                    self.tima = self.tma;
                    interrupt_controller.set_timer_interrupt();
                } else {
                    self.tima = tima;
                }
            }
        }
    }

    pub fn div(&self) -> u8 {
        (self.div_internal >> 8) as u8
    }

    fn tac_divider(&self) -> Option<usize> {
        if self.tac & 0b00000100 == 0 {
            None
        } else {
            match self.tac & 0x03 {
                0b00 => Some(1024),
                0b01 => Some(16),
                0b10 => Some(64),
                0b11 => Some(256),
                _ => unreachable!(),
            }
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteAddressable for Timer {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        match address {
            0xFF04 => Ok(self.div()),
            0xFF05 => Ok(self.tima),
            0xFF06 => Ok(self.tma),
            0xFF07 => Ok(self.tac),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "Timer read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()> {
        match address {
            0xFF04 => self.div_internal = 0,
            0xFF05 => self.tima = byte,
            0xFF06 => self.tma = byte,
            0xFF07 => self.tac = byte,
            _ => {
                return Err(GameBoyError::NonMappedAddress {
                    address,
                    description: "Timer write",
                });
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn div() {
        let mut timer = Timer::new();
        let mut ic = InterruptController::new();
        assert_eq!(timer.div(), 0, "div before");
        timer.step(&mut ic, 63);
        assert_eq!(timer.div(), 0, "div middle");
        timer.step(&mut ic, 1);
        assert_eq!(timer.div(), 1, "div after");
    }

    #[test]
    fn timer() {
        let mut timer = Timer::new();
        let mut ic = InterruptController::new();
        timer.step(&mut ic, 256);
        assert_eq!(timer.tima, 0, "tima off");
        timer.tac |= 0x04;
        timer.step(&mut ic, 256);
        assert_eq!(timer.tima, 1, "tima on");
    }

    #[test]
    fn reset() {
        let mut timer = Timer::new();
        let mut ic = InterruptController::new();
        timer.tac |= 0x04;
        timer.tma = 0xFE;
        timer.tima = 0xFF;
        timer.step(&mut ic, 256);
        assert_eq!(timer.tima, timer.tma, "tima");
        assert_eq!(ic.read_byte(0xFF0F).unwrap(), 0x04, "if")
    }

    #[test]
    fn tac_divider() {
        let mut timer = Timer::new();
        let mut ic = InterruptController::new();
        timer.tac = 0x05;
        timer.tma = 0xFE;
        timer.tima = 0xFF;
        timer.step(&mut ic, 4);
        assert_eq!(timer.tima, timer.tma, "tima");
        assert_eq!(ic.read_byte(0xFF0F).unwrap(), 0x04, "if")
    }
}
