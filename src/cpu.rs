use crate::components::Memory;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CpuError {
    #[error("Undefined opcode at pc {pc}: {opcode}")]
    UndefinedOpcode { pc: u16, opcode: u8 },
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

    fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    fn set_bc(&mut self, bc: u16) {
        self.b = (bc >> 8) as u8;
        self.c = bc as u8;
    }

    fn set_de(&mut self, bc: u16) {
        self.d = (bc >> 8) as u8;
        self.e = bc as u8;
    }

    fn set_hl(&mut self, bc: u16) {
        self.h = (bc >> 8) as u8;
        self.l = bc as u8;
    }

    fn inc_pc(&mut self, by: u16) {
        self.pc = self.pc.wrapping_add(by)
    }
}

pub struct ExecutingCpu<'a> {
    cpu: &'a mut Cpu,
    memory: &'a mut dyn Memory,
}

impl<'a> ExecutingCpu<'a> {
    pub fn new(cpu: &'a mut Cpu, memory_map: &'a mut dyn Memory) -> Self {
        Self {
            cpu,
            memory: memory_map,
        }
    }

    pub fn step(&mut self) -> Result<usize, CpuError> {
        // Handle interrupt
        let cycles = self.handle_interrupt();
        if cycles > 0 {
            return Ok(cycles);
        }

        //  EI is delayed by 1 cycle
        if self.cpu.in_enable_interrupt_delay {
            self.cpu.interrupt_master_enable = true
        }

        let opcode = self.memory.read_byte(self.cpu.pc);
        self.cpu.inc_pc(1);
        let cycles = match opcode {
            0x00 => 1,
            0x01 => self.load_immediate_16(Register16::BC),
            0x11 => self.load_immediate_16(Register16::DE),
            0x21 => self.load_immediate_16(Register16::HL),
            0x31 => self.load_immediate_16(Register16::SP),
            0x40..=0x7F => {
                if opcode == 0x76 {
                    self.cpu.state = State::Halted;
                    1
                } else {
                    let source = Self::decode_register(opcode);
                    let target = Self::decode_ld_target_register(opcode);
                    self.load(source, target)
                }
            }
            0x80..=0x87 => {
                let reg = Self::decode_register(opcode);
                self.add_reg(reg, false)
            }
            0x88..=0x8F => {
                let reg = Self::decode_register(opcode);
                self.add_reg(reg, true)
            }
            0x90..=0x97 => {
                let reg = Self::decode_register(opcode);
                self.sub_reg(reg, false)
            }
            0x98..=0x9F => {
                let reg = Self::decode_register(opcode);
                self.sub_reg(reg, true)
            }
            0xC9 => self.ret(),
            0xCB => self.cb_prefix()?,
            0xCD => {
                let target = self.get_from_address_16(self.cpu.pc);
                self.cpu.inc_pc(2);
                self.call(target)
            }
            0xF3 => {
                self.cpu.interrupt_master_enable = false;
                1
            }
            0xFB => {
                self.cpu.in_enable_interrupt_delay = true;
                1
            }
            0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB | 0xEC | 0xED | 0xF4 | 0xFC | 0xFD => {
                Err(CpuError::UndefinedOpcode {
                    pc: self.cpu.pc,
                    opcode,
                })?
            }
            _ => panic!("Unimplemented opcode {:#x}", opcode), // TODO comment this out once everything's implemented
        };
        Ok(cycles)
    }

    fn cb_prefix(&mut self) -> Result<usize, CpuError> {
        match self.memory.read_byte(self.cpu.pc) {
            _ => todo!(),
        }
        self.cpu.inc_pc(1);
        Ok(todo!())
    }

    fn load_immediate_16(&mut self, reg: Register16) -> usize {
        let result = self.memory.read_word(self.cpu.pc);
        match reg {
            Register16::BC => self.cpu.set_bc(result),
            Register16::DE => self.cpu.set_de(result),
            Register16::HL => self.cpu.set_hl(result),
            Register16::SP => self.cpu.sp = result,
        };
        self.cpu.inc_pc(2);
        3
    }

