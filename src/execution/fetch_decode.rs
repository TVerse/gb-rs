use crate::components::bus::Bus;
use crate::components::cpu::{Cpu, Register16, Register8};
use crate::execution::instructions::{CommonRegister, Instruction, JumpCondition, ResetVector};
use crate::{GameBoyError, RawResult};
use Instruction::*;

pub struct DecodeContext {
    pub instruction: Instruction,
    pub pc: u16,
    pub three_bytes_at_pc: [Option<u8>; 3],
    pub three_bytes_before_pc: [Option<u8>; 3],
}

pub fn fetch_and_decode(cpu: &Cpu, bus: &dyn Bus) -> RawResult<DecodeContext> {
    let decoder = Decoder { cpu, bus };
    decoder.decode()
}

struct Decoder<'a> {
    cpu: &'a Cpu,
    bus: &'a dyn Bus,
}

impl<'a> Decoder<'a> {
    fn decode(&self) -> RawResult<DecodeContext> {
        let pc = self.cpu.get_register16(Register16::PC);
        let three_bytes_at_pc = [
            self.bus.read_byte(pc).ok(),
            self.bus.read_byte(pc.wrapping_add(1)).ok(),
            self.bus.read_byte(pc.wrapping_add(2)).ok(),
        ];

        let three_bytes_before_pc = [
            self.bus.read_byte(pc.wrapping_sub(3)).ok(),
            self.bus.read_byte(pc.wrapping_sub(2)).ok(),
            self.bus.read_byte(pc.wrapping_sub(1)).ok(),
        ];

        let instruction = self.decode_pc_override(pc)?;
        Ok(DecodeContext {
            instruction,
            pc,
            three_bytes_at_pc,
            three_bytes_before_pc,
        })
    }

