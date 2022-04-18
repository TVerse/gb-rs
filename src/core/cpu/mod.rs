mod instructions;
pub mod registers;

use crate::core::cpu::instructions::{ArithmeticOperation, CommonRegister, Immediate16, Immediate8, Instruction, JumpCondition, RotationShiftOperation};
use crate::core::cpu::registers::{Flags, Register16, Register8};
use crate::core::{Clock, ExecuteContext, MemoryView};
use registers::Registers;

#[derive(Default, Debug)]
pub struct Cpu {
    registers: Registers,
    interrupt_master_enable: bool,
}

impl Cpu {
    pub fn get_first_opcode<M: MemoryView>(&mut self, mem: &M) -> u8 {
        let opcode = mem.read(self.registers.read_register16(Register16::PC));
        self.registers.increment_pc();
        // TODO should the clock tick here? If so, forward to Execution?
        opcode
    }

    pub fn decode_execute_fetch<M: MemoryView, CLOCK: Clock, CONTEXT: ExecuteContext>(
        &mut self,
        opcode: u8,
        mem: &mut M,
        clock: &mut CLOCK,
        context: &mut CONTEXT,
    ) -> (Instruction, u8) {
        let mut execution = Execution {
            cpu: self,
            mem,
            clock,
            context,
        };
        execution.decode_execute_fetch(opcode)
    }
}

struct Execution<'a, M: MemoryView, CLOCK: Clock, CONTEXT: ExecuteContext> {
    cpu: &'a mut Cpu,
    mem: &'a mut M,
    clock: &'a mut CLOCK,
    context: &'a mut CONTEXT,
}

impl<'a, M: MemoryView, CLOCK: Clock, CONTEXT: ExecuteContext> Execution<'a, M, CLOCK, CONTEXT> {
    /*
       Notes:
       * Post-increment PC, always. Current PC is suitable for use/peeking.
       * Clock ticks are coupled to memory reads, and therefore also handled by fetch_next_opcode.
       * Any reads at PC also increment and tick.
    */
    pub fn decode_execute_fetch(&mut self, opcode: u8) -> (Instruction, u8) {
        let x = (opcode & 0b11000000) >> 6;
        let y = (opcode & 0b00111000) >> 3;
        let z = (opcode & 0b00000111) >> 0;
        let p = (y & 0b110) >> 1;
        let q = (y & 0b1) >> 0;
        let instruction: Instruction = match x {
            0 => self.x_is_0_tree(y, z, p, q),
            1 => {
                let target = CommonRegister::from_u8(y);
                let source = CommonRegister::from_u8(z);
                if target == CommonRegister::HLIndirect && source == CommonRegister::HLIndirect {
                    self.halt()
                } else {
                    self.ld_r_r(target, source)
                }
            }
            2 => {
                let op = ArithmeticOperation::from_u8(y);
                let reg = CommonRegister::from_u8(z);
                self.alu_reg(op, reg)
            }
            3 => self.x_is_3_tree(y, z, p, q),
            _ => panic!("Invalid opcode"),
        };

        (instruction, self.fetch_opcode())
    }

