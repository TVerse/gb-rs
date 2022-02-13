use crate::{ByteAddressable, GameBoyError};
use crate::{RawResult};

#[derive(Debug, Clone)]
pub struct InterruptController {
    interrupt_flags: u8,
    interrupt_enable: u8,
}

impl InterruptController {
    pub fn new() -> Self {
        Self {
            interrupt_flags: 0,
            interrupt_enable: 0,
        }
    }

    pub fn set_serial_interrupt(&mut self) {
        self.interrupt_flags |= Interrupt::Serial.bit()
    }

    pub fn set_timer_interrupt(&mut self) {
        self.interrupt_flags |= Interrupt::Timer.bit();
    }

}

impl Default for InterruptController {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteAddressable for InterruptController {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        match address {
            0xFF0F => Ok(self.interrupt_flags),
            0xFFFF => Ok(self.interrupt_enable),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "InterruptController read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()> {
        match address {
            0xFF0F => self.interrupt_flags = byte,
            0xFFFF => self.interrupt_enable = byte,
            _ => {
                return Err(GameBoyError::NonMappedAddress {
                    address,
                    description: "InterruptController write",
                })
            }
        };
        Ok(())
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
    pub fn address(&self) -> u16 {
        match self {
            Interrupt::VBlank => 0x40,
            Interrupt::LcdStat => 0x48,
            Interrupt::Timer => 0x50,
            Interrupt::Serial => 0x58,
            Interrupt::Joypad => 0x60,
        }
    }

    pub fn bit(&self) -> u8 {
        match self {
            Interrupt::VBlank => 0b00000001,
            Interrupt::LcdStat => 0b00000010,
            Interrupt::Timer => 0b00000100,
            Interrupt::Serial => 0b00001000,
            Interrupt::Joypad => 0b00010000,
        }
    }
}

impl std::fmt::Display for Interrupt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interrupt::VBlank => write!(f, "VBlank"),
            Interrupt::LcdStat => write!(f, "LCD STAT"),
            Interrupt::Timer => write!(f, "Timer"),
            Interrupt::Serial => write!(f, "Serial"),
            Interrupt::Joypad => write!(f, "Joypad"),
        }
    }
}
