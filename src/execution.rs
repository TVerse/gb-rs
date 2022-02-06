use crate::components::cpu::{CpuError, State};
use crate::components::AddressError;
use crate::{Bus, Cpu};
use thiserror::Error;

type Result<T> = std::result::Result<T, ExecutionError>;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error(transparent)]
    Cpu(#[from] CpuError),
    #[error(transparent)]
    Address(#[from] AddressError),
}

pub struct ExecutingCpu<'a> {
    cpu: &'a mut Cpu,
    bus: &'a mut dyn Bus,
}

impl<'a> ExecutingCpu<'a> {
    pub fn new(cpu: &'a mut Cpu, bus: &'a mut dyn Bus) -> Self {
        Self { cpu, bus }
    }

    pub fn step(&mut self) -> Result<usize> {
        // Handle interrupt
        let cycles = self.handle_interrupt()?;
        if cycles > 0 {
            return Ok(cycles);
        }

        //  EI is delayed by 1 cycle
        if self.cpu.in_enable_interrupt_delay {
            self.cpu.interrupt_master_enable = true
        }

        let starting_pc = self.cpu.pc;

        let opcode = self.bus.read_byte(self.cpu.pc)?;
        self.cpu.inc_pc(1);
        let cycles = match opcode {
            0x00 => 1,
            0x01 => self.load_immediate_16(Register16::BC)?,
            0x02 => self.load_to_indirect_16(Register16::BC)?,
            0x03 => self.inc_reg16(Register16::BC),
            0x04 => self.inc_reg8(Register8::B),
            0x05 => self.dec_reg8(Register8::B),
            0x06 => self.load_immediate_8(DecodedRegister::Register8(Register8::B))?,
            0x0A => self.load_from_indirect_16(Register16::BC)?,
            0x0B => self.dec_reg16(Register16::BC),
            0x0C => self.inc_reg8(Register8::C),
            0x0D => self.dec_reg8(Register8::C),
            0x0E => self.load_immediate_8(DecodedRegister::Register8(Register8::C))?,
            0x11 => self.load_immediate_16(Register16::DE)?,
            0x12 => self.load_to_indirect_16(Register16::DE)?,
            0x13 => self.inc_reg16(Register16::DE),
            0x14 => self.inc_reg8(Register8::D),
            0x15 => self.dec_reg8(Register8::D),
            0x16 => self.load_immediate_8(DecodedRegister::Register8(Register8::D))?,
            0x18 => {
                let target = self.get_immediate_8()? as i8;
                self.jump_relative(target);
                3
            }
            0x1A => self.load_from_indirect_16(Register16::DE)?,
            0x1B => self.dec_reg16(Register16::DE),
            0x1C => self.inc_reg8(Register8::E),
            0x1D => self.dec_reg8(Register8::E),
            0x1E => self.load_immediate_8(DecodedRegister::Register8(Register8::E))?,
            0x20 => {
                if !self.cpu.flags.z {
                    let target = self.get_immediate_8()? as i8;
                    self.jump_relative(target);
                    3
                } else {
                    2
                }
            }
            0x21 => self.load_immediate_16(Register16::HL)?,
            0x22 => {
                let c = self.load_to_indirect_16(Register16::HL)?;
                self.cpu.set_hl(self.cpu.get_hl().wrapping_add(1));
                c
            }
            0x23 => self.inc_reg16(Register16::HL),
            0x24 => self.inc_reg8(Register8::H),
            0x25 => self.dec_reg8(Register8::H),
            0x26 => self.load_immediate_8(DecodedRegister::Register8(Register8::H))?,
            0x28 => {
                if self.cpu.flags.z {
                    let target = self.get_immediate_8()? as i8;
                    self.jump_relative(target);
                    3
                } else {
                    2
                }
            }
            0x2A => {
                let c = self.load_from_indirect_16(Register16::HL)?;
                self.cpu.set_hl(self.cpu.get_hl().wrapping_add(1));
                c
            }
            0x2B => self.dec_reg16(Register16::HL),
            0x2C => self.inc_reg8(Register8::L),
            0x2D => self.dec_reg8(Register8::L),
            0x2E => self.load_immediate_8(DecodedRegister::Register8(Register8::L))?,
            0x30 => {
                if !self.cpu.flags.c {
                    let target = self.get_immediate_8()? as i8;
                    self.jump_relative(target);
                    3
                } else {
                    2
                }
            }
            0x31 => self.load_immediate_16(Register16::SP)?,
            0x32 => {
                let c = self.load_to_indirect_16(Register16::HL)?;
                self.cpu.set_hl(self.cpu.get_hl().wrapping_sub(1));
                c
            }
            0x33 => self.inc_reg16(Register16::SP),
            0x34 => self.inc_indirect()?,
            0x35 => self.dec_indirect()?,
            0x36 => self.load_immediate_8(DecodedRegister::IndirectHL)?,
            0x38 => {
                if self.cpu.flags.c {
                    let target = self.get_immediate_8()? as i8;
                    self.jump_relative(target);
                    3
                } else {
                    2
                }
            }
            0x3A => {
                let c = self.load_from_indirect_16(Register16::HL)?;
                self.cpu.set_hl(self.cpu.get_hl().wrapping_sub(1));
                c
            }
            0x3B => self.dec_reg16(Register16::SP),
            0x3C => self.inc_reg8(Register8::A),
            0x3D => self.dec_reg8(Register8::A),
            0x3E => self.load_immediate_8(DecodedRegister::Register8(Register8::A))?,
            0x40..=0x7F => {
                if opcode == 0x76 {
                    self.cpu.state = State::Halted;
                    1
                } else {
                    let source = Self::decode_register(opcode);
                    let target = Self::decode_ld_target_register(opcode);
                    self.load(source, target)?
                }
            }
            0x80..=0x87 => {
                let reg = Self::decode_register(opcode);
                self.add_reg(reg, false)?
            }
            0x88..=0x8F => {
                let reg = Self::decode_register(opcode);
                self.add_reg(reg, true)?
            }
            0x90..=0x97 => {
                let reg = Self::decode_register(opcode);
                self.sub_reg(reg, false)?
            }
            0x98..=0x9F => {
                let reg = Self::decode_register(opcode);
                self.sub_reg(reg, true)?
            }
            0xC3 => {
                let imm = self.get_immediate_16()?;
                self.cpu.pc = imm;
                4
            }
            0xC7 => self.rst(ResetVector::Zero)?,
            0xC9 => self.ret()?,
            0xCB => self.cb_prefix()?,
            0xCD => {
                let target = self.read_address_16(self.cpu.pc)?;
                self.cpu.inc_pc(2);
                self.call(target)?
            }
            0xCF => self.rst(ResetVector::One)?,
            0xD7 => self.rst(ResetVector::Two)?,
            0xDF => self.rst(ResetVector::Three)?,
            0xE7 => self.rst(ResetVector::Four)?,
            0xEF => self.rst(ResetVector::Five)?,
            0xF3 => {
                self.cpu.interrupt_master_enable = false;
                1
            }
            0xF7 => self.rst(ResetVector::Six)?,
            0xFB => {
                self.cpu.in_enable_interrupt_delay = true;
                1
            }
            0xFF => self.rst(ResetVector::Seven)?,
            0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB | 0xEC | 0xED | 0xF4 | 0xFC | 0xFD => {
                Err(CpuError::UndefinedOpcode {
                    pc: self.cpu.pc,
                    opcode,
                })?
            }
            _ => panic!(
                "Unimplemented opcode {:#04x} at pc {:#06x}",
                opcode, starting_pc
            ), // TODO comment this out once everything's implemented
        };
        Ok(cycles)
    }

