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
        let mut blank = Self::zeroed();
        blank.pc = 0x0100;
        blank.sp = 0xFFFE;
        blank
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

    pub fn set_bc(&mut self, bc: u16) {
        self.b = (bc >> 8) as u8;
        self.c = bc as u8;
    }

    pub fn set_de(&mut self, bc: u16) {
        self.d = (bc >> 8) as u8;
        self.e = bc as u8;
    }

    pub fn set_hl(&mut self, bc: u16) {
        self.h = (bc >> 8) as u8;
        self.l = bc as u8;
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
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum State {
    Running,
    Halted,
    Stopped,
}
