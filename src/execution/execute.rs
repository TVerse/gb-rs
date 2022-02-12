use crate::components::bus::Bus;
use crate::components::cpu::{Cpu, Flags, Register16, Register8, State};
use crate::execution::instructions::{CommonRegister, Instruction, JumpCondition};
use crate::RawResult;

// 16-bit H/C is weird https://stackoverflow.com/a/57981912

pub fn execute_instruction(
    cpu: &mut Cpu,
    bus: &mut dyn Bus,
    instruction: Instruction,
) -> RawResult<usize> {
    let mut executor = Executor { cpu, bus };
    executor.execute(instruction)
}

struct Executor<'a> {
    cpu: &'a mut Cpu,
    bus: &'a mut dyn Bus,
}

impl<'a> Executor<'a> {
    fn execute(&mut self, instruction: Instruction) -> RawResult<usize> {
        let bytes = instruction.bytes();

        let is_in_interrupt_delay = self.cpu.in_interrupt_delay();

        self.cpu.increment_pc(bytes);

        let mut conditional_branch_not_taken = false;

        match instruction {
            Instruction::LoadRegisterRegister(t, s) => {
                let b = self.get_common_register(s)?;
                self.set_common_register(t, b)?;
            }
            Instruction::LoadRegisterImmediate8(r, b) => self.set_common_register(r, b.0)?,
            Instruction::LoadAIndirectRegister(r) => {
                let addr = self.cpu.get_register16(r);
                let b = self.read_from_memory(addr)?;
                self.cpu.set_register8(Register8::A, b);
            }
            Instruction::LoadAIndirectImmediate16(a) => {
                let b = self.read_from_memory(a.0)?;
                self.cpu.set_register8(Register8::A, b);
            }
            Instruction::LoadIndirectRegisterA(r) => {
                let b = self.cpu.get_register8(Register8::A);
                let addr = self.cpu.get_register16(r);
                self.write_to_memory(addr, b)?;
            }
            Instruction::LoadIndirectImmediate16A(a) => {
                let b = self.cpu.get_register8(Register8::A);
                self.write_to_memory(a.0, b)?;
            }
            Instruction::LoadIOAIndirectImmediate8(a) => {
                let byte = self.read_from_memory(0xFF00 + (a.0 as u16))?;
                self.cpu.set_register8(Register8::A, byte);
            }
            Instruction::LoadIOIndirectImmediate8A(a) => {
                let byte = self.cpu.get_register8(Register8::A);
                self.write_to_memory(0xFF00 + (a.0 as u16), byte)?;
            }
            Instruction::LoadIOIndirectCA => {
                let a = self.cpu.get_register8(Register8::C);
                let byte = self.cpu.get_register8(Register8::A);
                self.write_to_memory(0xFF00 + (a as u16), byte)?;
            }
            Instruction::LoadIOAIndirectC => {
                let a = self.cpu.get_register8(Register8::C);
                let byte = self.read_from_memory(0xFF00 + (a as u16))?;
                self.cpu.set_register8(Register8::A, byte);
            }
            Instruction::LoadAIncrementHLIndirect => {
                let addr = self.cpu.get_register16(Register16::HL);
                let byte = self.read_from_memory(addr)?;
                self.cpu.set_register8(Register8::A, byte);
                self.cpu
                    .set_register16(Register16::HL, addr.wrapping_add(1));
            }
            Instruction::LoadIncrementHLIndirectA => {
                let addr = self.cpu.get_register16(Register16::HL);
                let byte = self.cpu.get_register8(Register8::A);
                self.write_to_memory(addr, byte)?;
                self.cpu
                    .set_register16(Register16::HL, addr.wrapping_add(1));
            }
            Instruction::LoadADecrementHLIndirect => {
                let addr = self.cpu.get_register16(Register16::HL);
                let byte = self.read_from_memory(addr)?;
                self.cpu.set_register8(Register8::A, byte);
                self.cpu
                    .set_register16(Register16::HL, addr.wrapping_sub(1));
            }
            Instruction::LoadDecrementHLAIndirect => {
                let addr = self.cpu.get_register16(Register16::HL);
                let byte = self.cpu.get_register8(Register8::A);
                self.write_to_memory(addr, byte)?;
                self.cpu
                    .set_register16(Register16::HL, addr.wrapping_sub(1));
            }
            Instruction::LoadRegisterImmediate16(r, w) => self.cpu.set_register16(r, w.0),
            Instruction::LoadIndirectImmediate16SP(a) => {
                let w = self.cpu.get_register16(Register16::SP);
                self.write_word_to_memory(a.0, w)?;
            }
            Instruction::LoadSPHL => {
                let w = self.cpu.get_register16(Register16::HL);
                self.cpu.set_register16(Register16::SP, w);
            }
            Instruction::Push(r) => self.push(r)?,
            Instruction::Pop(r) => self.pop(r)?,
            Instruction::AddRegister(r) => {
                let a = self.cpu.get_register8(Register8::A);
                let b = self.get_common_register(r)?;
                let res = self.add_8_set_flags(a, b, false);
                self.cpu.set_register8(Register8::A, res);
            }
            Instruction::AddImmediate8(b) => {
                let a = self.cpu.get_register8(Register8::A);
                let res = self.add_8_set_flags(a, b.0, false);
                self.cpu.set_register8(Register8::A, res)
            }
            Instruction::AddCarryRegister(r) => {
                let a = self.cpu.get_register8(Register8::A);
                let b = self.get_common_register(r)?;
                let res = self.add_8_set_flags(a, b, true);
                self.cpu.set_register8(Register8::A, res);
            }
            Instruction::AddCarryImmediate8(b) => {
                let a = self.cpu.get_register8(Register8::A);
                let res = self.add_8_set_flags(a, b.0, true);
                self.cpu.set_register8(Register8::A, res)
            }
            Instruction::SubRegister(r) => {
                let a = self.cpu.get_register8(Register8::A);
                let b = self.get_common_register(r)?;
                let res = self.sub_8_set_flags(a, b, false);
                self.cpu.edit_flags(None, Some(true), None, None);
                self.cpu.set_register8(Register8::A, res);
            }
            Instruction::SubImmediate8(b) => {
                let a = self.cpu.get_register8(Register8::A);
                let res = self.sub_8_set_flags(a, b.0, false);
                self.cpu.edit_flags(None, Some(true), None, None);
                self.cpu.set_register8(Register8::A, res);
            }
            Instruction::SubCarryRegister(r) => {
                let a = self.cpu.get_register8(Register8::A);
                let b = self.get_common_register(r)?;
                let res = self.sub_8_set_flags(a, b, true);
                self.cpu.edit_flags(None, Some(true), None, None);
                self.cpu.set_register8(Register8::A, res);
            }
            Instruction::SubCarryImmediate8(b) => {
                let a = self.cpu.get_register8(Register8::A);
                let res = self.sub_8_set_flags(a, b.0, true);
                self.cpu.edit_flags(None, Some(true), None, None);
                self.cpu.set_register8(Register8::A, res);
            }
            Instruction::AndRegister8(r) => {
                let b = self.get_common_register(r)?;
                self.and(b);
            }
            Instruction::AndImmediate8(b) => {
                self.and(b.0);
            }
            Instruction::XorRegister(r) => {
                let b = self.get_common_register(r)?;
                self.xor(b);
            }
            Instruction::XorImmediate8(b) => {
                self.xor(b.0);
            }
            Instruction::OrRegister(r) => {
                let b = self.get_common_register(r)?;
                self.or(b);
            }
            Instruction::OrImmediate8(b) => {
                self.or(b.0);
            }
            Instruction::CompareRegister(r) => {
                let a = self.cpu.get_register8(Register8::A);
                let b = self.get_common_register(r)?;
                let _res = self.sub_8_set_flags(a, b, false);
                self.cpu.edit_flags(None, Some(true), None, None);
            }
            Instruction::CompareImmediate8(b) => {
                let a = self.cpu.get_register8(Register8::A);
                let _res = self.sub_8_set_flags(a, b.0, false);
                self.cpu.edit_flags(None, Some(true), None, None);
            }
            Instruction::IncRegister8(r) => {
                let byte = self.get_common_register(r)?;
                let h = byte & 0xF == 0xF;
                let byte = byte.wrapping_add(1);
                let z = byte == 0;
                self.set_common_register(r, byte)?;
                self.cpu.edit_flags(Some(z), Some(false), Some(h), None);
            }
            Instruction::DecRegister8(r) => {
                let byte = self.get_common_register(r)?;
                let h = byte & 0xF == 0x0;
                let byte = byte.wrapping_sub(1);
                let z = byte == 0;
                self.set_common_register(r, byte)?;
                self.cpu.edit_flags(Some(z), Some(true), Some(h), None);
            }
            Instruction::DecimalAdjust => todo!(),
            Instruction::Complement => {
                let a = self.cpu.get_register8(Register8::A);
                self.cpu.set_register8(Register8::A, !a);
                self.cpu.edit_flags(None, Some(true), Some(true), None);
            }
            Instruction::AddHLRegister(r) => self.add_16bit_hl(r),
            Instruction::IncRegister16(r) => {
                let word = self.cpu.get_register16(r);
                let word = word.wrapping_add(1);
                self.cpu.set_register16(r, word);
            }
            Instruction::DecRegister16(r) => {
                let word = self.cpu.get_register16(r);
                let word = word.wrapping_sub(1);
                self.cpu.set_register16(r, word);
            }
            Instruction::AddSPImmediate(i) => {
                let res = self.add_i8_to_sp_and_set_flags(i.0);
                self.cpu.set_register16(Register16::SP, res);
            }
            Instruction::LoadHLSPImmediate(i) => {
                let res = self.add_i8_to_sp_and_set_flags(i.0);
                self.cpu.set_register16(Register16::HL, res);
            }
            Instruction::RotateALeft => {
                self.rlc(CommonRegister::Register8(Register8::A))?;
                self.cpu
                    .edit_flags(Some(false), Some(false), Some(false), None);
            }
            Instruction::RotateALeftThroughCarry => {
                self.rl(CommonRegister::Register8(Register8::A))?;
                self.cpu
                    .edit_flags(Some(false), Some(false), Some(false), None);
            }
            Instruction::RotateARight => {
                self.rrc(CommonRegister::Register8(Register8::A))?;
                self.cpu
                    .edit_flags(Some(false), Some(false), Some(false), None);
            }
            Instruction::RotateARightThroughCarry => {
                self.rr(CommonRegister::Register8(Register8::A))?;
                self.cpu
                    .edit_flags(Some(false), Some(false), Some(false), None);
            }
            Instruction::RotateLeftRegister(r) => self.rlc(r)?,
            Instruction::RotateLeftThroughCarryRegister(r) => self.rl(r)?,
            Instruction::RotateRightRegister(r) => self.rrc(r)?,
            Instruction::RotateRightThroughCarryRegister(r) => self.rr(r)?,
            Instruction::ShiftLeftRegister(r) => self.sla(r)?,
            Instruction::ShiftRightArithmeticRegister(r) => self.sra(r)?,
            Instruction::ShiftRightLogicalRegister(r) => self.srl(r)?,
            Instruction::SwapRegister(r) => {
                let byte = self.get_common_register(r)?;
                let byte = (byte >> 4) | (byte << 4);
                self.set_common_register(r, byte)?;
            }
            Instruction::BitRegister(n, r) => self.bit(r, n)?,
            Instruction::SetRegister(n, r) => self.set(r, n)?,
            Instruction::ResRegister(n, r) => self.res(r, n)?,
            Instruction::Ccf => {
                let flags = *self.cpu.get_flags();
                self.cpu
                    .edit_flags(None, Some(false), Some(false), Some(!flags.c));
            }
            Instruction::Scf => {
                self.cpu
                    .edit_flags(None, Some(false), Some(false), Some(true));
            }
            Instruction::Nop => (),
            Instruction::Halt => self.cpu.set_state(State::Halted),
            Instruction::Stop => self.cpu.set_state(State::Stopped),
            Instruction::DI => self.cpu.disable_interrupts(),
            Instruction::EI => self.cpu.start_enable_interrupts(),
            Instruction::JumpImmediate(a) => {
                self.cpu.set_register16(Register16::PC, a.0);
            }
            Instruction::JumpHL => {
                let a = self.cpu.get_register16(Register16::HL);
                self.cpu.set_register16(Register16::PC, a);
            }
            Instruction::JumpConditionalImmediate(c, a) => {
                let flags = self.cpu.get_flags();
                if Self::should_conditional_jump(flags, c) {
                    self.cpu.set_register16(Register16::PC, a.0)
                } else {
                    conditional_branch_not_taken = true;
                }
            }
            Instruction::JumpRelative(offset) => {
                let pc = self.cpu.get_register16(Register16::PC);
                let a = Self::add_u16_to_i8(pc, offset.0);
                self.cpu.set_register16(Register16::PC, a)
            }
            Instruction::JumpConditionalRelative(c, offset) => {
                let flags = self.cpu.get_flags();
                if Self::should_conditional_jump(flags, c) {
                    let pc = self.cpu.get_register16(Register16::PC);
                    let a = Self::add_u16_to_i8(pc, offset.0);
                    self.cpu.set_register16(Register16::PC, a)
                } else {
                    conditional_branch_not_taken = true;
                }
            }
            Instruction::CallImmediate(a) => self.call(a.0)?,
            Instruction::CallConditionalImmediate(c, a) => {
                let flags = self.cpu.get_flags();
                if Self::should_conditional_jump(flags, c) {
                    self.call(a.0)?;
                } else {
                    conditional_branch_not_taken = true;
                }
            }
            Instruction::Return => self.ret()?,
            Instruction::ReturnConditional(c) => {
                let flags = self.cpu.get_flags();
                if Self::should_conditional_jump(flags, c) {
                    self.ret()?;
                } else {
                    conditional_branch_not_taken = true;
                }
            }
            Instruction::ReturnInterrupt => {
                self.ret()?;
                self.cpu.start_enable_interrupts(); // TODO or enable immediately?
            }
            Instruction::Reset(rv) => self.call(rv.address())?,
        };

        if is_in_interrupt_delay {
            self.cpu.enable_interrupts()
        }

        let cycles = if conditional_branch_not_taken {
            instruction
                .cycles_branch_not_taken()
                .expect("Called cycles_branch_not_taken on an incompatible instruction")
        } else {
            instruction.cycles()
        };

        Ok(cycles)
    }