    fn cb_prefix(&mut self) -> Result<usize> {
        todo!()
        // match self.bus.read_byte(self.cpu.pc) {
        //     _ => todo!(),
        // }
        // self.cpu.inc_pc(1);
        // Ok(todo!())
    }

    fn rst(&mut self, vector: ResetVector) -> Result<usize> {
        match vector {
            ResetVector::Zero => self.call(0x0000)?,
            ResetVector::One => self.call(0x0008)?,
            ResetVector::Two => self.call(0x0010)?,
            ResetVector::Three => self.call(0x0018)?,
            ResetVector::Four => self.call(0x0020)?,
            ResetVector::Five => self.call(0x0028)?,
            ResetVector::Six => self.call(0x0030)?,
            ResetVector::Seven => self.call(0x0038)?,
        };

        Ok(4)
    }

    fn jump_relative(&mut self, offset: i8) {
        self.cpu.pc = self.cpu.pc.wrapping_add(offset as u16)
    }

    fn load_from_indirect_16(&mut self, reg: Register16) -> Result<usize> {
        debug_assert!(reg == Register16::BC || reg == Register16::DE || reg == Register16::HL);
        let byte = self.read_address_8(self.read_word_from_register16(reg))?;
        self.write_byte_to_register8(Register8::A, byte);
        Ok(2)
    }

