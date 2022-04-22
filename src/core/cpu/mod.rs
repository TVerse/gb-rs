pub mod instructions;
pub mod registers;

use crate::core::cpu::instructions::{
    ArithmeticOperation, CommonRegister, Immediate16, Immediate8, Instruction, JumpCondition,
    ResetVector, RotationShiftOperation,
};
use crate::core::cpu::registers::{Flags, Register16, Register8};
use crate::core::{EventContext, ExecuteContext, ExecutionEvent, HexAddress, HexByte};
use registers::Registers;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CpuError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(u8),
}

/*
TODO
Currently 16-bit reads tick after each 8bit write, but the value in the register isn't updated until after both.
Does that matter? Is that observable?
 */
#[derive(Default, Debug)]
pub struct Cpu {
    registers: Registers,
    interrupt_master_enable: bool,
    schedule_ime: bool,
}

impl Cpu {
    pub fn after_boot_rom() -> Self {
        Self {
            registers: Registers::after_boot_rom(),
            interrupt_master_enable: false,
            schedule_ime: false,
        }
    }

    pub fn get_first_opcode<C: ExecuteContext>(&mut self, ctx: &mut C) -> u8 {
        let opcode = ctx.read(self.registers.read_register16(Register16::PC));
        self.registers.increment_pc();
        // TODO should the clock tick here? If so, forward to Execution?
        opcode
    }

    pub fn decode_execute_fetch<C: ExecuteContext + EventContext>(
        &mut self,
        opcode: u8,
        context: &mut C,
    ) -> Result<u8, CpuError> {
        let mut execution = Execution { cpu: self, context };
        execution.decode_execute_fetch(opcode)
    }
}

impl std::fmt::Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Registers:")?;
        writeln!(f, "{}", self.registers)?;
        if self.interrupt_master_enable {
            writeln!(f, "Interrupts enabled")?;
        } else {
            writeln!(f, "Interrupts disabled")?;
        }
        writeln!(f, "Interrupt enable pending: {:?}", self.schedule_ime)
    }
}

struct Execution<'a, C: ExecuteContext + EventContext> {
    cpu: &'a mut Cpu,
    context: &'a mut C,
}

impl<'a, C: ExecuteContext + EventContext> Execution<'a, C> {
    /*
       Notes:
       * Post-increment PC, always. Current PC is suitable for use/peeking.
       * Clock ticks are coupled to memory reads, and therefore also handled by fetch_next_opcode.
       * Any reads at PC also increment and tick.
    */
    pub fn decode_execute_fetch(&mut self, opcode: u8) -> Result<u8, CpuError> {
        let x = (opcode & 0b11000000) >> 6;
        let y = (opcode & 0b00111000) >> 3;
        let z = opcode & 0b00000111;
        let p = (y & 0b110) >> 1;
        let q = y & 0b1;
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
            3 => self.x_is_3_tree(opcode, y, z, p, q)?,
            _ => return Err(CpuError::InvalidOpcode(opcode)),
        };
        self.context
            .push_event(ExecutionEvent::InstructionExecuted {
                opcode: HexByte(opcode),
                instruction,
                new_pc: HexAddress(self.cpu.registers.read_register16(Register16::PC)),
                registers: self.cpu.registers.clone(),
            });
        Ok(self.read_byte_at_pc())
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

