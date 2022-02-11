use strum_macros::Display;

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display)]
pub enum Register16 {
    AF,
    BC,
    DE,
    HL,
    SP,
    PC,
}

#[derive(Debug, Clone)]
pub struct Cpu {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
    flags: Flags,
    state: State,
    interrupt_master_enable: bool,
    in_enable_interrupt_delay: bool,
}

impl std::fmt::Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "a: {:#04x}\tf: {:#04x}", self.a, self.flags.as_byte())?;
        writeln!(f, "b: {:#04x}\tc: {:#04x}", self.b, self.c)?;
        writeln!(f, "d: {:#04x}\te: {:#04x}", self.d, self.e)?;
        writeln!(f, "h: {:#04x}\tl: {:#04x}", self.h, self.l)?;
        writeln!(f, "sp: {:#06x}", self.sp)?;
        writeln!(f, "pc: {:#06x}", self.pc)?;

        writeln!(
            f,
            "flags: z {} n {} h {} c {}",
            self.flags.z, self.flags.n, self.flags.h, self.flags.c
        )?;

        writeln!(f, "state: {}", self.state)?;

        if self.interrupt_master_enable {
            write!(f, "interrupts enabled")?;
        } else {
            write!(f, "interrupts disabled")?;
        }

        Ok(())
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::post_boot_rom()
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self::post_boot_rom()
    }

    pub fn zeroed() -> Self {
        Self {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
            flags: Flags::new(),
            state: State::Running,
            interrupt_master_enable: false,
            in_enable_interrupt_delay: false,
        }
    }

    pub fn post_boot_rom() -> Self {
        let mut cpu = Self::zeroed();
        cpu.pc = 0x0100;
        cpu.sp = 0xFFFE;
        cpu
    }

    pub fn get_register8(&self, reg: Register8) -> u8 {
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

    pub fn set_register8(&mut self, reg: Register8, byte: u8) {
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

    pub fn get_register16(&self, reg: Register16) -> u16 {
        match reg {
            Register16::AF => ((self.a as u16) << 8) | (self.flags.as_byte() as u16),
            Register16::BC => ((self.b as u16) << 8) | (self.c as u16),
            Register16::DE => ((self.d as u16) << 8) | (self.e as u16),
            Register16::HL => ((self.h as u16) << 8) | (self.l as u16),
            Register16::SP => self.sp,
            Register16::PC => self.pc,
        }
    }

    pub fn set_register16(&mut self, reg: Register16, word: u16) {
        let high = (word >> 8) as u8;
        let low = word as u8;

        match reg {
            Register16::AF => {
                self.a = high;
                self.flags = Flags::from_byte(low);
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

    pub fn increment_pc(&mut self, by: u16) {
        self.set_register16(
            Register16::PC,
            self.get_register16(Register16::PC).wrapping_add(by),
        )
    }

    pub fn get_state(&self) -> State {
        self.state
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state
    }

    pub fn in_interrupt_delay(&self) -> bool {
        self.in_enable_interrupt_delay
    }

    pub fn interrupts_enabled(&self) -> bool {
        self.interrupt_master_enable
    }

    pub fn start_enable_interrupts(&mut self) {
        self.in_enable_interrupt_delay = true
    }

    pub fn enable_interrupts(&mut self) {
        self.in_enable_interrupt_delay = false;
        self.interrupt_master_enable = true
    }

    pub fn disable_interrupts(&mut self) {
        self.interrupt_master_enable = false
    }

    pub fn edit_flags(
        &mut self,
        z: Option<bool>,
        n: Option<bool>,
        h: Option<bool>,
        c: Option<bool>,
    ) {
        z.into_iter().for_each(|z| self.flags.z = z);
        n.into_iter().for_each(|n| self.flags.n = n);
        h.into_iter().for_each(|h| self.flags.h = h);
        c.into_iter().for_each(|c| self.flags.c = c);
    }

    pub fn get_flags(&self) -> &Flags {
        &self.flags
    }

    pub fn set_flags(&mut self, f: Flags) {
        self.flags = f;
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Flags {
    pub z: bool,
    pub n: bool,
    pub h: bool,
    pub c: bool,
}

impl Flags {
    fn new() -> Self {
        Self {
            z: false,
            n: false,
            h: false,
            c: false,
        }
    }

    pub fn as_byte(&self) -> u8 {
        let mut res = 0;
        if self.z {
            res |= 0x80
        }
        if self.n {
            res |= 0x40
        }
        if self.h {
            res |= 0x20
        }
        if self.c {
            res |= 0x10
        }

        res
    }

    pub fn from_byte(byte: u8) -> Self {
        let z = byte & 0x80 != 0;
        let n = byte & 0x40 != 0;
        let h = byte & 0x20 != 0;
        let c = byte & 0x10 != 0;
        Flags { z, n, h, c }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum State {
    Running,
    Halted,
    Stopped,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Running => write!(f, "Running"),
            State::Halted => write!(f, "Halted"),
            State::Stopped => write!(f, "Stopped"),
        }
    }
}
