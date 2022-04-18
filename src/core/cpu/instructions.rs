use crate::core::cpu::registers::{Register16, Register8};
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
#[repr(u8)]
pub enum ArithmeticOperation {
    AddA = 0,
    AdcA = 1,
    Sub = 2,
    SbcA = 3,
    And = 4,
    Xor = 5,
    Or = 6,
    Cp = 7,
}

impl ArithmeticOperation {
    pub fn from_u8(b: u8) -> Self {
        assert!(b <= 7);
        unsafe { std::mem::transmute::<_, Self>(b) }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display)]
#[repr(u8)]
pub enum RotationShiftOperation {
    Rlc = 0,
    Rrc = 1,
    Rl = 2,
    Rr = 3,
    Sla = 4,
    Sra = 5,
    Swap = 6,
    Srl = 7,
}

impl RotationShiftOperation {
    pub fn from_u8(b: u8) -> Self {
        assert!(b <= 7);
        unsafe { std::mem::transmute::<_, Self>(b) }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display)]
#[repr(u8)]
pub enum JumpCondition {
    NZ = 0,
    Z = 1,
    NC = 2,
    C = 3,
}

impl JumpCondition {
    pub fn from_u8(b: u8) -> Self {
        assert!(b <= 3);
        unsafe { std::mem::transmute::<_, Self>(b) }
    }
}
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum ResetVector {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
}

impl ResetVector {
    pub fn from_u8(b: u8) -> Self {
        assert!(b <= 7);
        unsafe { std::mem::transmute::<_, Self>(b) }
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