    fn add_i8_to_sp_and_set_flags(&mut self, i: u8) -> u16 {
        let sp = self.cpu.get_register16(Register16::SP);
        let lower = (sp & 0xFF) as u8;
        let res_h = (lower & 0x0F) + (i & 0x0F);
        let h = res_h > 0x0F;
        let res_lower = (lower as u16) + (i as u16);
        let c = res_lower > 0xFF;

        self.cpu
            .edit_flags(Some(false), Some(false), Some(h), Some(c));
        Self::add_u16_to_i8(sp, i)
    }

    fn add_u16_to_i8(a: u16, b: u8) -> u16 {
        let b = if b & 0x80 == 0 {
            b as u16
        } else {
            (b as u16) | 0xFF00
        };
        a.wrapping_add(b)
    }

    fn add_16bit_hl(&mut self, r: Register16) {
        let hl = self.cpu.get_register16(Register16::HL);
        let word = self.cpu.get_register16(r);
        let h = (hl & 0x0FFF).wrapping_add(word & 0x0FFF) > 0x0FFF;
        let res = (hl as u32).wrapping_add(word as u32);
        let c = res > 0xFFFF;
        self.cpu.edit_flags(None, Some(false), Some(h), Some(c));
        self.cpu.set_register16(Register16::HL, res as u16)
    }