    fn decode_pc_override(&self, start_pc: u16) -> RawResult<Instruction> {
        let opcode = self.bus.read_byte(start_pc)?;

        let immediate_8 = self.read_immediate_8(start_pc);
        let immediate_16 = self.read_immediate_16(start_pc);

        let instruction = match opcode {
            0x00 => Nop,
            0x01 => LoadRegisterImmediate16(Register16::BC, immediate_16?),
            0x02 => LoadIndirectRegisterA(Register16::BC),
            0x03 => IncRegister16(Register16::BC),
            0x04 => IncRegister8(CommonRegister::Register8(Register8::B)),
            0x05 => DecRegister8(CommonRegister::Register8(Register8::B)),
            0x06 => LoadRegisterImmediate8(CommonRegister::Register8(Register8::B), immediate_8?),
            0x07 => RotateALeft,
            0x08 => LoadIndirectImmediate16SP(immediate_16?),
            0x09 => AddHLRegister(Register16::BC),
            0x0A => LoadAIndirectRegister(Register16::BC),
            0x0B => DecRegister16(Register16::BC),
            0x0C => IncRegister8(CommonRegister::Register8(Register8::C)),
            0x0D => DecRegister8(CommonRegister::Register8(Register8::C)),
            0x0E => LoadRegisterImmediate8(CommonRegister::Register8(Register8::C), immediate_8?),
            0x0F => RotateARight,
            0x10 => self.stop(start_pc, immediate_8?)?,
            0x11 => LoadRegisterImmediate16(Register16::DE, immediate_16?),
            0x12 => LoadIndirectRegisterA(Register16::DE),
            0x13 => IncRegister16(Register16::DE),
            0x14 => IncRegister8(CommonRegister::Register8(Register8::D)),
            0x15 => DecRegister8(CommonRegister::Register8(Register8::D)),
            0x16 => LoadRegisterImmediate8(CommonRegister::Register8(Register8::D), immediate_8?),
            0x17 => RotateALeftThroughCarry,
            0x18 => JumpRelative(immediate_8? as i8),
            0x19 => AddHLRegister(Register16::DE),
            0x1A => LoadAIndirectRegister(Register16::DE),
            0x1B => DecRegister16(Register16::DE),
            0x1C => IncRegister8(CommonRegister::Register8(Register8::E)),
            0x1D => DecRegister8(CommonRegister::Register8(Register8::E)),
            0x1E => LoadRegisterImmediate8(CommonRegister::Register8(Register8::E), immediate_8?),
            0x1F => RotateARightThroughCarry,
            0x20 => JumpConditionalRelative(JumpCondition::NZ, immediate_8? as i8),
            0x21 => LoadRegisterImmediate16(Register16::HL, immediate_16?),
            0x22 => LoadIncrementHLIndirectA,
            0x23 => IncRegister16(Register16::HL),
            0x24 => IncRegister8(CommonRegister::Register8(Register8::H)),
            0x25 => DecRegister8(CommonRegister::Register8(Register8::H)),
            0x26 => LoadRegisterImmediate8(CommonRegister::Register8(Register8::H), immediate_8?),
            0x27 => DecimalAdjust,
            0x28 => JumpConditionalRelative(JumpCondition::Z, immediate_8? as i8),
            0x29 => AddHLRegister(Register16::HL),
            0x2A => LoadAIncrementHLIndirect,
            0x2B => DecRegister16(Register16::HL),
            0x2C => IncRegister8(CommonRegister::Register8(Register8::L)),
            0x2D => DecRegister8(CommonRegister::Register8(Register8::L)),
            0x2E => LoadRegisterImmediate8(CommonRegister::Register8(Register8::L), immediate_8?),
            0x2F => Complement,
            0x30 => JumpConditionalRelative(JumpCondition::NC, immediate_8? as i8),
            0x31 => LoadRegisterImmediate16(Register16::SP, immediate_16?),
            0x32 => LoadDecrementHLAIndirect,
            0x33 => IncRegister16(Register16::SP),
            0x34 => IncRegister8(CommonRegister::HLIndirect),
            0x35 => DecRegister8(CommonRegister::HLIndirect),
            0x36 => LoadRegisterImmediate8(CommonRegister::HLIndirect, immediate_8?),
            0x37 => Scf,
            0x38 => JumpConditionalRelative(JumpCondition::C, immediate_8? as i8),
            0x39 => AddHLRegister(Register16::SP),
            0x3A => LoadADecrementHLIndirect,
            0x3B => DecRegister16(Register16::SP),
            0x3C => IncRegister8(CommonRegister::Register8(Register8::A)),
            0x3D => DecRegister8(CommonRegister::Register8(Register8::A)),
            0x3E => LoadRegisterImmediate8(CommonRegister::Register8(Register8::A), immediate_8?),
            0x3F => Ccf,
            0x40..=0x7F => {
                if opcode == 0x76 {
                    Halt
                } else {
                    let source = CommonRegister::from_lowest_3_bits(opcode & 0x07);
                    let target = CommonRegister::from_lowest_3_bits((opcode >> 3) & 0x07);
                    LoadRegisterRegister(target, source)
                }
            }
            0x80..=0x87 => AddRegister(CommonRegister::from_lowest_3_bits(opcode & 0x07)),
            0x88..=0x8F => AddCarryRegister(CommonRegister::from_lowest_3_bits(opcode & 0x07)),
            0x90..=0x97 => SubRegister(CommonRegister::from_lowest_3_bits(opcode & 0x07)),
            0x98..=0x9F => SubCarryRegister(CommonRegister::from_lowest_3_bits(opcode & 0x07)),
            0xA0..=0xA7 => AndRegister8(CommonRegister::from_lowest_3_bits(opcode & 0x07)),
            0xA8..=0xAF => XorRegister(CommonRegister::from_lowest_3_bits(opcode & 0x07)),
            0xB0..=0xB7 => OrRegister(CommonRegister::from_lowest_3_bits(opcode & 0x07)),
            0xB8..=0xBF => CompareRegister(CommonRegister::from_lowest_3_bits(opcode & 0x07)),
            0xC0 => ReturnConditional(JumpCondition::NZ),
            0xC1 => Pop(Register16::BC),
            0xC2 => JumpConditionalImmediate(JumpCondition::NZ, immediate_16?),
            0xC3 => JumpImmediate(immediate_16?),
            0xC4 => CallConditionalImmediate(JumpCondition::NZ, immediate_16?),
            0xC5 => Push(Register16::BC),
            0xC6 => AddImmediate8(immediate_8?),
            0xC7 => Reset(ResetVector::Zero),
            0xC8 => ReturnConditional(JumpCondition::Z),
            0xC9 => Return,
            0xCA => JumpConditionalImmediate(JumpCondition::Z, immediate_16?),
            0xCB => self.cb_prefix(immediate_8?),
            0xCC => CallConditionalImmediate(JumpCondition::Z, immediate_16?),
            0xCD => CallImmediate(immediate_16?),
            0xCE => AddCarryImmediate8(immediate_8?),
            0xCF => Reset(ResetVector::One),
            0xD0 => ReturnConditional(JumpCondition::NC),
            0xD1 => Pop(Register16::DE),
            0xD2 => JumpConditionalImmediate(JumpCondition::NC, immediate_16?),
            0xD4 => CallConditionalImmediate(JumpCondition::NC, immediate_16?),
            0xD5 => Push(Register16::DE),
            0xD6 => SubImmediate8(immediate_8?),
            0xD7 => Reset(ResetVector::Two),
            0xD8 => ReturnConditional(JumpCondition::C),
            0xD9 => ReturnInterrupt,
            0xDA => JumpConditionalImmediate(JumpCondition::C, immediate_16?),
            0xDC => CallConditionalImmediate(JumpCondition::C, immediate_16?),
            0xDE => SubCarryImmediate8(immediate_8?),
            0xDF => Reset(ResetVector::Three),
            0xE0 => LoadIOIndirectImmediate8A(immediate_8?),
            0xE1 => Pop(Register16::HL),
            0xE2 => LoadIOIndirectCA,
            0xE5 => Push(Register16::HL),
            0xE6 => AndImmediate8(immediate_8?),
            0xE7 => Reset(ResetVector::Four),
            0xE8 => AddSPImmediate(immediate_8? as i8),
            0xE9 => JumpHL,
            0xEA => LoadIndirectImmediate16A(immediate_16?),
            0xEE => XorImmediate8(immediate_8?),
            0xEF => Reset(ResetVector::Five),
            0xF0 => LoadIOAIndirectImmediate8(immediate_8?),
            0xF1 => Pop(Register16::AF),
            0xF2 => LoadIOAIndirectC,
            0xF3 => DI,
            0xF5 => Push(Register16::AF),
            0xF6 => OrImmediate8(immediate_8?),
            0xF7 => Reset(ResetVector::Six),
            0xF8 => LoadHLSPImmediate(immediate_8? as i8),
            0xF9 => LoadSPHL,
            0xFA => LoadAIndirectImmediate16(immediate_16?),
            0xFB => EI,
            0xFE => CompareImmediate8(immediate_8?),
            0xFF => Reset(ResetVector::Seven),
            0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB | 0xEC | 0xED | 0xF4 | 0xFC | 0xFD => {
                return Err(GameBoyError::InvalidOpcode {
                    opcode,
                    pc: start_pc,
                });
            }
        };

        Ok(instruction)
    }