    fn x_is_0_tree(&mut self, y: u8, z: u8, p: u8, q: u8) -> Instruction {
        match z {
            0 => match y {
                0 => self.noop(),
                1 => self.ld_inn_sp(),
                2 => panic!("STOP"),
                3 => self.jr(),
                y => {
                    let cc = JumpCondition::from_u8(y - 4);
                    self.jr_cc(cc)
                }
            },
            1 => {
                let rp = Register16::from_byte_sp(p);
                match q {
                    0 => self.ld_rp_nn(rp),
                    1 => self.add_hl_rp(rp),
                    _ => unreachable!(),
                }
            }
            2 => match q {
                0 => match p {
                    0 => self.ld_irp_a(Register16::BC),
                    1 => self.ld_irp_a(Register16::DE),
                    2 => self.ld_hlp_a(),
                    3 => self.ld_hlm_a(),
                    _ => unreachable!(),
                },
                1 => match p {
                    0 => self.ld_a_irp(Register16::BC),
                    1 => self.ld_a_irp(Register16::DE),
                    2 => self.ld_a_hlp(),
                    3 => self.ld_a_hlm(),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            3 => {
                let rp = Register16::from_byte_sp(p);
                match q {
                    0 => self.inc_16(rp),
                    1 => self.dec_16(rp),
                    _ => unreachable!(),
                }
            }
            4 => {
                let r = CommonRegister::from_u8(y);
                self.inc(r)
            }
            5 => {
                let r = CommonRegister::from_u8(y);
                self.dec(r)
            }
            6 => {
                let r = CommonRegister::from_u8(y);
                self.ld_r_n(r)
            }
            7 => match y {
                0 => self.rlca(),
                1 => self.rrca(),
                2 => self.rla(),
                3 => self.rra(),
                4 => self.daa(),
                5 => self.cpl(),
                6 => self.scf(),
                7 => self.ccf(),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    fn x_is_3_tree(&mut self, y: u8, z: u8, p: u8, q: u8) -> Instruction {
        todo!()
    }

    fn read_byte_at(&mut self, addr: u16) -> u8 {
        let b = self.mem.read(addr);
        self.cpu.registers.increment_pc();
        self.clock.tick();
        b
    }

    fn read_byte_at_pc(&mut self) -> u8 {
        self.read_byte_at(self.cpu.registers.read_register16(Register16::PC))
    }

    fn read_word_at(&mut self, addr: u16) -> u16 {
        let lower = self.read_byte_at(addr);
        let upper = self.read_byte_at(addr.wrapping_add(1));
        ((upper as u16) << 8) | (lower as u16)
    }

    fn read_word_at_pc(&mut self) -> u16 {
        let lower = self.read_byte_at_pc();
        let upper = self.read_byte_at_pc();
        ((upper as u16) << 8) | (lower as u16)
    }

    fn write_byte_to(&mut self, addr: u16, b: u8) {
        self.mem.write(addr, b);
        self.clock.tick();
    }

    fn write_word_to(&mut self, addr: u16, w: u16) {
        let lsb = w as u8;
        let msb = (w >> 8) as u8;

        self.write_byte_to(addr, lsb);
        self.write_byte_to(addr.wrapping_add(1), msb);
    }

    fn noop(&mut self) -> Instruction {
        Instruction::Nop
    }

    fn ld_inn_sp(&mut self) -> Instruction {
        let addr = self.read_word_at_pc();
        let sp = self.cpu.registers.read_register16(Register16::SP);
        self.write_word_to(addr, sp);
        Instruction::LoadIndirectImmediate16SP(Immediate16(addr))
    }

    fn jr(&mut self) -> Instruction {
        let offset = self.read_byte_at_pc();
        let ioffset = offset as i8;
        self.clock.tick();
        self.cpu.registers.write_register16(
            Register16::PC,
            add_i8_to_u16(ioffset, self.cpu.registers.read_register16(Register16::PC)),
        );
        Instruction::JumpRelative(Immediate8(offset))
    }

    fn jr_cc(&mut self, cc: JumpCondition) -> Instruction {
        let offset = self.read_byte_at_pc();
        let ioffset = offset as i8;

        if self.should_jump(cc) {
            self.clock.tick();
            self.cpu.registers.write_register16(
                Register16::PC,
                add_i8_to_u16(ioffset, self.cpu.registers.read_register16(Register16::PC)),
            );
        }
        Instruction::JumpConditionalRelative(cc, Immediate8(offset))
    }

    fn load_rp_nn(&mut self, rp: Register16) -> Instruction {
        let nn = self.read_word_at_pc();
        self.cpu.registers.write_register16(rp, nn);
        Instruction::LoadRegisterImmediate16(rp, Immediate16(nn))
    }

    fn add_hl_rp(&mut self, rp: Register16) -> Instruction {
        let src = self.cpu.registers.read_register16(rp);
        let lsb = src as u8;
        let msb = (src >> 8) as u8;

        let l = self.cpu.registers.read_register8(Register8::L);
        let l_res = self.add_8bit(l, lsb);
        self.cpu.registers.write_register8(Register8::L, l_res);
        self.clock.tick();
        let h = self.cpu.registers.read_register8(Register8::H);
        let h_res = self.add_8bit_carry(h, msb);
        self.cpu.registers.write_register8(Register8::H, h_res);

        Instruction::AddHLRegister(rp)
    }

    fn add_8bit(&mut self, a: u8, b: u8) -> u8 {
        let (res, carry) = a.carrying_add(b, false);
        let h = (a & 0x0F) + (b & 0x0F) > 0x0F;
        let z= res == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::C, carry);
            f.set(Flags::H, h);
            f.set(Flags::Z, z);
            f.remove(Flags::N);
        });
        res
    }

    fn add_8bit_carry(&mut self, a: u8, b: u8) -> u8 {
        let carry = self.cpu.registers.flags().contains(Flags::C);
        let (res, carry) = a.carrying_add(b, carry);
        let h = (a & 0x0F) + (b & 0x0F) + (if carry { 1 } else { 0 }) > 0x0F;
        let z= res == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::C, carry);
            f.set(Flags::H, h);
            f.set(Flags::Z, z);
            f.remove(Flags::N);
        });

