use bitflags::bitflags;

use crate::{Addressable, HandleInterruptContext, InterruptContext};

bitflags! {
    #[derive(Default)]
    pub struct InterruptFlag: u8 {
        const VBLANK = 0b00000001;
        const LCD_STAT = 0b00000010;
        const TIMER = 0b00000100;
        const SERIAL = 0b00001000;
        const JOYPAD = 0b00010000;
    }
}

impl From<Interrupt> for InterruptFlag {
    fn from(interrupt: Interrupt) -> Self {
        match interrupt {
            Interrupt::VBlank => InterruptFlag::VBLANK,
            Interrupt::LcdStat => InterruptFlag::LCD_STAT,
            Interrupt::Timer => InterruptFlag::TIMER,
            Interrupt::Serial => InterruptFlag::SERIAL,
            Interrupt::Joypad => InterruptFlag::JOYPAD,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Interrupt {
    VBlank,
    LcdStat,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    pub fn handler_address(&self) -> u16 {
        match self {
            Interrupt::VBlank => 0x40,
            Interrupt::LcdStat => 0x48,
            Interrupt::Timer => 0x50,
            Interrupt::Serial => 0x58,
            Interrupt::Joypad => 0x60,
        }
    }
}

impl std::fmt::Display for Interrupt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interrupt::VBlank => write!(f, "VBLANK"),
            Interrupt::LcdStat => write!(f, "LCD_STAT"),
            Interrupt::Timer => write!(f, "TIMER"),
            Interrupt::Serial => write!(f, "SERIAL"),
            Interrupt::Joypad => write!(f, "JOYPAD"),
        }
    }
}

#[derive(Debug, Default)]
pub struct InterruptController {
    interrupt_master_enable: bool,
    interrupt_flag: InterruptFlag,
    interrupt_enable: InterruptFlag,
    ime_scheduled: bool,
}

impl InterruptController {
    pub fn tick(&mut self) {
        if self.ime_scheduled {
            self.interrupt_master_enable = true;
            self.ime_scheduled = false;
        }
    }
}

impl Addressable for InterruptController {
    fn read(&self, address: u16) -> Option<u8> {
        match address {
            0xFF0F => Some(self.interrupt_flag.bits),
            0xFFFF => Some(self.interrupt_enable.bits),
            _ => None,
        }
    }

    fn write(&mut self, address: u16, value: u8) -> Option<()> {
        match address {
            0xFF0F => {
                self.interrupt_flag = InterruptFlag::from_bits_truncate(value);
                Some(())
            }
            0xFFFF => {
                self.interrupt_enable = InterruptFlag::from_bits_truncate(value);
                Some(())
            }
            _ => None,
        }
    }
}

impl InterruptContext for InterruptController {
    fn raise_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_flag.insert(interrupt.into())
    }
}

impl HandleInterruptContext for InterruptController {
    fn unraise_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_flag.remove(interrupt.into())
    }

    fn should_start_interrupt_routine(&self) -> bool {
        self.interrupt_master_enable && self.interrupt_flag.intersects(self.interrupt_enable)
    }

    fn get_highest_priority_interrupt(&self) -> Option<Interrupt> {
        if !self.interrupt_master_enable {
            None
        } else {
            let candidates = self.interrupt_flag.intersection(self.interrupt_enable);
            if candidates.contains(InterruptFlag::VBLANK) {
                Some(Interrupt::VBlank)
            } else if candidates.contains(InterruptFlag::LCD_STAT) {
                Some(Interrupt::LcdStat)
            } else if candidates.contains(InterruptFlag::TIMER) {
                Some(Interrupt::Timer)
            } else if candidates.contains(InterruptFlag::SERIAL) {
                Some(Interrupt::Serial)
            } else if candidates.contains(InterruptFlag::JOYPAD) {
                Some(Interrupt::Joypad)
            } else {
                None
            }
        }
    }

    fn should_cancel_halt(&self) -> bool {
        self.interrupt_flag.intersects(self.interrupt_enable)
    }

    fn schedule_ime_enable(&mut self) {
        self.ime_scheduled = true; // TODO think I might need a counter from 2
    }

    fn enable_interrupts(&mut self) {
        self.interrupt_master_enable = true;
    }

    fn disable_interrupts(&mut self) {
        self.ime_scheduled = false;
        self.interrupt_master_enable = false;
    }
}

impl std::fmt::Display for InterruptController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "interrupts globally {}",
            if self.interrupt_master_enable {
                "enabled"
            } else {
                "disabled"
            }
        )?;
        writeln!(f, "interrupt flags: {:?}", self.interrupt_flag)?;
        writeln!(f, "interrupts enabled: {:?}", self.interrupt_enable)
    }
}