    fn load(&mut self, source: DecodedRegister, target: DecodedRegister) -> usize {
        let byte = self.read_byte_register(source);
        self.write_byte_register(target, byte);

        let mut cycles = 1;
        if source == DecodedRegister::IndirectHL {
            cycles += 1;
        }
        if target == DecodedRegister::IndirectHL {
            cycles += 1;
        }
        cycles
    }

    fn add_reg(&mut self, source: DecodedRegister, including_carry: bool) -> usize {
        let to_add = self.read_byte_register(source) as u16;
        let cur_a = self.cpu.a as u16;
        let carry = if including_carry {
            self.cpu.flags.c() as u16
        } else {
            0
        };
        let res = cur_a + to_add + carry;
        if res > 0xFF {
            self.cpu.flags.c = true;
        }
        let res = res as u8;
        if res == 0 {
            self.cpu.flags.z = true;
        }

        self.cpu.flags.n = false;
        // TODO H
        self.cpu.a = res;

        if source == DecodedRegister::IndirectHL {
            2
        } else {
            1
        }
    }

    fn sub_reg(&mut self, source: DecodedRegister, including_carry: bool) -> usize {
        let to_sub = self.read_byte_register(source) as i16;
        let cur_a = self.cpu.a as i16;
        let carry = if including_carry {
            self.cpu.flags.c() as i16
        } else {
            0
        };
        let res = cur_a.wrapping_sub(to_sub).wrapping_sub(carry);
        if res < 0 {
            self.cpu.flags.c = true;
        }
        let res = res as u8;
        if res == 0 {
            self.cpu.flags.z = true;
        }
        self.cpu.flags.n = true;
        // TODO H
        self.cpu.a = res as u8;

        if source == DecodedRegister::IndirectHL {
            2
        } else {
            1
        }
    }

    fn decode_register(byte: u8) -> DecodedRegister {
        DecodedRegister::from_triple(byte & 0x7)
    }

    fn decode_ld_target_register(byte: u8) -> DecodedRegister {
        DecodedRegister::from_triple((byte >> 3) & 0x7)
    }

    fn read_byte_register(&self, register: DecodedRegister) -> u8 {
        match register {
            DecodedRegister::Register8(r) => match r {
                Register8::A => self.cpu.a,
                Register8::B => self.cpu.b,
                Register8::C => self.cpu.c,
                Register8::D => self.cpu.d,
                Register8::E => self.cpu.e,
                Register8::H => self.cpu.h,
                Register8::L => self.cpu.l,
            },
            DecodedRegister::IndirectHL => self.get_from_address_8(self.cpu.get_hl()),
        }
    }

    fn write_byte_register(&mut self, register: DecodedRegister, byte: u8) {
        match register {
            DecodedRegister::Register8(r) => match r {
                Register8::A => self.cpu.a = byte,
                Register8::B => self.cpu.b = byte,
                Register8::C => self.cpu.c = byte,
                Register8::D => self.cpu.d = byte,
                Register8::E => self.cpu.e = byte,
                Register8::H => self.cpu.h = byte,
                Register8::L => self.cpu.l = byte,
            },
            DecodedRegister::IndirectHL => self.set_address_to(self.cpu.get_hl(), byte),
        }
    }

    fn get_from_address_8(&self, addr: u16) -> u8 {
        self.memory.read_byte(addr)
    }

    fn get_from_address_16(&self, addr: u16) -> u16 {
        self.memory.read_word(addr)
    }

    fn set_address_to(&mut self, addr: u16, byte: u8) {
        self.memory.write_byte(addr, byte)
    }

    fn do_push(&mut self, value: u16) {
        self.set_address_to(self.cpu.sp.wrapping_sub(1), (value >> 8) as u8);
        self.set_address_to(self.cpu.sp.wrapping_sub(2), (value & 0xFF) as u8);
        self.cpu.sp = self.cpu.sp.wrapping_sub(2);
    }

    fn do_pop(&mut self) -> u16 {
        let lower = self.get_from_address_8(self.cpu.sp) as u16;
        let higher = self.get_from_address_8(self.cpu.sp.wrapping_add(1)) as u16;
        self.cpu.sp = self.cpu.sp.wrapping_add(2);

        (higher << 8) | lower
    }

    fn call(&mut self, addr: u16) -> usize {
        self.do_push(self.cpu.pc);
        self.cpu.pc = addr;
        6
    }