        res
    }

    // TODO figure out if these can be merged with add
    fn sub(&mut self, a: u8, b: u8) -> u8 {
        let result = (a as i16).wrapping_sub(b as i16);
        let c = result < 0x00;
        let result = result as u8;
        let z = result == 0;
        let h = ((a & 0x0F) as i8)
            .wrapping_sub((b & 0x0F) as i8)
            < 0x00;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.set(Flags::C, c);
            f.set(Flags::H, h);
            f.insert(Flags::N);
        });
        result
    }

    fn sub_carry(&mut self, a: u8, b: u8) -> u8 {
        let carry = if self.cpu.registers.flags().contains(Flags::C) {1} else {0};
        let result = (a as i16).wrapping_sub(b as i16).wrapping_sub(carry);
        let c = result < 0x00;
        let result = result as u8;
        let z = result == 0;
        let h = ((a & 0x0F) as i8)
            .wrapping_sub((b & 0x0F) as i8)
            .wrapping_sub(carry as i8)
            < 0x00;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.set(Flags::C, c);
            f.set(Flags::H, h);
            f.insert(Flags::N);
        });
        result
    }

    fn and(&mut self, a: u8, b: u8) -> u8 {
        let result = a & b;
        let z = result == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.remove(Flags::N| Flags::C);
            f.insert(Flags::H);
        });
        result
    }

    fn or(&mut self, a: u8, b: u8) -> u8 {
        let result = a | b;
        let z = result == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.remove(Flags::N | Flags::C | Flags::H);
        });
        result
    }

    fn xor(&mut self, a: u8, b: u8) -> u8 {
        let result = a ^ b;
        let z = result == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.remove(Flags::N | Flags::C | Flags::H);
        });
        result
    }

    fn fetch_opcode(&mut self) -> u8 {
        self.read_byte_at_pc()
    }

    fn should_jump(&self, cc: JumpCondition) -> bool {
        match cc {
            JumpCondition::NZ => !self.cpu.registers.flags().contains(Flags::Z),
            JumpCondition::Z => self.cpu.registers.flags().contains(Flags::Z),
            JumpCondition::NC => !self.cpu.registers.flags().contains(Flags::C),
            JumpCondition::C => self.cpu.registers.flags().contains(Flags::C),
        }
    }

    fn read_common_register(&mut self, reg: CommonRegister) -> u8 {
        match reg {
            CommonRegister::Register8(r) => self.cpu.registers.read_register8(r),
            CommonRegister::HLIndirect => {
                self.read_byte_at(self.cpu.registers.read_register16(Register16::HL))
            }
        }
    }

    fn write_common_register(&mut self, reg: CommonRegister, value: u8) {
        match reg {
            CommonRegister::Register8(r) => self.cpu.registers.write_register8(r, value),
            CommonRegister::HLIndirect => {
                self.write_byte_to(self.cpu.registers.read_register16(Register16::HL), value)
            }
        }
    }

    fn halt(&self) -> Instruction {
        todo!()
    }
    fn ld_r_r(&self, target: CommonRegister, source: CommonRegister) -> Instruction {
        debug_assert!(target != CommonRegister::HLIndirect || source != CommonRegister::HLIndirect);

        Instruction::LoadRegisterRegister(target, source)
    }
    fn alu_reg(&mut self, op: ArithmeticOperation, reg: CommonRegister) -> Instruction {
        let to_add = self.read_common_register(reg);
        self.alu(op, to_add);

        Instruction::AluRegister(op, reg)
    }
    fn alu_imm(&mut self, op: ArithmeticOperation) -> Instruction {
        let to_add = self.read_byte_at_pc();
        self.alu(op, to_add);

        Instruction::AluImmediate(op, Immediate8(to_add))
    }
    fn alu(&mut self, op: ArithmeticOperation, to_add: u8) {
        let a = self.cpu.registers.read_register8(Register8::A);
        let res = match op {
            ArithmeticOperation::AddA => self.add_8bit(a, to_add),
            ArithmeticOperation::AdcA => self.add_8bit_carry(a, to_add),
            ArithmeticOperation::Sub => self.sub(a, to_add),
            ArithmeticOperation::SbcA => self.sub_carry(a, to_add),
            ArithmeticOperation::And => self.and(a, to_add),
            ArithmeticOperation::Xor => self.xor(a, to_add),
            ArithmeticOperation::Or => self.or(a, to_add),
            ArithmeticOperation::Cp => self.sub(a, to_add),
        };
        if op != ArithmeticOperation::Cp {
            self.cpu.registers.write_register8(Register8::A, res)
        }
    }

    fn ld_rp_nn(&mut self, reg: Register16) -> Instruction {
        let word = self.read_word_at_pc();
        self.cpu.registers.write_register16(reg, word);

        Instruction::LoadRegisterImmediate16(reg, Immediate16(word))
    }
    fn ld_irp_a(&mut self, rp: Register16) -> Instruction {
        let res = self.cpu.registers.read_register8(Register8::A);
        self.mem.write(self.cpu.registers.read_register16(rp), res);
        Instruction::LoadIndirectRegisterA(rp)
    }
    fn ld_hlp_a(&mut self) -> Instruction {
        let res = self.cpu.registers.read_register8(Register8::A);
        self.mem
            .write(self.cpu.registers.read_register16(Register16::HL), res);
        self.cpu.registers.write_register16(
            Register16::HL,
            self.cpu
                .registers
                .read_register16(Register16::HL)
                .wrapping_add(1),
        );
        Instruction::LoadIncrementHLIndirectA
    }
    fn ld_hlm_a(&mut self) -> Instruction {
        let res = self.cpu.registers.read_register8(Register8::A);
        self.mem
            .write(self.cpu.registers.read_register16(Register16::HL), res);
        self.cpu.registers.write_register16(
            Register16::HL,
            self.cpu
                .registers
                .read_register16(Register16::HL)
                .wrapping_sub(1),
        );
        Instruction::LoadDecrementHLIndirectA
    }
    fn ld_a_irp(&mut self, rp: Register16) -> Instruction {
        let res = self.mem.read(self.cpu.registers.read_register16(rp));
        self.cpu.registers.write_register8(Register8::A, res);
        Instruction::LoadAIndirectRegister(rp)
    }
    fn ld_a_hlp(&mut self) -> Instruction {
        let res = self
            .mem
            .read(self.cpu.registers.read_register16(Register16::HL));
        self.cpu.registers.write_register16(
            Register16::HL,
            self.cpu
                .registers
                .read_register16(Register16::HL)
                .wrapping_add(1),
        );
        self.cpu.registers.write_register8(Register8::A, res);
        Instruction::LoadAIncrementHLIndirect
    }
    fn ld_a_hlm(&mut self) -> Instruction {
        let res = self
            .mem
            .read(self.cpu.registers.read_register16(Register16::HL));
        self.cpu.registers.write_register16(
            Register16::HL,
            self.cpu
                .registers
                .read_register16(Register16::HL)
                .wrapping_sub(1),
        );
        self.cpu.registers.write_register8(Register8::A, res);
        Instruction::LoadADecrementHLIndirect
    }
    fn inc_16(&mut self, rp: Register16) -> Instruction {
        // TODO assuming intermediate 8bit is not observable, since no flags are set
        self.cpu
            .registers
            .write_register16(rp, self.cpu.registers.read_register16(rp).wrapping_add(1));
        self.clock.tick();
        Instruction::IncRegister16(rp)
    }
    fn dec_16(&mut self, rp: Register16) -> Instruction {
        // TODO assuming intermediate 8bit is not observable, since no flags are set
        self.cpu
            .registers
            .write_register16(rp, self.cpu.registers.read_register16(rp).wrapping_sub(1));
        self.clock.tick();
        Instruction::DecRegister16(rp)
    }
    fn inc(&mut self, reg: CommonRegister) -> Instruction {
        let val = self.read_common_register(reg);
        let res = val.wrapping_add(1);
        let mut flags = Flags::empty();
        if res == 0 {
            flags |= Flags::Z;
        }
        if (val & 0x0F) + 1 > 0x0F {
            flags |= Flags::H;
        }
        self.cpu.registers.modify_flags(|f| {
            f.insert(flags);
            f.remove(Flags::H);
        });

        self.write_common_register(reg, res);
        Instruction::IncRegister8(reg)
    }
    fn dec(&mut self, reg: CommonRegister) -> Instruction {
        let val = self.read_common_register(reg);
        let res = val.wrapping_add(1);
        let mut flags = Flags::N;
        if res == 0 {
            flags |= Flags::Z;
        }
        if (val & 0xF0) - 1 < 0x10 {
            flags |= Flags::H;
        }
        self.cpu.registers.modify_flags(|f| {
            f.insert(flags);
        });

        self.write_common_register(reg, res);
        Instruction::IncRegister8(reg)
    }
    fn ld_r_n(&mut self, reg: CommonRegister) -> Instruction {
        let n = self.read_byte_at_pc();
        self.write_common_register(reg, n);
        Instruction::LoadRegisterImmediate8(reg, Immediate8(n))
    }
    fn rl(&mut self, register: CommonRegister) -> Instruction {
        let a = self.read_common_register(register);
        let cur_carry = self.cpu.registers.flags().contains(Flags::C);
        let c = a & 0x80 > 0;
        let res = a.rotate_left(1);
        let res = res & 0xFE | (if cur_carry {1} else {0});
        let z = res == 0;
        self.cpu.registers.modify_flags(|f|{
            f.set(Flags::C, c);
            f.set(Flags::Z, z);
            f.remove(Flags::H | Flags::N);
        });
        self.write_common_register(register, res);

        Instruction::RotateShiftRegister(RotationShiftOperation::Rl, register)
    }
    fn rla(&mut self) -> Instruction {
        self.rl(CommonRegister::Register8(Register8::A));
        self.cpu.registers.modify_flags(|f| f.remove(Flags::Z));

        Instruction::RotateALeftThroughCarry
    }
    fn rr(&mut self, register: CommonRegister) -> Instruction {
        let a = self.read_common_register(register);
        let cur_carry = self.cpu.registers.flags().contains(Flags::C);
        let c = a & 0x01 > 0;
        let res = a.rotate_right(1);
        let res = res & 0x7F | (if cur_carry {0x80} else {0});
        let z = res == 0;
        self.cpu.registers.modify_flags(|f|{
            f.set(Flags::C, c);
            f.set(Flags::Z, z);
            f.remove(Flags::H | Flags::N);
        });
        self.write_common_register(register, res);

        Instruction::RotateShiftRegister(RotationShiftOperation::Rr, register)
    }
    fn rra(&mut self) -> Instruction {
        self.rr(CommonRegister::Register8(Register8::A));
        self.cpu.registers.modify_flags(|f| f.remove(Flags::Z));

        Instruction::RotateARightThroughCarry
    }
    fn rlc(&mut self, register: CommonRegister) -> Instruction {
        let a = self.read_common_register(register);
        let c = a & 0x80 > 0;
        let res = a.rotate_left(1);
        let z = res == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.set(Flags::C, c);
            f.remove(Flags::H | Flags::N);
        });
        self.write_common_register(register, res);

        Instruction::RotateShiftRegister(RotationShiftOperation::Rlc, register)
    }
    fn rlca(&mut self) -> Instruction {
        self.rlc(CommonRegister::Register8(Register8::A));
        self.cpu.registers.modify_flags(|f| f.remove(Flags::Z));

        Instruction::RotateALeft
    }
    fn rrc(&mut self, register: CommonRegister) -> Instruction {
        let a = self.read_common_register(register);
        let c = a & 0x01 > 0;
        let res = a.rotate_right(1);
        let z = res == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.set(Flags::C, c);
            f.remove(Flags::H | Flags::N);
        });
        self.write_common_register(register, res);

        Instruction::RotateShiftRegister(RotationShiftOperation::Rrc, register)
    }
    fn rrca(&mut self) -> Instruction {
        self.rrc(CommonRegister::Register8(Register8::A));
        self.cpu.registers.modify_flags(|f| f.remove(Flags::Z));

        Instruction::RotateARight
    }
    fn daa(&mut self) -> Instruction {
        let flags = self.cpu.registers.flags();
        let mut a = self.cpu.registers.read_register8(Register8::A);
        let mut c = false;
        if !flags.contains(Flags::N) {
            if flags.contains(Flags::C) || a > 0x99 {
                a = a.wrapping_add(0x60);
                c = true;
            }
            if flags.contains(Flags::H) || (a & 0x0F) > 0x09 {
                a = a.wrapping_add(0x06);
            }
        } else {
            if flags.contains(Flags::C) {
                a = a.wrapping_sub(0x60);
                c = true;
            }
            if flags.contains(Flags::H) {
                a = a.wrapping_sub(0x06);
            }
        }

        let z = a == 0;
        self.cpu.registers.write_register8(Register8::A, a);
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.set(Flags::C, c);
            f.remove(Flags::H)
        });

        Instruction::DecimalAdjust
    }
    fn cpl(&mut self) -> Instruction {
        let a = self.cpu.registers.read_register8(Register8::A);
        self.cpu.registers.write_register8(Register8::A, !a);
        self.cpu.registers.modify_flags(|f| {
            f.insert(Flags::H | Flags::N);
        });

        Instruction::Complement
    }
    fn scf(&mut self) -> Instruction {
       self.cpu.registers.modify_flags(|f| {
           f.remove(Flags::N | Flags::H);
           f.insert(Flags::C);
       });

        Instruction::Scf
    }
    fn ccf(&mut self) -> Instruction {
        self.cpu.registers.modify_flags(|f| {
            f.remove(Flags::N | Flags::H);
            f.toggle(Flags::C);
        });

        Instruction::Ccf
    }
}