    fn load_to_indirect_16(&mut self, reg: Register16) -> Result<usize> {
        debug_assert!(reg == Register16::BC || reg == Register16::DE || reg == Register16::HL);
        let byte = self.read_byte_from_register8(Register8::A);
        self.write_address(self.read_word_from_register16(reg), byte)?;
        Ok(2)
    }

    fn inc_reg8(&mut self, reg: Register8) -> usize {
        let result = self.read_byte_from_register8(reg).wrapping_add(1);
        self.write_byte_to_register8(reg, result);
        if result == 0 {
            self.cpu.flags.z = true;
        }
        self.cpu.flags.n = false;
        1
    }

    fn dec_reg8(&mut self, reg: Register8) -> usize {
        let result = self.read_byte_from_register8(reg).wrapping_sub(1);
        self.write_byte_to_register8(reg, result);
        if result == 0 {
            self.cpu.flags.z = true;
        }
        self.cpu.flags.n = true;
        1
    }

    fn inc_indirect(&mut self) -> Result<usize> {
        let result = self.read_address_8(self.cpu.get_hl())?.wrapping_add(1);
        self.write_address(self.cpu.get_hl(), result)?;
        if result == 0 {
            self.cpu.flags.z = true;
        }
        self.cpu.flags.n = false;
        Ok(3)
    }

    fn dec_indirect(&mut self) -> Result<usize> {
        let result = self.read_address_8(self.cpu.get_hl())?.wrapping_sub(1);
        self.write_address(self.cpu.get_hl(), result)?;
        if result == 0 {
            self.cpu.flags.z = true;
        }
        self.cpu.flags.n = true;
        Ok(3)
    }

    fn inc_reg16(&mut self, reg: Register16) -> usize {
        self.write_word_to_register16(reg, self.read_word_from_register16(reg).wrapping_add(1));
        2
    }

    fn dec_reg16(&mut self, reg: Register16) -> usize {
        self.write_word_to_register16(reg, self.read_word_from_register16(reg).wrapping_sub(1));
        2
    }

    fn get_immediate_8(&mut self) -> Result<u8> {
        let result = self.bus.read_byte(self.cpu.pc)?;
        self.cpu.inc_pc(1);
        Ok(result)
    }

    fn load_immediate_8(&mut self, reg: DecodedRegister) -> Result<usize> {
        let result = self.get_immediate_8()?;

        match reg {
            DecodedRegister::Register8(r) => self.write_byte_to_register8(r, result),
            DecodedRegister::IndirectHL => {
                self.write_address(self.read_address_16(self.cpu.get_hl())?, result)?;
            }
        }

        if reg == DecodedRegister::IndirectHL {
            Ok(3)
        } else {
            Ok(2)
        }
    }

    fn get_immediate_16(&mut self) -> Result<u16> {
        let result = self.bus.read_word(self.cpu.pc)?;
        self.cpu.inc_pc(2);
        Ok(result)
    }

    fn load_immediate_16(&mut self, reg: Register16) -> Result<usize> {
        let result = self.get_immediate_16()?;
        match reg {
            Register16::BC => self.cpu.set_bc(result),
            Register16::DE => self.cpu.set_de(result),
            Register16::HL => self.cpu.set_hl(result),
            Register16::SP => self.cpu.sp = result,
        };
        Ok(3)
    }