    fn get_common_register(&self, reg: CommonRegister) -> RawResult<u8> {
        match reg {
            CommonRegister::Register8(r) => Ok(self.cpu.get_register8(r)),
            CommonRegister::HLIndirect => {
                self.read_from_memory(self.cpu.get_register16(Register16::HL))
            }
        }
    }

    fn set_common_register(&mut self, reg: CommonRegister, byte: u8) -> RawResult<()> {
        match reg {
            CommonRegister::Register8(r) => {
                self.cpu.set_register8(r, byte);
                Ok(())
            }
            CommonRegister::HLIndirect => {
                self.write_to_memory(self.cpu.get_register16(Register16::HL), byte)
            }
        }
    }

    fn read_from_memory(&self, address: u16) -> RawResult<u8> {
        self.bus.read_byte(address)
    }

    fn read_word_from_memory(&self, address: u16) -> RawResult<u16> {
        self.bus.read_word(address)
    }

    fn write_to_memory(&mut self, address: u16, byte: u8) -> RawResult<()> {
        self.bus.write_byte(address, byte)
    }

    fn write_word_to_memory(&mut self, address: u16, word: u16) -> RawResult<()> {
        self.bus.write_word(address, word)
    }

    fn should_conditional_jump(flags: &Flags, condition: JumpCondition) -> bool {
        match condition {
            JumpCondition::NZ => !flags.z,
            JumpCondition::Z => flags.z,
            JumpCondition::NC => !flags.c,
            JumpCondition::C => flags.c,
        }
    }

