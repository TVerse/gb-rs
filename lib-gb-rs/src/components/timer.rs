use crate::components::interrupt_controller::Interrupt;
use crate::{Addressable, InterruptContext};

#[derive(Debug, Copy, Clone)]
enum TimerControl {
    Div1024,
    Div16,
    Div64,
    Div256,
}

impl Default for TimerControl {
    fn default() -> Self {
        Self::Div1024
    }
}

impl TimerControl {
    fn into_bits(self) -> u8 {
        match self {
            Self::Div1024 => 0b00,
            Self::Div16 => 0b01,
            Self::Div64 => 0b10,
            Self::Div256 => 0b11,
        }
    }

    fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::Div1024,
            0b01 => Self::Div16,
            0b10 => Self::Div64,
            0b11 => Self::Div256,
            _ => unreachable!(),
        }
    }

    fn mask(&self) -> u16 {
        match self {
            TimerControl::Div1024 => 1 << 9,
            TimerControl::Div16 => 1 << 3,
            TimerControl::Div64 => 1 << 5,
            TimerControl::Div256 => 1 << 7,
        }
    }
}

#[derive(Default, Debug)]
pub struct Timer {
    divider: u16,
    timer_counter: u8,
    timer_modulo: u8,
    timer_enabled: bool,
    timer_control: TimerControl,
    timer_was_high_last_tick: bool,
    timer_overflowed_last_tick: bool,
}

impl Timer {
    const TIMER_ENABLE_BIT: u8 = 0b00000100;

    pub fn tick<I: InterruptContext>(&mut self, context: &mut I) {
        self.divider = self.divider.wrapping_add(1);
        let is_high = self.divider & self.timer_control.mask() > 0;
        let high_and_enabled = is_high && self.timer_enabled;
        if self.timer_overflowed_last_tick {
            context.raise_interrupt(Interrupt::Timer);
            self.timer_counter = self.timer_modulo;
            self.timer_overflowed_last_tick = false;
        }
        if !high_and_enabled && self.timer_was_high_last_tick {
            self.timer_counter = self.timer_counter.wrapping_add(1);
            if self.timer_counter == 0 {
                self.timer_overflowed_last_tick = true;
            }
        }
        self.timer_was_high_last_tick = high_and_enabled;
    }
}

impl Addressable for Timer {
    fn read(&self, address: u16) -> Option<u8> {
        match address {
            0xFF04 => Some((self.divider >> 8) as u8),
            0xFF05 => Some(self.timer_counter),
            0xFF06 => Some(self.timer_modulo),
            0xFF07 => {
                let enabled = if self.timer_enabled {
                    Self::TIMER_ENABLE_BIT
                } else {
                    0
                };
                Some(enabled | self.timer_control.into_bits())
            }
            _ => None,
        }
    }