    fn x_is_3_tree(
        &mut self,
        opcode: u8,
        y: u8,
        z: u8,
        p: u8,
        q: u8,
    ) -> Result<Instruction, CpuError> {
        let instruction = match z {
            0 => match y {
                0..=3 => self.ret_cc(JumpCondition::from_u8(y)),
                4 => self.ld_io_imm_a(),
                5 => self.add_sp_d(),
                6 => self.ld_io_a_imm(),
                7 => self.ld_hl_sp_d(),
                _ => unreachable!(),
            },
            1 => match q {
                0 => self.pop(Register16::from_byte_af(p)),
                1 => match p {
                    0 => self.ret(),
                    1 => self.reti(),
                    2 => self.jp_hl(),
                    3 => self.ld_sp_hl(),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            2 => match y {
                0..=3 => {
                    let cc = JumpCondition::from_u8(y);
                    self.jp_cc(cc)
                }
                4 => self.ld_io_c_a(),
                5 => self.ld_inn_a(),
                6 => self.ld_io_a_c(),
                7 => self.ld_a_inn(),
                _ => unreachable!(),
            },
            3 => match y {
                0 => self.jp(),
                1 => self.cb_prefix(),
                2 | 3 | 4 | 5 => return Err(CpuError::InvalidOpcode(opcode)),
                6 => self.di(),
                7 => self.ei(),
                _ => unreachable!(),
            },
            4 => match y {
                0..=3 => self.call_cc(JumpCondition::from_u8(y)),
                4..=7 => return Err(CpuError::InvalidOpcode(opcode)),
                _ => unreachable!(),
            },
            5 => match q {
                0 => self.push(Register16::from_byte_af(p)),
                1 => match p {
                    0 => self.call(),
                    1..=3 => return Err(CpuError::InvalidOpcode(opcode)),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            6 => {
                let op = ArithmeticOperation::from_u8(y);
                self.alu_imm(op)
            }
            7 => self.rst(ResetVector::from_u8(y)),
            _ => unreachable!(),
        };

        Ok(instruction)
    }

    fn cb_prefix(&mut self) -> Instruction {
        let opcode = self.read_byte_at_pc();
        let x = (opcode & 0b11000000) >> 6;
        let y = (opcode & 0b00111000) >> 3;
        let z = opcode & 0b00000111;
        match x {
            0 => self.rotate_shift(
                RotationShiftOperation::from_u8(y),
                CommonRegister::from_u8(z),
            ),
            1 => self.bit(y, CommonRegister::from_u8(z)),
            2 => self.res(y, CommonRegister::from_u8(z)),
            3 => self.set(y, CommonRegister::from_u8(z)),
            _ => unreachable!(),
        }
    }

    fn read_byte_at(&mut self, addr: u16) -> u8 {
        let b = self.context.read(addr);
        self.context.tick();
        b
    }

    fn read_byte_at_pc(&mut self) -> u8 {
        let res = self.read_byte_at(self.cpu.registers.read_register16(Register16::PC));
        self.cpu.registers.increment_pc();
        res
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
        self.context.write(addr, b);
        self.context.tick();
    }

    fn write_word_to(&mut self, addr: u16, w: u16) {
        let lsb = w as u8;
        let msb = (w >> 8) as u8;

        self.write_byte_to(addr, lsb);
        self.write_byte_to(addr.wrapping_add(1), msb);
    }

    fn noop(&self) -> Instruction {
        Instruction::Nop
    }

    fn bit(&mut self, bit: u8, reg: CommonRegister) -> Instruction {
        debug_assert!(bit <= 7);
        let value = self.read_common_register(reg);
        let z = value & (1 << bit) == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.insert(Flags::H);
            f.remove(Flags::N);
        });

        Instruction::BitRegister(bit, reg)
    }

    fn set(&mut self, bit: u8, reg: CommonRegister) -> Instruction {
        debug_assert!(bit <= 7);
        let value = self.read_common_register(reg);
        let res = value | (1 << bit);
        self.write_common_register(reg, res);
        Instruction::SetRegister(bit, reg)
    }

    fn res(&mut self, bit: u8, reg: CommonRegister) -> Instruction {
        debug_assert!(bit <= 7);
        let value = self.read_common_register(reg);
        let res = value & !(1 << bit);
        self.write_common_register(reg, res);
        Instruction::ResRegister(bit, reg)
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
        self.context.tick();
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
            self.context.tick();
            self.cpu.registers.write_register16(
                Register16::PC,
                add_i8_to_u16(ioffset, self.cpu.registers.read_register16(Register16::PC)),
            );
        }
        Instruction::JumpConditionalRelative(cc, Immediate8(offset))
    }

    fn add_hl_rp(&mut self, rp: Register16) -> Instruction {
        let src = self.cpu.registers.read_register16(rp);
        let lsb = src as u8;
        let msb = (src >> 8) as u8;

        let z = self.cpu.registers.flags().contains(Flags::Z);
        let l = self.cpu.registers.read_register8(Register8::L);
        let l_res = self.add_8bit(l, lsb);
        self.cpu.registers.write_register8(Register8::L, l_res);
        self.context.tick();
        let h = self.cpu.registers.read_register8(Register8::H);
        let h_res = self.add_8bit_carry(h, msb);
        self.cpu.registers.write_register8(Register8::H, h_res);
        self.cpu.registers.modify_flags(|f| f.set(Flags::Z, z));

        Instruction::AddHLRegister(rp)
    }

    fn add_8bit(&mut self, a: u8, b: u8) -> u8 {
        let (res, carry) = a.carrying_add(b, false);
        let h = (a & 0x0F) + (b & 0x0F) > 0x0F;
        let z = res == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::C, carry);
            f.set(Flags::H, h);
            f.set(Flags::Z, z);
            f.remove(Flags::N);
        });
        res
    }

    fn add_8bit_carry(&mut self, a: u8, b: u8) -> u8 {
        let carry_in = self.cpu.registers.flags().contains(Flags::C);
        let (res, carry) = a.carrying_add(b, carry_in);
        let h = (a & 0x0F) + (b & 0x0F) + (if carry_in { 1 } else { 0 }) > 0x0F;
        let z = res == 0;
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
        let result = (a as i16) - (b as i16);
        let c = result < 0x00;
        let result = result as u8;
        let z = result == 0;
        let h = ((a & 0x0F) as i8).wrapping_sub((b & 0x0F) as i8) < 0x00;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.set(Flags::C, c);
            f.set(Flags::H, h);
            f.insert(Flags::N);
        });
        result
    }

    fn sub_carry(&mut self, a: u8, b: u8) -> u8 {
        let carry = if self.cpu.registers.flags().contains(Flags::C) {
            1
        } else {
            0
        };
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
            f.remove(Flags::N | Flags::C);
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
    fn ld_r_r(&mut self, target: CommonRegister, source: CommonRegister) -> Instruction {
        debug_assert!(target != CommonRegister::HLIndirect || source != CommonRegister::HLIndirect);
        if target == CommonRegister::Register8(Register8::B)
            && source == CommonRegister::Register8(Register8::B)
        {
            self.context.push_event(ExecutionEvent::DebugTrigger)
        }
        let v = self.read_common_register(source);
        self.write_common_register(target, v);

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
        self.context
            .write(self.cpu.registers.read_register16(rp), res);
        Instruction::LoadIndirectRegisterA(rp)
    }
    fn ld_hlp_a(&mut self) -> Instruction {
        let res = self.cpu.registers.read_register8(Register8::A);
        self.context
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
        self.context
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
        let res = self.context.read(self.cpu.registers.read_register16(rp));
        self.cpu.registers.write_register8(Register8::A, res);
        Instruction::LoadAIndirectRegister(rp)
    }
    fn ld_a_hlp(&mut self) -> Instruction {
        let res = self.read_byte_at(self.cpu.registers.read_register16(Register16::HL));
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
        let res = self.read_byte_at(self.cpu.registers.read_register16(Register16::HL));
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
        self.cpu
            .registers
            .write_register16(rp, self.cpu.registers.read_register16(rp).wrapping_add(1));
        self.context.tick();
        Instruction::IncRegister16(rp)
    }
    fn dec_16(&mut self, rp: Register16) -> Instruction {
        self.cpu
            .registers
            .write_register16(rp, self.cpu.registers.read_register16(rp).wrapping_sub(1));
        self.context.tick();
        Instruction::DecRegister16(rp)
    }
    fn inc(&mut self, reg: CommonRegister) -> Instruction {
        let val = self.read_common_register(reg);
        let res = val.wrapping_add(1);
        let z = res == 0;
        let h = (val & 0x0F) + 1 > 0x0F;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.set(Flags::H, h);
            f.remove(Flags::N);
        });

        self.write_common_register(reg, res);
        Instruction::IncRegister8(reg)
    }
    fn dec(&mut self, reg: CommonRegister) -> Instruction {
        let val = self.read_common_register(reg);
        let res = val.wrapping_sub(1);
        let z = res == 0;
        let h = val & 0xF == 0;
        self.cpu.registers.modify_flags(|f| {
            f.insert(Flags::N);
            f.set(Flags::Z, z);
            f.set(Flags::H, h);
        });

        self.write_common_register(reg, res);
        Instruction::DecRegister8(reg)
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
        let res = res & 0xFE | (if cur_carry { 1 } else { 0 });
        let z = res == 0;
        self.cpu.registers.modify_flags(|f| {
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
        let res = res & 0x7F | (if cur_carry { 0x80 } else { 0 });
        let z = res == 0;
        self.cpu.registers.modify_flags(|f| {
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
    fn sla(&mut self, register: CommonRegister) -> Instruction {
        let a = self.read_common_register(register);
        let c = a & 0x80 > 0;
        let res = a << 1;
        let z = res == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.set(Flags::C, c);
            f.remove(Flags::N | Flags::H);
        });
        self.write_common_register(register, res);

        Instruction::RotateShiftRegister(RotationShiftOperation::Sla, register)
    }
    fn sra(&mut self, register: CommonRegister) -> Instruction {
        let a = self.read_common_register(register);
        let c = a & 0x01 > 0;
        let bit_7 = a & 0x80;
        let res = a >> 1;
        let res = (res & 0x7F) | bit_7;
        let z = res == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.set(Flags::C, c);
            f.remove(Flags::N | Flags::H);
        });
        self.write_common_register(register, res);
        Instruction::RotateShiftRegister(RotationShiftOperation::Sra, register)
    }
    fn srl(&mut self, register: CommonRegister) -> Instruction {
        let a = self.read_common_register(register);
        let c = a & 0x01 > 0;
        let res = a >> 1;
        let res = res & 0x7F;
        let z = res == 0;
        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, z);
            f.set(Flags::C, c);
            f.remove(Flags::N | Flags::H);
        });
        self.write_common_register(register, res);
        Instruction::RotateShiftRegister(RotationShiftOperation::Srl, register)
    }
    fn swap(&mut self, register: CommonRegister) -> Instruction {
        let a = self.read_common_register(register);
        let res = ((a & 0xF0) >> 4) | ((a & 0x0F) << 4);
        self.write_common_register(register, res);

        self.cpu.registers.modify_flags(|f| {
            f.set(Flags::Z, res == 0);
            f.remove(Flags::C | Flags::N | Flags::H);
        });

        Instruction::RotateShiftRegister(RotationShiftOperation::Swap, register)
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

    fn ret(&mut self) -> Instruction {
        self.pop(Register16::PC);
        self.context.tick();

        Instruction::Return
    }

    fn ret_cc(&mut self, cc: JumpCondition) -> Instruction {
        self.context.tick();
        if self.should_jump(cc) {
            self.pop(Register16::PC);
            self.context.tick();
        }

        Instruction::ReturnConditional(cc)
    }

    fn pop(&mut self, register: Register16) -> Instruction {
        let sp = self.cpu.registers.read_register16(Register16::SP);
        let w = self.read_word_at(sp);
        self.cpu
            .registers
            .write_register16(Register16::SP, sp.wrapping_add(2));
        self.cpu.registers.write_register16(register, w);

        Instruction::Pop(register)
    }
    fn push(&mut self, register: Register16) -> Instruction {
        let sp = self.cpu.registers.read_register16(Register16::SP);
        self.context.tick();
        let w = self.cpu.registers.read_register16(register);
        self.write_word_to(sp.wrapping_sub(2), w);
        self.cpu
            .registers
            .write_register16(Register16::SP, sp.wrapping_sub(2));

        Instruction::Push(register)
    }
    fn ld_io_imm_a(&mut self) -> Instruction {
        let val = self.cpu.registers.read_register8(Register8::A);
        let lsb = self.read_byte_at_pc();
        self.write_byte_to(0xFF00 | (lsb as u16), val);

        Instruction::LoadIOIndirectImmediate8A(Immediate8(lsb))
    }
    fn ld_io_a_imm(&mut self) -> Instruction {
        let lsb = self.read_byte_at_pc();
        let val = self.read_byte_at(0xFF00 | (lsb as u16));
        self.cpu.registers.write_register8(Register8::A, val);

        Instruction::LoadIOAIndirectImmediate8(Immediate8(lsb))
    }
    fn add_signed_to_sp(&mut self, imm: u8) -> u16 {
        let sp = self.cpu.registers.read_register16(Register16::SP);

        let lower = sp as u8;
        let res_h = (lower & 0x0F) + (imm & 0x0F);
        let h = res_h > 0x0F;
        let res_lower = (lower as u16) + (imm as u16);
        let c = res_lower > 0xFF;

        let res = add_i8_to_u16(imm as i8, sp);
        self.context.tick();
        self.context.tick();

        self.cpu.registers.modify_flags(|f| {
            f.remove(Flags::Z | Flags::N);
            f.set(Flags::H, h);
            f.set(Flags::C, c);
        });

        res
    }
    fn add_sp_d(&mut self) -> Instruction {
        let imm = self.read_byte_at_pc();
        let res = self.add_signed_to_sp(imm);
        self.cpu.registers.write_register16(Register16::SP, res);
        Instruction::AddSPImmediate(Immediate8(imm))
    }
    fn ld_hl_sp_d(&mut self) -> Instruction {
        let imm = self.read_byte_at_pc();
        let res = self.add_signed_to_sp(imm);
        self.cpu.registers.write_register16(Register16::HL, res);
        Instruction::LoadHLSPImmediate(Immediate8(imm))
    }
    fn reti(&mut self) -> Instruction {
        self.ret();
        self.cpu.interrupt_master_enable = true;

        Instruction::ReturnInterrupt
    }

    fn ld_sp_hl(&mut self) -> Instruction {
        self.cpu.registers.write_register16(
            Register16::SP,
            self.cpu.registers.read_register16(Register16::HL),
        );

        Instruction::LoadSPHL
    }
    fn jp_cc(&mut self, cc: JumpCondition) -> Instruction {
        let imm = self.read_word_at_pc();
        if self.should_jump(cc) {
            self.context.tick();
            self.cpu.registers.write_register16(Register16::PC, imm)
        }
        Instruction::JumpConditionalImmediate(cc, Immediate16(imm))
    }
    fn ld_io_c_a(&mut self) -> Instruction {
        let val = self.cpu.registers.read_register8(Register8::A);
        let addr = 0xFF00 | (self.cpu.registers.read_register8(Register8::C) as u16);
        self.write_byte_to(addr, val);

        Instruction::LoadIOIndirectCA
    }
    fn ld_io_a_c(&mut self) -> Instruction {
        let addr = 0xFF00 | (self.cpu.registers.read_register8(Register8::C) as u16);
        let val = self.read_byte_at(addr);
        self.cpu.registers.write_register8(Register8::A, val);

        Instruction::LoadIOAIndirectC
    }
    fn ld_inn_a(&mut self) -> Instruction {
        let addr = self.read_word_at_pc();
        let val = self.cpu.registers.read_register8(Register8::A);
        self.write_byte_to(addr, val);

        Instruction::LoadIndirectImmediate16A(Immediate16(addr))
    }
    fn ld_a_inn(&mut self) -> Instruction {
        let addr = self.read_word_at_pc();
        let val = self.read_byte_at(addr);
        self.cpu.registers.write_register8(Register8::A, val);

        Instruction::LoadAIndirectImmediate16(Immediate16(addr))
    }
    fn jp(&mut self) -> Instruction {
        let addr = self.read_word_at_pc();
        self.context.tick();
        self.cpu.registers.write_register16(Register16::PC, addr);

        Instruction::JumpImmediate(Immediate16(addr))
    }
    fn jp_hl(&mut self) -> Instruction {
        let addr = self.cpu.registers.read_register16(Register16::HL);
        self.cpu.registers.write_register16(Register16::PC, addr);
        Instruction::JumpHL
    }
    fn di(&mut self) -> Instruction {
        self.cpu.schedule_ime = false;
        self.cpu.interrupt_master_enable = false;

        Instruction::DI
    }
    fn ei(&mut self) -> Instruction {
        // TODO actually enable this on clock tick
        self.cpu.schedule_ime = true;

        Instruction::EI
    }
    fn call(&mut self) -> Instruction {
        let target = self.read_word_at_pc();
        self.context.tick();
        self.push(Register16::PC);
        self.cpu.registers.write_register16(Register16::PC, target);

        Instruction::CallImmediate(Immediate16(target))
    }
    fn call_cc(&mut self, cc: JumpCondition) -> Instruction {
        let target = self.read_word_at_pc();
        if self.should_jump(cc) {
            self.context.tick();
            self.push(Register16::PC);
            self.cpu.registers.write_register16(Register16::PC, target);
        }

        Instruction::CallConditionalImmediate(cc, Immediate16(target))
    }
    fn rst(&mut self, vector: ResetVector) -> Instruction {
        let target = vector.address();
        self.push(Register16::PC);
        self.cpu.registers.write_register16(Register16::PC, target);

        Instruction::Reset(vector)
    }
    fn rotate_shift(&mut self, op: RotationShiftOperation, reg: CommonRegister) -> Instruction {
        match op {
            RotationShiftOperation::Rlc => self.rlc(reg),
            RotationShiftOperation::Rrc => self.rrc(reg),
            RotationShiftOperation::Rl => self.rl(reg),
            RotationShiftOperation::Rr => self.rr(reg),
            RotationShiftOperation::Sla => self.sla(reg),
            RotationShiftOperation::Sra => self.sra(reg),
            RotationShiftOperation::Swap => self.swap(reg),
            RotationShiftOperation::Srl => self.srl(reg),
        }
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
        let mut context = TestContext::default();
        context.mem[1] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(context.instruction.unwrap(), Instruction::Nop);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 1);
    }

    #[test]
    fn ld_inn_sp() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::SP, 0x1234);
        let mut context = TestContext::default();
        context.mem[0] = 0x08;
        context.mem[1] = 0x10;
        context.mem[2] = 0x00;
        context.mem[3] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::LoadIndirectImmediate16SP(Immediate16(0x0010))
        );
        assert_eq!(context.mem[0x0010], 0x34);
        assert_eq!(context.mem[0x0011], 0x12);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 5);
    }

    #[test]
    fn jr_positive() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::PC, 0x1234);
        let mut context = TestContext::default();
        context.mem[0x1234] = 0x18;
        context.mem[0x1235] = 0x05;
        context.mem[0x123B] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::JumpRelative(Immediate8(0x05))
        );
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 3);
    }

    #[test]
    fn jr_negative() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::PC, 0x1234);
        let mut context = TestContext::default();
        context.mem[0x1234] = 0x18;
        context.mem[0x1235] = 0xFD;
        context.mem[0x1233] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::JumpRelative(Immediate8(0xFD))
        );
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 3);
    }

    #[test]
    fn jr_cc_taken() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::PC, 0x1234);
        cpu.registers.modify_flags(|f| f.insert(Flags::Z));
        let mut context = TestContext::default();
        context.mem[0x1234] = 0b00101000;
        context.mem[0x1235] = 0x05;
        context.mem[0x123B] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::JumpConditionalRelative(JumpCondition::Z, Immediate8(0x05))
        );
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 3);
    }

    #[test]
    fn jr_cc_not_taken() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::PC, 0x1234);
        let mut context = TestContext::default();
        context.mem[0x1234] = 0b00101000;
        context.mem[0x1235] = 0x05;
        context.mem[0x1236] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::JumpConditionalRelative(JumpCondition::Z, Immediate8(0x05))
        );
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 2);
    }

    #[test]
    fn add_hl_rp() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::HL, 0xFFFF);
        cpu.registers.write_register16(Register16::BC, 0x0001);
        let mut context = TestContext::default();
        context.mem[0] = 0x09;
        context.mem[1] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::AddHLRegister(Register16::BC)
        );
        assert_eq!(cpu.registers.read_register16(Register16::HL), 0);
        assert_eq!(cpu.registers.flags(), Flags::H | Flags::C);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 2);

        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::HL, 0x0EFF);
        cpu.registers.write_register16(Register16::BC, 0x0001);
        let mut context = TestContext::default();
        context.mem[0] = 0x09;
        context.mem[1] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);
        cpu.registers.modify_flags(|f| f.insert(Flags::Z));

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::AddHLRegister(Register16::BC)
        );
        assert_eq!(cpu.registers.read_register16(Register16::HL), 0x0F00);
        assert_eq!(cpu.registers.flags(), Flags::Z);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 2);
    }

    #[test]
    fn sub_n() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register8(Register8::A, 10);
        cpu.registers.write_register8(Register8::B, 5);
        let mut context = TestContext::default();
        context.mem[0] = 0x90;
        context.mem[1] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::AluRegister(
                ArithmeticOperation::Sub,
                CommonRegister::Register8(Register8::B)
            )
        );
        assert_eq!(cpu.registers.read_register8(Register8::A), 5);
        assert_eq!(cpu.registers.flags(), Flags::N);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 1);
    }

    #[test]
    fn sub_n_carry() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register8(Register8::A, 5);
        cpu.registers.write_register8(Register8::B, 10);
        let mut context = TestContext::default();
        context.mem[0] = 0x90;
        context.mem[1] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::AluRegister(
                ArithmeticOperation::Sub,
                CommonRegister::Register8(Register8::B)
            )
        );
        assert_eq!(cpu.registers.read_register8(Register8::A), 251);
        assert_eq!(cpu.registers.flags(), Flags::N | Flags::C | Flags::H);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 1);
    }

    #[test]
    fn rst() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register8(Register8::A, 10);
        cpu.registers.write_register8(Register8::B, 5);
        let mut context = TestContext::default();
        context.mem[0] = 0xD7;
        context.mem[0x10] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::Reset(ResetVector::Two)
        );
        assert_eq!(cpu.registers.read_register16(Register16::PC), 0x11);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 4);
    }

    #[test]
    fn push_pop() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register8(Register8::B, 0x01);
        cpu.registers.write_register8(Register8::C, 0x02);
        cpu.registers.write_register16(Register16::SP, 0x4000);
        let mut context = TestContext::default();
        context.mem[0] = 0xC5;
        context.mem[1] = 0xD1;
        context.mem[2] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();
        assert_eq!(
            context.instruction.unwrap(),
            Instruction::Push(Register16::BC)
        );
        assert_eq!(context.cycles, 4, "push cycles");
        assert_eq!(context.mem[0x3FFF], 0x01);
        assert_eq!(context.mem[0x3FFE], 0x02);
        assert_eq!(cpu.registers.read_register16(Register16::SP), 0x3FFE);

        context.reset_cycles();
        let next_opcode = cpu.decode_execute_fetch(next_opcode, &mut context).unwrap();
        assert_eq!(
            context.instruction.unwrap(),
            Instruction::Pop(Register16::DE)
        );
        assert_eq!(context.cycles, 3, "pop cycles");
        assert_eq!(cpu.registers.read_register16(Register16::SP), 0x4000);
        assert_eq!(cpu.registers.read_register16(Register16::DE), 0x0102);
        assert_eq!(next_opcode, 0xFF);
    }

    #[test]
    fn add_8bit_carry() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register8(Register8::A, 10);
        cpu.registers.write_register8(Register8::B, 5);
        let mut context = TestContext::default();
        context.mem[0] = 0x88;
        context.mem[1] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::AluRegister(
                ArithmeticOperation::AdcA,
                CommonRegister::Register8(Register8::B)
            )
        );
        assert_eq!(cpu.registers.read_register8(Register8::A), 15);
        assert_eq!(cpu.registers.flags(), Flags::empty());
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 1);
    }

    #[test]
    fn add_8bit_carry_carry_in() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register8(Register8::A, 10);
        cpu.registers.write_register8(Register8::B, 5);
        cpu.registers.modify_flags(|f| f.insert(Flags::C));
        let mut context = TestContext::default();
        context.mem[0] = 0x88;
        context.mem[1] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::AluRegister(
                ArithmeticOperation::AdcA,
                CommonRegister::Register8(Register8::B)
            )
        );
        assert_eq!(cpu.registers.read_register8(Register8::A), 16);
        assert_eq!(cpu.registers.flags(), Flags::H);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 1);
    }

    #[test]
    fn add_hl_bc_1() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::HL, 0x0FFF);
        cpu.registers.write_register16(Register16::BC, 1);
        let mut context = TestContext::default();
        context.mem[0] = 0x09;
        context.mem[1] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::AddHLRegister(Register16::BC)
        );
        assert_eq!(cpu.registers.read_register16(Register16::HL), 0x1000);
        assert_eq!(cpu.registers.flags(), Flags::H);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 2);
    }

    #[test]
    fn add_hl_bc_2() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register16(Register16::HL, 0xFFFF);
        cpu.registers.write_register16(Register16::BC, 1);
        let mut context = TestContext::default();
        context.mem[0] = 0x09;
        context.mem[1] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::AddHLRegister(Register16::BC)
        );
        assert_eq!(cpu.registers.read_register16(Register16::HL), 0x0000);
        assert_eq!(cpu.registers.flags(), Flags::H | Flags::C);
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 2);
    }

    #[test]
    fn swap() {
        let mut cpu = Cpu::default();
        cpu.registers.write_register8(Register8::C, 0x12);
        let mut context = TestContext::default();
        context.mem[0] = 0xCB;
        context.mem[1] = 0x31;
        context.mem[2] = 0xFF;

        let opcode = cpu.get_first_opcode(&mut context);

        let next_opcode = cpu.decode_execute_fetch(opcode, &mut context).unwrap();

        assert_eq!(
            context.instruction.unwrap(),
            Instruction::RotateShiftRegister(
                RotationShiftOperation::Swap,
                CommonRegister::Register8(Register8::C)
            )
        );
        assert_eq!(cpu.registers.read_register8(Register8::C), 0x21);
        assert_eq!(cpu.registers.flags(), Flags::empty());
        assert_eq!(next_opcode, 0xFF);
        assert_eq!(context.cycles, 2);
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