fn add_i8_to_u16(a: i8, b: u16) -> u16 {
    (a as u16).wrapping_add(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::testsupport::*;

    #[test]
    fn noop() {
        let mut cpu = Cpu::default();
        let mut memory = TestMemoryView::default();
        memory.mem[1] = 0xFF;
        let mut clock = CycleCountingClock::default();
        let mut context = NoopContext::default();

        let opcode = cpu.get_first_opcode(&memory);

        let (instruction, next_opcode) =
            cpu.decode_execute_fetch(opcode, &mut memory, &mut clock, &mut context);

        assert_eq!(instruction, Instruction::Nop);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(clock.cycles, 1);
    }

    #[test]
    fn ld_inn_sp() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::SP, 0x1234);
        let mut memory = TestMemoryView::default();
        memory.mem[0] = 0x08;
        memory.mem[1] = 0x10;
        memory.mem[2] = 0x00;
        memory.mem[3] = 0xFF;
        let mut clock = CycleCountingClock::default();
        let mut context = NoopContext::default();

        let opcode = cpu.get_first_opcode(&memory);

        let (instruction, next_opcode) =
            cpu.decode_execute_fetch(opcode, &mut memory, &mut clock, &mut context);

        assert_eq!(
            instruction,
            Instruction::LoadIndirectImmediate16SP(Immediate16(0x0010))
        );
        assert_eq!(memory.mem[0x0010], 0x34);
        assert_eq!(memory.mem[0x0011], 0x12);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(clock.cycles, 5);
    }

    #[test]
    fn jr_positive() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::PC, 0x1234);
        let mut memory = TestMemoryView::default();
        memory.mem[0x1234] = 0x18;
        memory.mem[0x1235] = 0x05;
        memory.mem[0x123B] = 0xFF;
        let mut clock = CycleCountingClock::default();
        let mut context = NoopContext::default();

        let opcode = cpu.get_first_opcode(&memory);

        let (instruction, next_opcode) =
            cpu.decode_execute_fetch(opcode, &mut memory, &mut clock, &mut context);

        assert_eq!(instruction, Instruction::JumpRelative(Immediate8(0x05)));
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(clock.cycles, 3);
    }

    #[test]
    fn jr_negative() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::PC, 0x1234);
        let mut memory = TestMemoryView::default();
        memory.mem[0x1234] = 0x18;
        memory.mem[0x1235] = 0xFD;
        memory.mem[0x1233] = 0xFF;
        let mut clock = CycleCountingClock::default();
        let mut context = NoopContext::default();

        let opcode = cpu.get_first_opcode(&memory);

        let (instruction, next_opcode) =
            cpu.decode_execute_fetch(opcode, &mut memory, &mut clock, &mut context);

        assert_eq!(instruction, Instruction::JumpRelative(Immediate8(0xFD)));
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(clock.cycles, 3);
    }

    #[test]
    fn jr_cc_taken() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::PC, 0x1234);
        cpu.registers.modify_flags(|f| f.insert(Flags::Z));
        let mut memory = TestMemoryView::default();
        memory.mem[0x1234] = 0b00101000;
        memory.mem[0x1235] = 0x05;
        memory.mem[0x123B] = 0xFF;
        let mut clock = CycleCountingClock::default();
        let mut context = NoopContext::default();

        let opcode = cpu.get_first_opcode(&memory);

        let (instruction, next_opcode) =
            cpu.decode_execute_fetch(opcode, &mut memory, &mut clock, &mut context);

        assert_eq!(
            instruction,
            Instruction::JumpConditionalRelative(JumpCondition::Z, Immediate8(0x05))
        );
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(clock.cycles, 3);
    }

    #[test]
    fn jr_cc_not_taken() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::PC, 0x1234);
        let mut memory = TestMemoryView::default();
        memory.mem[0x1234] = 0b00101000;
        memory.mem[0x1235] = 0x05;
        memory.mem[0x1236] = 0xFF;
        let mut clock = CycleCountingClock::default();
        let mut context = NoopContext::default();

        let opcode = cpu.get_first_opcode(&memory);

        let (instruction, next_opcode) =
            cpu.decode_execute_fetch(opcode, &mut memory, &mut clock, &mut context);

        assert_eq!(
            instruction,
            Instruction::JumpConditionalRelative(JumpCondition::Z, Immediate8(0x05))
        );
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(clock.cycles, 2);
    }

    #[test]
    fn add_hl_rp() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::HL, 0xFFFF);
        cpu.registers.write_register16(Register16::BC, 0x0001);
        let mut memory = TestMemoryView::default();
        memory.mem[0] = 0x09;
        memory.mem[1] = 0xFF;
        let mut clock = CycleCountingClock::default();
        let mut context = NoopContext::default();

        let opcode = cpu.get_first_opcode(&memory);

        let (instruction, next_opcode) =
            cpu.decode_execute_fetch(opcode, &mut memory, &mut clock, &mut context);

        assert_eq!(instruction, Instruction::AddHLRegister(Register16::BC));
        assert_eq!(cpu.registers.read_register16(Register16::HL), 0);
        assert_eq!(cpu.registers.flags(), Flags::Z | Flags::H | Flags::C);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(clock.cycles, 2);

        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::HL, 0x0EFF);
        cpu.registers.write_register16(Register16::BC, 0x0001);
        let mut memory = TestMemoryView::default();
        memory.mem[0] = 0x09;
        memory.mem[1] = 0xFF;
        let mut clock = CycleCountingClock::default();
        let mut context = NoopContext::default();

        let opcode = cpu.get_first_opcode(&memory);

        let (instruction, next_opcode) =
            cpu.decode_execute_fetch(opcode, &mut memory, &mut clock, &mut context);

        assert_eq!(instruction, Instruction::AddHLRegister(Register16::BC));
        assert_eq!(cpu.registers.read_register16(Register16::HL), 0x0F00);
        assert_eq!(cpu.registers.flags(), Flags::empty());
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(clock.cycles, 2);
    }

    #[test]
    fn sub_n() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register8(Register8::A, 10);
        cpu.registers.write_register8(Register8::B, 5);
        let mut memory = TestMemoryView::default();
        memory.mem[0] = 0x90;
        memory.mem[1] = 0xFF;
        let mut clock = CycleCountingClock::default();
        let mut context = NoopContext::default();

        let opcode = cpu.get_first_opcode(&memory);

        let (instruction, next_opcode) =
            cpu.decode_execute_fetch(opcode, &mut memory, &mut clock, &mut context);

        assert_eq!(instruction, Instruction::AluRegister(ArithmeticOperation::Sub, CommonRegister::Register8(Register8::B)));
        assert_eq!(cpu.registers.read_register8(Register8::A), 5);
        assert_eq!(cpu.registers.flags(), Flags::N);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(clock.cycles, 1);
    }

    #[test]
    fn add_i8_to_u16() {
        let a: i8 = 127;
        let b: u16 = 127;
        assert_eq!(super::add_i8_to_u16(a, b), 254);

        let a: i8 = -50;
        let b: u16 = 5050;
        assert_eq!(super::add_i8_to_u16(a, b), 5000);
    }
}