    fn write(&mut self, address: u16, value: u8) -> Option<()> {
        match address {
            0xFF04 => {
                self.divider = 0;
                Some(())
            }
            0xFF05 => {
                self.timer_counter = value;
                Some(())
            }
            0xFF06 => {
                self.timer_modulo = value;
                Some(())
            }
            0xFF07 => {
                self.timer_enabled = value & Self::TIMER_ENABLE_BIT > 0;
                self.timer_control = TimerControl::from_bits(value);
                Some(())
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "DIV: {:#06X}", self.divider)?;
        writeln!(
            f,
            "timer {}",
            if self.timer_enabled {
                "enabled"
            } else {
                "disabled"
            }
        )?;
        writeln!(f, "TIMA: {:#04X}", self.timer_counter)?;
        writeln!(f, "TMA: {:#04X}", self.timer_modulo)?;
        let divider = match self.timer_control {
            TimerControl::Div1024 => "1024",
            TimerControl::Div16 => "16",
            TimerControl::Div64 => "64",
            TimerControl::Div256 => "256",
        };
        writeln!(f, "clock divider: {}", divider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct TestInterruptController {
        triggered_interrupt: bool,
    }

    impl InterruptContext for TestInterruptController {
        fn raise_interrupt(&mut self, interrupt: Interrupt) {
            if interrupt == Interrupt::Timer {
                self.triggered_interrupt = true;
            } else {
                panic!("Wrong interrupt raised: {:?}", interrupt)
            }
        }
    }

    #[test]
    fn trigger_interrupt() {
        let mut context = TestInterruptController::default();
        let mut timer = Timer {
            timer_enabled: true,
            divider: 0,
            timer_counter: 0xFF,
            timer_control: TimerControl::Div16,
            ..Timer::default()
        };
        for _ in 0..16 {
            dbg!(&mut timer).tick(&mut context);
        }
        assert_eq!(timer.timer_counter, 0);
        dbg!(&mut timer);
        timer.tick(&mut context);
        dbg!(&mut timer);
        assert!(context.triggered_interrupt)
    }

    #[test]
    fn trigger_interrupt_twice() {
        let mut context = TestInterruptController::default();
        let mut timer = Timer {
            timer_enabled: true,
            divider: 0,
            timer_counter: 0x00,
            timer_control: TimerControl::Div16,
            ..Timer::default()
        };
        for _ in 0..(16 * 256) + 1 {
            timer.tick(&mut context);
        }
        assert!(context.triggered_interrupt, "first interrupt");
        context.triggered_interrupt = false;
        for _ in 0..(16 * 256) + 1 {
            timer.tick(&mut context);
        }
        assert!(context.triggered_interrupt, "second interrupt");
    }

    #[test]
    fn trigger_interrupt_1024() {
        let mut context = TestInterruptController::default();
        let mut timer = Timer {
            timer_enabled: true,
            divider: 0,
            timer_counter: 0xFE,
            timer_control: TimerControl::Div1024,
            ..Timer::default()
        };
        for _ in 0..(1024 * 2) {
            dbg!(&timer);
            timer.tick(&mut context);
        }
        dbg!(&timer);
        dbg!(&context);
        assert!(!context.triggered_interrupt, "first interrupt in 1");
        timer.tick(&mut context);
        assert!(context.triggered_interrupt, "first interrupt");
        context.triggered_interrupt = false;
        for _ in 0..(16 * 256) {
            timer.tick(&mut context);
        }
        assert!(!context.triggered_interrupt, "second interrupt");
    }

    #[test]
    fn div_reload() {
        let mut context = TestInterruptController::default();
        let mut timer = Timer {
            timer_enabled: true,
            divider: 0,
            timer_counter: 0xFF,
            timer_control: TimerControl::Div16,
            ..Timer::default()
        };
        for _ in 0..8 {
            timer.tick(&mut context);
        }
        timer.write(0xFF04, 0).unwrap();
        timer.tick(&mut context);
        assert_eq!(timer.timer_counter, 0);
    }

    #[test]
    fn tac_frequency_change_timer_enabled() {
        let mut context = TestInterruptController::default();
        let mut timer = Timer {
            timer_enabled: true,
            divider: 0b001000,
            timer_counter: 0xFF,
            timer_control: TimerControl::Div16,
            ..Timer::default()
        };
        timer.tick(&mut context);
        timer.write(0xFF07, 0b110).unwrap();
        timer.tick(&mut context);
        assert_eq!(timer.timer_counter, 0)
    }

    #[test]
    fn tac_frequency_change_timer_disabling() {
        let mut context = TestInterruptController::default();
        let mut timer = Timer {
            timer_enabled: true,
            divider: 0b001000,
            timer_counter: 0xFF,
            timer_control: TimerControl::Div16,
            ..Timer::default()
        };
        timer.tick(&mut context);
        timer.write(0xFF07, 0b010).unwrap();
        timer.tick(&mut context);
        assert_eq!(timer.timer_counter, 0)
    }

    #[test]
    fn tac_frequency_change_timer_disabled() {
        let mut context = TestInterruptController::default();
        let mut timer = Timer {
            timer_enabled: false,
            divider: 0b001000,
            timer_counter: 0xFF,
            timer_control: TimerControl::Div16,
            ..Timer::default()
        };
        timer.tick(&mut context);
        timer.write(0xFF07, 0b010).unwrap();
        timer.tick(&mut context);
        assert!(!context.triggered_interrupt)
    }
}