    fn push(&mut self, r: Register16) -> RawResult<()> {
        let sp = self.cpu.get_register16(Register16::SP);
        let sp = sp.wrapping_sub(2);
        self.cpu.set_register16(Register16::SP, sp);
        let w = self.cpu.get_register16(r);
        self.write_word_to_memory(sp, w)
    }

    fn pop(&mut self, r: Register16) -> RawResult<()> {
        let sp = self.cpu.get_register16(Register16::SP);
        let w = self.read_word_from_memory(sp)?;
        self.cpu.set_register16(r, w);
        let sp = sp.wrapping_add(2);
        self.cpu.set_register16(Register16::SP, sp);
        Ok(())
    }

    fn call(&mut self, a: u16) -> RawResult<()> {
        self.push(Register16::PC)?;
        self.cpu.set_register16(Register16::PC, a);
        Ok(())
    }

    fn ret(&mut self) -> RawResult<()> {
        self.pop(Register16::PC)?;
        Ok(())
    }

    fn bit(&mut self, reg: CommonRegister, bit: u8) -> RawResult<()> {
        let b = self.get_common_register(reg)?;
        let z = (b & (1 << bit)) == 0;
        self.cpu.edit_flags(Some(z), Some(false), Some(true), None);
        Ok(())
    }

    fn res(&mut self, reg: CommonRegister, bit: u8) -> RawResult<()> {
        let b = self.get_common_register(reg)?;
        let b = b & !(1 << bit);
        self.set_common_register(reg, b)
    }