    fn ret(&mut self) -> usize {
        self.cpu.pc = self.do_pop();

        4
    }

    fn handle_interrupt(&mut self) -> usize {
        if self.cpu.interrupt_master_enable {
            let interrupt_enable_flags = self.memory.read_byte(0xFFFF);
            let interrupt_request_flags = self.memory.read_byte(0xFF0F);
            let pending = interrupt_enable_flags | interrupt_request_flags;
            if pending & 0x01 != 0 {
                // VBlank
                self.memory
                    .write_byte(0xFF0F, interrupt_request_flags & !0x01);
                self.cpu.interrupt_master_enable = false;
                self.call(0x40);
                5
            } else if pending & 0x02 != 0 {
                // LCD STAT
                self.memory
                    .write_byte(0xFF0F, interrupt_request_flags & !0x02);
                self.cpu.interrupt_master_enable = false;
                self.call(0x48);
                5
            } else if pending & 0x04 != 0 {
                // Timer
                self.memory
                    .write_byte(0xFF0F, interrupt_request_flags & !0x04);
                self.cpu.interrupt_master_enable = false;
                self.call(0x50);
                5
            } else if pending & 0x08 != 0 {
                // Serial
                self.memory
                    .write_byte(0xFF0F, interrupt_request_flags & !0x08);
                self.cpu.interrupt_master_enable = false;
                self.call(0x58);
                5
            } else if pending & 0x10 != 0 {
                // Joypad
                self.memory
                    .write_byte(0xFF0F, interrupt_request_flags & !0x10);
                self.cpu.interrupt_master_enable = false;
                self.call(0x60);
                5
            } else {
                0
            }
        } else {
            0
        }
    }
}

#[derive(Debug, Clone)]
struct Flags {
    z: bool,
    n: bool,
    h: bool,
    c: bool,
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