    fn load(&mut self, source: DecodedRegister, target: DecodedRegister) -> Result<usize> {
        let byte = match source {
            DecodedRegister::Register8(r) => self.read_byte_from_register8(r),
            DecodedRegister::IndirectHL => self.read_address_8(self.cpu.get_hl())?,
        };

        match target {
            DecodedRegister::Register8(r) => self.write_byte_to_register8(r, byte),
            DecodedRegister::IndirectHL => self.write_address(self.cpu.get_hl(), byte)?,
        }

        let mut cycles = 1;
        if source == DecodedRegister::IndirectHL {
            cycles += 1;
        }
        if target == DecodedRegister::IndirectHL {
            cycles += 1;
        }
        Ok(cycles)
    }

    fn add_reg(&mut self, source: DecodedRegister, including_carry: bool) -> Result<usize> {
        let to_add = match source {
            DecodedRegister::Register8(r) => self.read_byte_from_register8(r),
            DecodedRegister::IndirectHL => self.read_address_8(self.cpu.get_hl())?,
        } as u16;
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
            Ok(2)
        } else {
            Ok(1)
        }
    }

    fn sub_reg(&mut self, source: DecodedRegister, including_carry: bool) -> Result<usize> {
        let to_sub = match source {
            DecodedRegister::Register8(r) => self.read_byte_from_register8(r),
            DecodedRegister::IndirectHL => self.read_address_8(self.cpu.get_hl())?,
        } as i16;
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
            Ok(2)
        } else {
            Ok(1)
        }
    }

    fn decode_register(byte: u8) -> DecodedRegister {
        DecodedRegister::from_triple(byte & 0x7)
    }

    fn decode_ld_target_register(byte: u8) -> DecodedRegister {
        DecodedRegister::from_triple((byte >> 3) & 0x7)
    }

    fn read_byte_from_register8(&self, register: Register8) -> u8 {
        match register {
            Register8::A => self.cpu.a,
            Register8::B => self.cpu.b,
            Register8::C => self.cpu.c,
            Register8::D => self.cpu.d,
            Register8::E => self.cpu.e,
            Register8::H => self.cpu.h,
            Register8::L => self.cpu.l,
        }
    }

    fn read_word_from_register16(&self, register: Register16) -> u16 {
        match register {
            Register16::BC => self.cpu.get_bc(),
            Register16::DE => self.cpu.get_de(),
            Register16::HL => self.cpu.get_hl(),
            Register16::SP => self.cpu.sp,
        }
    }

    fn write_byte_to_register8(&mut self, register: Register8, byte: u8) {
        match register {
            Register8::A => self.cpu.a = byte,
            Register8::B => self.cpu.b = byte,
            Register8::C => self.cpu.c = byte,
            Register8::D => self.cpu.d = byte,
            Register8::E => self.cpu.e = byte,
            Register8::H => self.cpu.h = byte,
            Register8::L => self.cpu.l = byte,
        }
    }

    fn write_word_to_register16(&mut self, register: Register16, word: u16) {
        match register {
            Register16::BC => self.cpu.set_bc(word),
            Register16::DE => self.cpu.set_de(word),
            Register16::HL => self.cpu.set_hl(word),
            Register16::SP => self.cpu.sp = word,
        }
    }

    fn read_address_8(&self, addr: u16) -> Result<u8> {
        Ok(self.bus.read_byte(addr)?)
    }

    fn read_address_16(&self, addr: u16) -> Result<u16> {
        Ok(self.bus.read_word(addr)?)
    }

    fn write_address(&mut self, addr: u16, byte: u8) -> Result<()> {
        Ok(self.bus.write_byte(addr, byte)?)
    }

    fn do_push(&mut self, value: u16) -> Result<()> {
        self.write_address(self.cpu.sp.wrapping_sub(1), (value >> 8) as u8)?;
        self.write_address(self.cpu.sp.wrapping_sub(2), (value & 0xFF) as u8)?;
        self.cpu.sp = self.cpu.sp.wrapping_sub(2);
        Ok(())
    }

    fn do_pop(&mut self) -> Result<u16> {
        let word = self.read_address_16(self.cpu.sp)? as u16;
        self.cpu.sp = self.cpu.sp.wrapping_add(2);
        Ok(word)
    }

    fn call(&mut self, addr: u16) -> Result<usize> {
        self.do_push(self.cpu.pc)?;
        self.cpu.pc = addr;
        Ok(6)
    }

    fn ret(&mut self) -> Result<usize> {
        self.cpu.pc = self.do_pop()?;

        Ok(4)
    }

    fn handle_interrupt(&mut self) -> Result<usize> {
        if self.cpu.interrupt_master_enable {
            let interrupt_enable_flags = self.bus.read_byte(0xFFFF)?;
            let interrupt_request_flags = self.bus.read_byte(0xFF0F)?;
            let pending = interrupt_enable_flags | interrupt_request_flags;
            let cycles = if pending & 0x01 != 0 {
                // VBlank
                self.bus
                    .write_byte(0xFF0F, interrupt_request_flags & !0x01)?;
                self.cpu.interrupt_master_enable = false;
                self.call(0x40)?;
                5
            } else if pending & 0x02 != 0 {
                // LCD STAT
                self.bus
                    .write_byte(0xFF0F, interrupt_request_flags & !0x02)?;
                self.cpu.interrupt_master_enable = false;
                self.call(0x48)?;
                5
            } else if pending & 0x04 != 0 {
                // Timer
                self.bus
                    .write_byte(0xFF0F, interrupt_request_flags & !0x04)?;
                self.cpu.interrupt_master_enable = false;
                self.call(0x50)?;
                5
            } else if pending & 0x08 != 0 {
                // Serial
                self.bus
                    .write_byte(0xFF0F, interrupt_request_flags & !0x08)?;
                self.cpu.interrupt_master_enable = false;
                self.call(0x58)?;
                5
            } else if pending & 0x10 != 0 {
                // Joypad
                self.bus
                    .write_byte(0xFF0F, interrupt_request_flags & !0x10)?;
                self.cpu.interrupt_master_enable = false;
                self.call(0x60)?;
                5
            } else {
                0
            };
            Ok(cycles)
        } else {
            Ok(0)
        }
    }
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ResetVector {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bus::FlatBus;

    #[test]
    fn ld_b_c() {
        let mut cpu = Cpu::zeroed();
        cpu.c = 3;

        let mut memory = FlatBus { mem: vec![0x41] };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        let cycles = ex.step().unwrap();
        assert_eq!(cycles, 1, "cycles");
        assert_eq!(cpu.b, cpu.c, "value");
    }

    #[test]
    fn ld_h_hl() {
        let mut cpu = Cpu::zeroed();
        cpu.set_hl(1);

        let mut memory = FlatBus {
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

        let mut memory = FlatBus { mem: vec![0x80] };

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

        let mut memory = FlatBus { mem: vec![0x88] };

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

        let mut memory = FlatBus { mem: vec![0x80] };

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

        let mut memory = FlatBus { mem: vec![0x88] };

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

        let mut memory = FlatBus { mem: vec![0x92] };

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

        let mut memory = FlatBus { mem: vec![0x92] };

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

        let mut memory = FlatBus { mem: vec![0x92] };

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

        let mut memory = FlatBus { mem: vec![0x9A] };

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

        let mut memory = FlatBus { mem: vec![0x9A] };

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

        let mut memory = FlatBus {
            mem: vec![0xCD, 0x34, 0x12, 0x00, 0x00],
        };

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

        let mut memory = FlatBus {
            mem: vec![0xC9, 0x00, 0x00, 0x34, 0x12],
        };

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

        let mut memory = FlatBus {
            mem: vec![0xCD, 0x05, 0x00, 0x00, 0x00, 0xC9],
        };

        let mut ex = ExecutingCpu::new(&mut cpu, &mut memory);

        ex.step().unwrap();
        ex.step().unwrap();
        assert_eq!(cpu.pc, 0x0003, "pc");
        assert_eq!(cpu.sp, 0x0005, "sp");
        assert_eq!(memory.mem, [0xCD, 0x05, 0x00, 0x03, 0x00, 0xC9]);
    }
}