    fn set(&mut self, reg: CommonRegister, bit: u8) -> RawResult<()> {
        let b = self.get_common_register(reg)?;
        let b = b | (1 << bit);
        self.set_common_register(reg, b)
    }

    fn add_8_set_flags(&mut self, a: u8, b: u8, with_carry: bool) -> u8 {
        let carry = if with_carry && self.cpu.get_flags().c {
            1
        } else {
            0
        };
        let result = (a as u16) + (b as u16) + carry;
        let c = result > 0xFF;
        let half_result = (a & 0x0F).wrapping_add(b & 0x0F);
        let h = half_result > 0x0F;
        let z = result == 0;
        self.cpu.edit_flags(Some(z), Some(true), Some(h), Some(c));
        result as u8
    }

    fn sub_8_set_flags(&mut self, a: u8, b: u8, with_carry: bool) -> u8 {
        let carry = if with_carry && self.cpu.get_flags().c {
            1
        } else {
            0
        };
        let result = (a as i16).wrapping_sub(b as i16).wrapping_sub(carry);
        let c = result < 0x00;
        let z = result == 0;
        let h = ((a & 0x0F) as i8).wrapping_sub((b & 0x0F) as i8) < 0x00;
        self.cpu.edit_flags(Some(z), Some(true), Some(h), Some(c));
        result as u8
    }

    fn and(&mut self, b: u8) {
        let a = self.cpu.get_register8(Register8::A);
        let result = a & b;
        let z = result == 0;
        self.cpu.set_register8(Register8::A, result);
        self.cpu
            .edit_flags(Some(z), Some(false), Some(true), Some(false));
    }

    fn or(&mut self, b: u8) {
        let a = self.cpu.get_register8(Register8::A);
        let result = a | b;
        let z = result == 0;
        self.cpu.set_register8(Register8::A, result);
        self.cpu
            .edit_flags(Some(z), Some(false), Some(false), Some(false));
    }

    fn xor(&mut self, b: u8) {
        let a = self.cpu.get_register8(Register8::A);
        let result = a ^ b;
        let z = result == 0;
        self.cpu.set_register8(Register8::A, result);
        self.cpu
            .edit_flags(Some(z), Some(false), Some(false), Some(false));
    }

