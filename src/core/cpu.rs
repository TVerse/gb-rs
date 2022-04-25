use bitflags::bitflags;
use strum_macros::Display;

bitflags! {
    #[derive(Default)]
    pub struct Flags: u8 {
        const Z = 0x80;
        const N = 0x40;
        const H = 0x20;
        const C = 0x10;
    }
}

impl std::fmt::Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display)]
pub enum Register8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

// TODO can I fit SP and PC in Register8?
#[derive(Debug, Copy, Clone, Eq, PartialEq, Display)]
pub enum Register16 {
    AF,
    BC,
    DE,
    HL,
    SP,
    PC,
}

impl Register16 {
    pub fn from_byte_sp(b: u8) -> Register16 {
        debug_assert!(b <= 3);
        match b {
            0 => Register16::BC,
            1 => Register16::DE,
            2 => Register16::HL,
            3 => Register16::SP,
            _ => unreachable!(),
        }
    }

    pub fn from_byte_af(b: u8) -> Register16 {
        debug_assert!(b <= 3);
        match b {
            0 => Register16::BC,
            1 => Register16::DE,
            2 => Register16::HL,
            3 => Register16::AF,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum State {
    Running,
    Halted,
}

impl Default for State {
    fn default() -> Self {
        Self::Running
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Cpu {
    a: u8,
    f: Flags,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
    state: State,
}

impl Cpu {
    pub fn after_boot_rom() -> Self {
        Self {
            a: 0,
            f: Flags::empty(),
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0xFFFE,
            pc: 0x0100,
            state: State::Running,
        }
    }

    pub fn read_register8(&self, reg: Register8) -> u8 {
        match reg {
            Register8::A => self.a,
            Register8::B => self.b,
            Register8::C => self.c,
            Register8::D => self.d,
            Register8::E => self.e,
            Register8::H => self.h,
            Register8::L => self.l,
        }
    }

    pub fn write_register8(&mut self, reg: Register8, byte: u8) {
        match reg {
            Register8::A => self.a = byte,
            Register8::B => self.b = byte,
            Register8::C => self.c = byte,
            Register8::D => self.d = byte,
            Register8::E => self.e = byte,
            Register8::H => self.h = byte,
            Register8::L => self.l = byte,
        }
    }

    pub fn read_register16(&self, reg: Register16) -> u16 {
        match reg {
            Register16::AF => ((self.a as u16) << 8) | (self.f.bits as u16),
            Register16::BC => ((self.b as u16) << 8) | (self.c as u16),
            Register16::DE => ((self.d as u16) << 8) | (self.e as u16),
            Register16::HL => ((self.h as u16) << 8) | (self.l as u16),
            Register16::SP => self.sp,
            Register16::PC => self.pc,
        }
    }
    pub fn write_register16(&mut self, reg: Register16, word: u16) {
        let high = (word >> 8) as u8;
        let low = word as u8;

        match reg {
            Register16::AF => {
                self.a = high;
                self.f = Flags::from_bits_truncate(low);
            }
            Register16::BC => {
                self.b = high;
                self.c = low;
            }
            Register16::DE => {
                self.d = high;
                self.e = low;
            }
            Register16::HL => {
                self.h = high;
                self.l = low;
            }
            Register16::SP => self.sp = word,
            Register16::PC => self.pc = word,
        }
    }

    pub fn increment_pc(&mut self) {
        self.pc = self.pc.wrapping_add(1)
    }

    pub fn flags(&self) -> Flags {
        self.f
    }

    pub fn modify_flags(&mut self, f: impl FnOnce(&mut Flags)) {
        f(&mut self.f)
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }
}

impl std::fmt::Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "a: {:#04x}\tf: {:#04x}", self.a, self.f.bits)?;
        writeln!(f, "b: {:#04x}\tc: {:#04x}", self.b, self.c)?;
        writeln!(f, "d: {:#04x}\te: {:#04x}", self.d, self.e)?;
        writeln!(f, "h: {:#04x}\tl: {:#04x}", self.h, self.l)?;
        writeln!(f, "sp: {:#06x}", self.sp)?;
        writeln!(f, "pc: {:#06x}", self.pc)?;

        writeln!(f, "flags: {}", self.f)?;

        Ok(())
    }
}