    fn cb_prefix(&self, opcode: u8) -> Instruction {
        let register = CommonRegister::from_lowest_3_bits(opcode & 0x07);
        let bit = (opcode >> 3) & 0x07;
        match opcode {
            0x00..=0x07 => RotateLeftRegister(register),
            0x08..=0x0F => RotateRightRegister(register),
            0x10..=0x17 => RotateLeftThroughCarryRegister(register),
            0x18..=0x1F => RotateRightThroughCarryRegister(register),
            0x20..=0x27 => ShiftLeftRegister(register),
            0x28..=0x2F => ShiftRightArithmeticRegister(register),
            0x30..=0x37 => SwapRegister(register),
            0x38..=0x3F => ShiftRightLogicalRegister(register),
            0x40..=0x7F => BitRegister(bit, register),
            0x80..=0xBF => ResRegister(bit, register),
            0xC0..=0xFF => SetRegister(bit, register),
        }
    }

    fn stop(&self, start_pc: u16, opcode: u8) -> RawResult<Instruction> {
        if opcode == 0x00 {
            Ok(Stop)
        } else {
            Err(GameBoyError::InvalidOpcode {
                opcode,
                pc: start_pc,
            })
        }
    }

    fn read_immediate_8(&self, start_pc: u16) -> RawResult<u8> {
        self.bus.read_byte(start_pc.wrapping_add(1))
    }

    fn read_immediate_16(&self, start_pc: u16) -> RawResult<u16> {
        self.bus.read_word(start_pc.wrapping_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bus::FlatBus;

    #[test]
    fn cb_40() {
        let cpu = Cpu::zeroed();
        let bus = FlatBus {
            mem: vec![0xCB, 0x40],
        };
        let instr = fetch_and_decode(&cpu, &bus).unwrap().instruction;
        assert_eq!(
            instr,
            BitRegister(0, CommonRegister::Register8(Register8::B))
        )
    }
}