    fn rlc(&mut self, r: CommonRegister) -> RawResult<()> {
        let a = self.get_common_register(r)?;
        let c = a & 0x80 > 0;
        let res = a.rotate_left(1);
        let z = res == 0;
        self.cpu
            .edit_flags(Some(z), Some(false), Some(false), Some(c));
        self.set_common_register(r, res)
    }

    fn rrc(&mut self, r: CommonRegister) -> RawResult<()> {
        let a = self.get_common_register(r)?;
        let c = a & 0x01 > 0;
        let res = a.rotate_right(1);
        let z = res == 0;
        self.cpu
            .edit_flags(Some(z), Some(false), Some(false), Some(c));
        self.set_common_register(r, res)
    }

    fn rl(&mut self, r: CommonRegister) -> RawResult<()> {
        let a = self.get_common_register(r)?;
        let cur_carry = self.cpu.get_flags().c;
        let c = a & 0x80 > 0;
        let res = a.rotate_left(1);
        let res = (res & 0xFE) | (if cur_carry { 1 } else { 0 });
        let z = res == 0;
        self.cpu
            .edit_flags(Some(z), Some(false), Some(false), Some(c));
        self.set_common_register(r, res)
    }

    fn rr(&mut self, r: CommonRegister) -> RawResult<()> {
        let a = self.get_common_register(r)?;
        let cur_carry = self.cpu.get_flags().c;
        let c = a & 0x01 > 0;
        let res = a.rotate_right(1);
        let res = (res & 0x7F) | (if cur_carry { 0x80 } else { 0 });
        let z = res == 0;
        self.cpu
            .edit_flags(Some(z), Some(false), Some(false), Some(c));
        self.set_common_register(r, res)
    }

    fn sla(&mut self, r: CommonRegister) -> RawResult<()> {
        let a = self.get_common_register(r)?;
        let c = a & 0x80 > 0;
        let res = a << 1;
        let z = res == 0;
        self.cpu
            .edit_flags(Some(z), Some(false), Some(false), Some(c));
        self.set_common_register(r, res)
    }

    fn sra(&mut self, r: CommonRegister) -> RawResult<()> {
        let a = self.get_common_register(r)?;
        let c = a & 0x01 > 0;
        let bit_7 = a & 0x80;
        let res = a >> 1;
        let res = (res & 0x7F) | bit_7;
        let z = res == 0;
        self.cpu
            .edit_flags(Some(z), Some(false), Some(false), Some(c));
        self.set_common_register(r, res)
    }

    fn srl(&mut self, r: CommonRegister) -> RawResult<()> {
        let a = self.get_common_register(r)?;
        let c = a & 0x01 > 0;
        let res = a >> 1;
        let res = res & 0x7F;
        let z = res == 0;
        self.cpu
            .edit_flags(Some(z), Some(false), Some(false), Some(c));
        self.set_common_register(r, res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bus::FlatBus;
    use crate::execution::instructions::{Immediate16, Immediate8};

    #[test]
    fn test_call() {
        let instruction = Instruction::CallImmediate(Immediate16(0x5050));
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::SP, 0x2);
        cpu.set_register16(Register16::PC, 0x1000);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();

        assert_eq!(cpu.get_register16(Register16::PC), 0x5050, "pc");
        assert_eq!(cpu.get_register16(Register16::SP), 0x00, "sp");
        assert_eq!(bus.read_word(0x00).unwrap(), 0x1003, "stack");
    }

    #[test]
    fn load_register_register() {
        let instruction = Instruction::LoadRegisterRegister(
            CommonRegister::Register8(Register8::C),
            CommonRegister::HLIndirect,
        );
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::HL, 0x0004);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00, 0x00, 0x00, 0xAB],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();