    fn c(&self) -> u8 {
        if self.c {
            1
        } else {
            0
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum State {
    Running,
    Halted,
    Stopped,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum DecodedRegister {
    Register8(Register8),
    IndirectHL,
}

impl DecodedRegister {
    fn from_triple(triple: u8) -> Self {
        match triple {
            0 => DecodedRegister::Register8(Register8::B),
            1 => DecodedRegister::Register8(Register8::C),
            2 => DecodedRegister::Register8(Register8::D),
            3 => DecodedRegister::Register8(Register8::E),
            4 => DecodedRegister::Register8(Register8::H),
            5 => DecodedRegister::Register8(Register8::L),
            6 => DecodedRegister::IndirectHL,
            7 => DecodedRegister::Register8(Register8::A),
            _ => unreachable!("Invalid register triple: {}", triple),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Register8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Register16 {
    BC,
    DE,
    HL,
    SP,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::FlatMemory;

    #[test]
    fn ld_b_c() {
        let mut cpu = Cpu::zeroed();
        cpu.c = 3;

        let mut memory = FlatMemory { mem: vec![0x41] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.b, cpu.c, "value");
    }

    #[test]
    fn ld_h_hl() {
        let mut cpu = Cpu::zeroed();
        cpu.set_hl(1);

        let mut memory = FlatMemory {
            mem: vec![0x66, 0x4],
        };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 2, "cycles");
        assert_eq!(cpu.h, memory.mem[1], "value");
    }

    #[test]
    fn add_a_b() {
        let mut cpu = Cpu::zeroed();
        cpu.a = 1;
        cpu.b = 3;

        let mut memory = FlatMemory { mem: vec![0x80] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.a, 4, "value");
        assert!(!cpu.flags.z, "zero");
        assert!(!cpu.flags.n, "sub");
        assert!(!cpu.flags.c, "carry");
    }

    #[test]
    fn adc_a_b_carry() {
        let mut cpu = Cpu::zeroed();
        cpu.a = 1;
        cpu.b = 3;
        cpu.flags.c = true;

        let mut memory = FlatMemory { mem: vec![0x88] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.a, 5, "value");
        assert!(!cpu.flags.z, "zero");
        assert!(!cpu.flags.n, "sub");
        assert!(cpu.flags.c, "carry");
    }

    #[test]
    fn add_a_b_overflow_zero() {
        let mut cpu = Cpu::zeroed();
        cpu.a = 1;
        cpu.b = 255;

        let mut memory = FlatMemory { mem: vec![0x80] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.a, 0, "value");
        assert!(cpu.flags.z, "zero");
        assert!(!cpu.flags.n, "sub");
        assert!(cpu.flags.c, "carry");
    }

    #[test]
    fn adc_a_b_overflow() {
        let mut cpu = Cpu::zeroed();
        cpu.a = 1;
        cpu.b = 255;
        cpu.flags.c = true;

        let mut memory = FlatMemory { mem: vec![0x88] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.a, 1, "value");
        assert!(!cpu.flags.z, "zero");
        assert!(!cpu.flags.n, "sub");
        assert!(cpu.flags.c, "carry");
    }

    #[test]
    fn sub_d() {
        let mut cpu = Cpu::zeroed();
        cpu.a = 50;
        cpu.d = 25;

        let mut memory = FlatMemory { mem: vec![0x92] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.a, 25, "value");
        assert!(!cpu.flags.z, "zero");
        assert!(cpu.flags.n, "sub");
        assert!(!cpu.flags.c, "carry");
    }

    #[test]
    fn sub_d_carry() {
        let mut cpu = Cpu::zeroed();
        cpu.a = 1;
        cpu.d = 2;

        let mut memory = FlatMemory { mem: vec![0x92] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.a, 255, "value");
        assert!(!cpu.flags.z, "zero");
        assert!(cpu.flags.n, "sub");
        assert!(cpu.flags.c, "carry");
    }

    #[test]
    fn sub_d_zero() {
        let mut cpu = Cpu::zeroed();
        cpu.a = 1;
        cpu.d = 1;

        let mut memory = FlatMemory { mem: vec![0x92] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.a, 0, "value");
        assert!(cpu.flags.z, "zero");
        assert!(cpu.flags.n, "sub");
        assert!(!cpu.flags.c, "carry");
    }

    #[test]
    fn sbc_d() {
        let mut cpu = Cpu::zeroed();
        cpu.a = 1;
        cpu.d = 1;

        let mut memory = FlatMemory { mem: vec![0x9A] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.a, 0, "value");
        assert!(cpu.flags.z, "zero");
        assert!(cpu.flags.n, "sub");
        assert!(!cpu.flags.c, "carry");
    }

    #[test]
    fn sbc_d_carry() {
        let mut cpu = Cpu::zeroed();
        cpu.a = 1;
        cpu.d = 1;
        cpu.flags.c = true;

        let mut memory = FlatMemory { mem: vec![0x9A] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.a, 255, "value");
        assert!(!cpu.flags.z, "zero");
        assert!(cpu.flags.n, "sub");
        assert!(cpu.flags.c, "carry");
    }

    #[test]
    fn call() {
        let mut cpu = Cpu::zeroed();
        cpu.sp = 0x0005;

        let mut memory = FlatMemory { mem: vec![0xCD, 0x34, 0x12, 0x00, 0x00] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 6, "cycles");
        assert_eq!(cpu.pc, 0x1234, "pc");
        assert_eq!(cpu.sp, 0x0003, "sp");
        assert_eq!(memory.mem[3..=4], [0x03, 0x00], "stack");
    }

    #[test]
    fn ret() {
        let mut cpu = Cpu::zeroed();
        cpu.sp = 0x0003;

        let mut memory = FlatMemory { mem: vec![0xC9, 0x00, 0x00, 0x34, 0x12] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 4, "cycles");
        assert_eq!(cpu.pc, 0x1234, "pc");
        assert_eq!(cpu.sp, 0x0005, "sp")
    }


    #[test]
    fn call_ret() {
        let mut cpu = Cpu::zeroed();
        cpu.sp = 0x0005;

        let mut memory = FlatMemory { mem: vec![0xCD, 0x05, 0x00, 0x00, 0x00, 0xC9] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        ex.step().unwrap();
        ex.step().unwrap();
        assert_eq!(cpu.pc, 0x0003, "pc");
        assert_eq!(cpu.sp, 0x0005, "sp");
        assert_eq!(memory.mem, [0xCD,0x05,0x00, 0x03, 0x00, 0xC9]);
    }
}
