use crate::core::registers::{Register16, Register8};
use strum_macros::Display;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Immediate8(pub u8);

impl std::fmt::Display for Immediate8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#04x}", self.0)
    }
}

impl std::fmt::Debug for Immediate8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Immediate16(pub u16);

impl std::fmt::Display for Immediate16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#06x}", self.0)
    }
}

impl std::fmt::Debug for Immediate16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CommonRegister {
    Register8(Register8),
    HLIndirect,
}

impl CommonRegister {
    pub fn from_u8(byte: u8) -> CommonRegister {
        debug_assert!(byte < 0x08);
        match byte {
            0 => CommonRegister::Register8(Register8::B),
            1 => CommonRegister::Register8(Register8::C),
            2 => CommonRegister::Register8(Register8::D),
            3 => CommonRegister::Register8(Register8::E),
            4 => CommonRegister::Register8(Register8::H),
            5 => CommonRegister::Register8(Register8::L),
            6 => CommonRegister::HLIndirect,
            7 => CommonRegister::Register8(Register8::A),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Instruction {
    // 8bit loads
    LoadRegisterRegister(CommonRegister, CommonRegister),
    LoadRegisterImmediate8(CommonRegister, Immediate8),
    LoadAIndirectRegister(Register16),
    LoadAIndirectImmediate16(Immediate16),
    LoadIndirectRegisterA(Register16),
    LoadIndirectImmediate16A(Immediate16),
    LoadIOAIndirectImmediate8(Immediate8),
    LoadIOIndirectImmediate8A(Immediate8),
    LoadIOIndirectCA,
    LoadIOAIndirectC,
    LoadAIncrementHLIndirect,
    LoadIncrementHLIndirectA,
    LoadADecrementHLIndirect,
    LoadDecrementHLIndirectA,
    // 16bit loads
    LoadRegisterImmediate16(Register16, Immediate16),
    LoadIndirectImmediate16SP(Immediate16),
    LoadSPHL,
    Push(Register16),
    Pop(Register16),
    // 8 bit arithmetic/logic
    AluRegister(ArithmeticOperation, CommonRegister),
    AluImmediate(ArithmeticOperation, Immediate8),
    IncRegister8(CommonRegister),
    DecRegister8(CommonRegister),
    DecimalAdjust,
    Complement,
    // 16 bit arithmetic/logic
    AddHLRegister(Register16),
    IncRegister16(Register16),
    DecRegister16(Register16),
    AddSPImmediate(Immediate8),
    LoadHLSPImmediate(Immediate8),
    // Rotate/shift A
    RotateALeft,
    RotateALeftThroughCarry,
    RotateARight,
    RotateARightThroughCarry,
    // CB prefix
    RotateShiftRegister(RotationShiftOperation, CommonRegister),
    BitRegister(u8, CommonRegister),
    SetRegister(u8, CommonRegister),
    ResRegister(u8, CommonRegister),
    // Control
    Ccf,
    Scf,
    Nop,
    Halt,
    Stop,
    DI,
    EI,
    // Jump
    JumpImmediate(Immediate16),
    JumpHL,
    JumpConditionalImmediate(JumpCondition, Immediate16),
    JumpRelative(Immediate8),
    JumpConditionalRelative(JumpCondition, Immediate8),
    CallImmediate(Immediate16),
    CallConditionalImmediate(JumpCondition, Immediate16),
    Return,
    ReturnConditional(JumpCondition),
    ReturnInterrupt,
    Reset(ResetVector),
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display)]
pub enum ArithmeticOperation {
    AddA,
    AdcA,
    Sub,
    SbcA,
    And,
    Xor,
    Or,
    Cp,
}

impl ArithmeticOperation {
    pub fn from_u8(b: u8) -> Self {
        assert!(b <= 7);
        match b {
            0 => Self::AddA,
            1 => Self::AdcA,
            2 => Self::Sub,
            3 => Self::SbcA,
            4 => Self::And,
            5 => Self::Xor,
            6 => Self::Or,
            7 => Self::Cp,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display)]
pub enum RotationShiftOperation {
    Rlc,
    Rrc,
    Rl,
    Rr,
    Sla,
    Sra,
    Swap,
    Srl,
}

impl RotationShiftOperation {
    pub fn from_u8(b: u8) -> Self {
        assert!(b <= 7);
        match b {
            0 => Self::Rlc,
            1 => Self::Rrc,
            2 => Self::Rl,
            3 => Self::Rr,
            4 => Self::Sla,
            5 => Self::Sra,
            6 => Self::Swap,
            7 => Self::Srl,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display)]
pub enum JumpCondition {
    NZ,
    Z,
    NC,
    C,
}

impl JumpCondition {
    pub fn from_u8(b: u8) -> Self {
        assert!(b <= 3);
        match b {
            0 => Self::NZ,
            1 => Self::Z,
            2 => Self::NC,
            3 => Self::C,
            _ => unreachable!(),
        }
    }
}
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ResetVector {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
}

impl ResetVector {
    pub fn from_u8(b: u8) -> Self {
        assert!(b <= 7);
        match b {
            0 => Self::Zero,
            1 => Self::One,
            2 => Self::Two,
            3 => Self::Three,
            4 => Self::Four,
            5 => Self::Five,
            6 => Self::Six,
            7 => Self::Seven,
            _ => unreachable!(),
        }
    }

    pub fn address(&self) -> u16 {
        ((*self as u8) * 8) as u16
    }
}

impl std::fmt::Display for ResetVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}