        assert_eq!(cpu.get_register8(Register8::C), 0xAB, "c");
        assert_eq!(cpu.get_register16(Register16::PC), 0x0001, "pc");
    }

    #[test]
    fn load_register_immediate8() {
        let instruction = Instruction::LoadRegisterImmediate8(
            CommonRegister::Register8(Register8::D),
            Immediate8(0xBA),
        );
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::HL, 0x0004);
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();

        assert_eq!(cpu.get_register8(Register8::D), 0xBA, "d");
        assert_eq!(cpu.get_register16(Register16::PC), 0x0002, "pc");
    }

    #[test]
    fn load_indirect_register_a() {
        let instruction = Instruction::LoadIndirectRegisterA(Register16::DE);
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::DE, 0x0004);
        cpu.set_register8(Register8::A, 0xFF);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00, 0x00, 0x00, 0x00],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();

        assert_eq!(bus.read_byte(0x04).unwrap(), 0xFF, "mem")
    }

    #[test]
    fn load_indirect_immediate_16_a() {
        let instruction = Instruction::LoadIndirectImmediate16A(Immediate16(0x0002));
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::A, 0x55);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00, 0x04],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();

        assert_eq!(bus.read_byte(0x0002).unwrap(), 0x55, "a")
    }

    #[test]
    fn load_a_hl_increment() {
        let instruction = Instruction::LoadAIncrementHLIndirect;
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::HL, 0x0002);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00, 0x04],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();

        assert_eq!(cpu.get_register8(Register8::A), 0x04, "a");
        assert_eq!(cpu.get_register16(Register16::HL), 0x0003, "hl");
    }

    #[test]
    fn load_a_hl_decrement() {
        let instruction = Instruction::LoadADecrementHLIndirect;
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::HL, 0x0002);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00, 0x04],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();

        assert_eq!(cpu.get_register8(Register8::A), 0x04, "a");
        assert_eq!(cpu.get_register16(Register16::HL), 0x0001, "hl");
    }

    #[test]
    fn load_hl_increment_a() {
        let instruction = Instruction::LoadIncrementHLIndirectA;
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::HL, 0x0002);
        cpu.set_register8(Register8::A, 0x55);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00, 0x04],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();

        assert_eq!(bus.read_byte(0x0002).unwrap(), 0x55, "mem");
        assert_eq!(cpu.get_register16(Register16::HL), 0x0003, "hl");
    }

    #[test]
    fn load_hl_decrement_a() {
        let instruction = Instruction::LoadDecrementHLAIndirect;
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::HL, 0x0002);
        cpu.set_register8(Register8::A, 0x55);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00, 0x04],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();

        assert_eq!(bus.read_byte(0x0002).unwrap(), 0x55, "mem");
        assert_eq!(cpu.get_register16(Register16::HL), 0x0001, "hl");
    }

    #[test]
    fn inc_r16() {
        let instruction = Instruction::IncRegister16(Register16::BC);
        let mut cpu = Cpu::zeroed();
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register16(Register16::BC), 1, "bc");
    }

    #[test]
    fn bit_0_b() {
        let instruction = Instruction::BitRegister(0, CommonRegister::Register8(Register8::B));
        let mut cpu = Cpu::zeroed();
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: true,
                n: false,
                h: true,
                c: false
            }
        )
    }

    #[test]
    fn bit_0_b_inverse() {
        let instruction = Instruction::BitRegister(0, CommonRegister::Register8(Register8::B));
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::B, 0x01);
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: false,
                n: false,
                h: true,
                c: false
            },
            "flags"
        )
    }

    #[test]
    fn rl_b() {
        let instruction = Instruction::RotateLeftRegister(CommonRegister::Register8(Register8::B));
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::B, 0xAA);
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register8(Register8::B), 0x55, "b");
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: false,
                n: false,
                h: false,
                c: true
            },
            "flags"
        );
    }

    #[test]
    fn rlc_b() {
        let instruction =
            Instruction::RotateLeftThroughCarryRegister(CommonRegister::Register8(Register8::B));
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::B, 0xAA);
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register8(Register8::B), 0x54, "b");
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: false,
                n: false,
                h: false,
                c: true
            },
            "flags"
        );
    }

    #[test]
    fn rr_b() {
        let instruction = Instruction::RotateRightRegister(CommonRegister::Register8(Register8::B));
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::B, 0x55);
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register8(Register8::B), 0xAA, "b");
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: false,
                n: false,
                h: false,
                c: true
            },
            "flags"
        );
    }

    #[test]
    fn rrc_b() {
        let instruction =
            Instruction::RotateRightThroughCarryRegister(CommonRegister::Register8(Register8::B));
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::B, 0x55);
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register8(Register8::B), 0x2A, "b");
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: false,
                n: false,
                h: false,
                c: true
            },
            "flags"
        );
    }

    #[test]
    fn sla_b() {
        let instruction = Instruction::ShiftLeftRegister(CommonRegister::Register8(Register8::B));
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::B, 0xAA);
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register8(Register8::B), 0x54, "b");
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: false,
                n: false,
                h: false,
                c: true
            },
            "flags"
        );
    }

    #[test]
    fn sra_b_negative() {
        let instruction =
            Instruction::ShiftRightArithmeticRegister(CommonRegister::Register8(Register8::B));
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::B, 0xA5);
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register8(Register8::B), 0xD2, "b");
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: false,
                n: false,
                h: false,
                c: true
            },
            "flags"
        );
    }

    #[test]
    fn sra_b_positive() {
        let instruction =
            Instruction::ShiftRightArithmeticRegister(CommonRegister::Register8(Register8::B));
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::B, 0x55);
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register8(Register8::B), 0x2A, "b");
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: false,
                n: false,
                h: false,
                c: true
            },
            "flags"
        );
    }

    #[test]
    fn srl_b() {
        let instruction =
            Instruction::ShiftRightLogicalRegister(CommonRegister::Register8(Register8::B));
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::B, 0x55);
        let mut bus = FlatBus { mem: vec![] };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register8(Register8::B), 0x2A, "b");
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: false,
                n: false,
                h: false,
                c: true
            },
            "flags"
        );
    }

    #[test]
    fn xor_hl() {
        let instruction = Instruction::XorRegister(CommonRegister::HLIndirect);
        let mut cpu = Cpu::zeroed();
        cpu.set_register8(Register8::A, 0x55);
        cpu.set_register16(Register16::HL, 0x02);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00, 0x5A],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register8(Register8::A), 0x0F, "a");
        assert_eq!(
            cpu.get_flags(),
            &Flags {
                z: false,
                n: false,
                h: false,
                c: false
            },
            "flags"
        );
    }

    #[test]
    fn call() {
        let instruction = Instruction::CallImmediate(Immediate16(0x1234));
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::SP, 0x0002);
        cpu.set_register16(Register16::PC, 0xAAAA);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00, 0x00],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register16(Register16::PC), 0x1234, "pc");
        assert_eq!(cpu.get_register16(Register16::SP), 0x0000, "sp");
        assert_eq!(bus.read_word(0x0000).unwrap(), 0xAAAA + 3, "at sp");
    }

    #[test]
    fn push_pop() {
        let instruction = Instruction::Push(Register16::HL);
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::HL, 0x1234);
        cpu.set_register16(Register16::SP, 0x0002);
        let mut bus = FlatBus {
            mem: vec![0x00, 0x00],
        };
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register16(Register16::SP), 0x0000, "sp");
        assert_eq!(bus.read_word(0x0000).unwrap(), 0x1234, "mem");

        let instruction = Instruction::Pop(Register16::BC);
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register16(Register16::BC), 0x1234, "bc");
    }

    #[test]
    fn add_sp_s8_1() {
        let instruction = Instruction::AddSPImmediate(Immediate8(0x7F));
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::SP, 0x1000);
        let mut bus = FlatBus {mem: vec![]};
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register16(Register16::SP), 0x107F, "sp");
    }

    #[test]
    fn add_sp_s8_2() {
        let instruction = Instruction::AddSPImmediate(Immediate8(0xFF));
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::SP, 0x1000);
        let mut bus = FlatBus {mem: vec![]};
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register16(Register16::SP), 0x0FFF, "sp");
    }

    #[test]
    fn add_sp_s8_3() {
        let instruction = Instruction::AddSPImmediate(Immediate8(0x01));
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::SP, 0x000F);
        let mut bus = FlatBus {mem: vec![]};
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register16(Register16::SP), 0x0010, "sp");
        assert!(cpu.get_flags().h, "h");
    }

    #[test]
    fn add_sp_s8_4() {
        let instruction = Instruction::AddSPImmediate(Immediate8(0xFF));
        let mut cpu = Cpu::zeroed();
        cpu.set_register16(Register16::SP, 0xFFFF);
        let mut bus = FlatBus {mem: vec![]};
        execute_instruction(&mut cpu, &mut bus, instruction).unwrap();
        assert_eq!(cpu.get_register16(Register16::SP), 0xFFFE, "sp");
        assert!(cpu.get_flags().h, "h");
        assert!(cpu.get_flags().c, "c");
    }
}
