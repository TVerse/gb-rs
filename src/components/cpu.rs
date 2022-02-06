use thiserror::Error;

#[derive(Error, Debug)]
pub enum CpuError {
    #[error("Undefined opcode at pc {pc}: {opcode}")]
    UndefinedOpcode { pc: u16, opcode: u8 },
}

#[derive(Debug, Clone)]
pub struct Cpu {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub flags: Flags,
    pub state: State,
    pub interrupt_master_enable: bool,
    pub in_enable_interrupt_delay: bool,
    pub instruction_counter: u64,
}

impl std::fmt::Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cpu state:")?;
        writeln!(f, "a: {:#04x}", self.a)?;
        writeln!(f, "b: {:#04x}", self.b)?;
        writeln!(f, "c: {:#04x}", self.c)?;
        writeln!(f, "d: {:#04x}", self.d)?;
        writeln!(f, "e: {:#04x}", self.e)?;
        writeln!(f, "h: {:#04x}", self.h)?;
        writeln!(f, "l: {:#04x}", self.l)?;
        writeln!(f, "sp: {:#06x}", self.sp)?;
        writeln!(f, "pc: {:#06x}", self.pc)?;

        writeln!(
            f,
            "flags: z {} n {} h {} c {}",
            self.flags.z, self.flags.n, self.flags.h, self.flags.c
        )?;

        writeln!(f, "state: {}", self.state)?;

        writeln!(f, "instruction counter: {}", self.instruction_counter)?;

        if self.interrupt_master_enable {
            writeln!(f, "interrupts enabled")?;
        } else {
            writeln!(f, "interrupts disabled")?;
        }

        Ok(())
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
            instruction_counter: 0,
        }
    }

    pub fn post_boot_rom() -> Self {
        let mut cpu = Self::zeroed();
        cpu.pc = 0x0100;
        cpu.sp = 0xFFFE;
        log::trace!("Initial CPU: {}", cpu);
        cpu
    }

    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.flags.as_byte() as u16)
    }

    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    pub fn set_af(&mut self, af: u16) {
        self.a = (af >> 8) as u8;
        self.flags.copy_from_byte(af as u8);
    }

    pub fn set_bc(&mut self, bc: u16) {
        self.b = (bc >> 8) as u8;
        self.c = bc as u8;
    }

    pub fn set_de(&mut self, de: u16) {
        self.d = (de >> 8) as u8;
        self.e = de as u8;
    }

    pub fn set_hl(&mut self, hl: u16) {
        self.h = (hl >> 8) as u8;
        self.l = hl as u8;
    }

    pub fn inc_pc(&mut self, by: u16) {
        self.pc = self.pc.wrapping_add(by)
    }
}

#[derive(Debug, Clone)]
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

    pub fn c(&self) -> u8 {
        if self.c {
            1
        } else {
            0
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

    pub fn copy_from_byte(&mut self, byte: u8) {
        if byte & 0x80 != 0 {
            self.z = true
        } else {
            self.z = false
        }
        if byte & 0x40 != 0 {
            self.n = true
        } else {
            self.n = false
        }
        if byte & 0x20 != 0 {
            self.h = true
        } else {
            self.h = false
        }
        if byte & 0x10 != 0 {
            self.c = true
        } else {
            self.c = false
        }
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
