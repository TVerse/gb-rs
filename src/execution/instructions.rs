use crate::components::cpu::{Register16, Register8};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CommonRegister {
    Register8(Register8),
    HLIndirect,
}

impl CommonRegister {
    pub(in crate::execution) fn from_lowest_3_bits(byte: u8) -> CommonRegister {
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
    LoadRegisterImmediate8(CommonRegister, u8),
    LoadAIndirectRegister(Register16),
    LoadAIndirectImmediate16(u16),
    LoadIndirectRegisterA(Register16),
    LoadIndirectImmediate16A(u16),
    LoadIOAIndirectImmediate8(u8),
    LoadIOIndirectImmediate8A(u8),
    LoadIOIndirectCA,
    LoadIOAIndirectC,
    LoadAIncrementHLIndirect,
    LoadIncrementHLIndirectA,
    LoadADecrementHLIndirect,
    LoadDecrementHLAIndirect,
    // 16bit loads
    LoadRegisterImmediate16(Register16, u16),
    LoadIndirectImmediate16SP(u16),
    LoadSPHL,
    Push(Register16),
    Pop(Register16),
    // 8 bit arithmetic/logic
    AddRegister(CommonRegister),
    AddImmediate8(u8),
    AddCarryRegister(CommonRegister),
    AddCarryImmediate8(u8),
    SubRegister(CommonRegister),
    SubImmediate8(u8),
    SubCarryRegister(CommonRegister),
    SubCarryImmediate8(u8),
    AndRegister8(CommonRegister),
    AndImmediate8(u8),
    XorRegister(CommonRegister),
    XorImmediate8(u8),
    OrRegister(CommonRegister),
    OrImmediate8(u8),
    CompareRegister(CommonRegister),
    CompareImmediate8(u8),
    IncRegister8(CommonRegister),
    DecRegister8(CommonRegister),
    DecimalAdjust,
    Complement,
    // 16 bit arithmetic/logic
    AddHLRegister(Register16),
    IncRegister16(Register16),
    DecRegister16(Register16),
    AddSPImmediate(i8),
    LoadHLSPImmediate(i8),
    // Rotate/shift A
    RotateALeft,
    RotateALeftThroughCarry,
    RotateARight,
    RotateARightThroughCarry,
    // CB prefix
    RotateLeftRegister(CommonRegister),
    RotateLeftThroughCarryRegister(CommonRegister),
    RotateRightRegister(CommonRegister),
    RotateRightThroughCarryRegister(CommonRegister),
    ShiftLeftRegister(CommonRegister),
    ShiftRightArithmeticRegister(CommonRegister),
    ShiftRightLogicalRegister(CommonRegister),
    SwapRegister(CommonRegister),
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
    JumpImmediate(u16),
    JumpHL,
    JumpConditionalImmediate(JumpCondition, u16),
    JumpRelative(i8),
    JumpConditionalRelative(JumpCondition, i8),
    CallImmediate(u16),
    CallConditionalImmediate(JumpCondition, u16),
    Return,
    ReturnConditional(JumpCondition),
    ReturnInterrupt,
    Reset(ResetVector),
}

impl Instruction {
    pub fn bytes(&self) -> u16 {
        match self {
            Instruction::LoadRegisterRegister(_, _) => 1,
            Instruction::LoadRegisterImmediate8(_, _) => 2,
            Instruction::LoadAIndirectRegister(_) => 1,
            Instruction::LoadAIndirectImmediate16(_) => 3,
            Instruction::LoadIndirectRegisterA(_) => 1,
            Instruction::LoadIndirectImmediate16A(_) => 3,
            Instruction::LoadIOAIndirectImmediate8(_) => 2,
            Instruction::LoadIOIndirectImmediate8A(_) => 2,
            Instruction::LoadIOIndirectCA => 1,
            Instruction::LoadIOAIndirectC => 1,
            Instruction::LoadAIncrementHLIndirect => 1,
            Instruction::LoadIncrementHLIndirectA => 1,
            Instruction::LoadADecrementHLIndirect => 1,
            Instruction::LoadDecrementHLAIndirect => 1,
            Instruction::LoadRegisterImmediate16(_, _) => 3,
            Instruction::LoadIndirectImmediate16SP(_) => 3,
            Instruction::LoadSPHL => 1,
            Instruction::Push(_) => 1,
            Instruction::Pop(_) => 1,
            Instruction::AddRegister(_) => 1,
            Instruction::AddImmediate8(_) => 2,
            Instruction::AddCarryRegister(_) => 1,
            Instruction::AddCarryImmediate8(_) => 2,
            Instruction::SubRegister(_) => 1,
            Instruction::SubImmediate8(_) => 2,
            Instruction::SubCarryRegister(_) => 1,
            Instruction::SubCarryImmediate8(_) => 2,
            Instruction::AndRegister8(_) => 1,
            Instruction::AndImmediate8(_) => 2,
            Instruction::XorRegister(_) => 1,
            Instruction::XorImmediate8(_) => 2,
            Instruction::OrRegister(_) => 1,
            Instruction::OrImmediate8(_) => 2,
            Instruction::CompareRegister(_) => 1,
            Instruction::CompareImmediate8(_) => 2,
            Instruction::IncRegister8(_) => 1,
            Instruction::DecRegister8(_) => 1,
            Instruction::DecimalAdjust => 1,
            Instruction::Complement => 1,
            Instruction::AddHLRegister(_) => 1,
            Instruction::IncRegister16(_) => 1,
            Instruction::DecRegister16(_) => 1,
            Instruction::AddSPImmediate(_) => 2,
            Instruction::LoadHLSPImmediate(_) => 2,
            Instruction::RotateALeft => 1,
            Instruction::RotateALeftThroughCarry => 1,
            Instruction::RotateARight => 1,
            Instruction::RotateARightThroughCarry => 1,
            Instruction::RotateLeftRegister(_) => 2,
            Instruction::RotateLeftThroughCarryRegister(_) => 2,
            Instruction::RotateRightRegister(_) => 2,
            Instruction::RotateRightThroughCarryRegister(_) => 2,
            Instruction::ShiftLeftRegister(_) => 2,
            Instruction::ShiftRightArithmeticRegister(_) => 2,
            Instruction::ShiftRightLogicalRegister(_) => 2,
            Instruction::SwapRegister(_) => 2,
            Instruction::BitRegister(_, _) => 2,
            Instruction::SetRegister(_, _) => 2,
            Instruction::ResRegister(_, _) => 2,
            Instruction::Ccf => 1,
            Instruction::Scf => 1,
            Instruction::Nop => 1,
            Instruction::Halt => 1,
            Instruction::Stop => 2,
            Instruction::DI => 1,
            Instruction::EI => 1,
            Instruction::JumpImmediate(_) => 3,
            Instruction::JumpHL => 1,
            Instruction::JumpConditionalImmediate(_, _) => 3,
            Instruction::JumpRelative(_) => 2,
            Instruction::JumpConditionalRelative(_, _) => 2,
            Instruction::CallImmediate(_) => 3,
            Instruction::CallConditionalImmediate(_, _) => 3,
            Instruction::Return => 1,
            Instruction::ReturnConditional(_) => 1,
            Instruction::ReturnInterrupt => 1,
            Instruction::Reset(_) => 1,
        }
    }

    pub fn cycles(&self) -> usize {
        match self {
            Instruction::LoadRegisterRegister(t, s) => {
                Self::cycles_hl(t, 1, 2).max(Self::cycles_hl(s, 1, 2))
            }
            Instruction::LoadRegisterImmediate8(t, _) => Self::cycles_hl(t, 2, 3),
            Instruction::LoadAIndirectRegister(_) => 2,
            Instruction::LoadAIndirectImmediate16(_) => 4,
            Instruction::LoadIndirectRegisterA(_) => 2,
            Instruction::LoadIndirectImmediate16A(_) => 4,
            Instruction::LoadIOAIndirectImmediate8(_) => 3,
            Instruction::LoadIOIndirectImmediate8A(_) => 3,
            Instruction::LoadIOIndirectCA => 2,
            Instruction::LoadIOAIndirectC => 2,
            Instruction::LoadAIncrementHLIndirect => 2,
            Instruction::LoadIncrementHLIndirectA => 2,
            Instruction::LoadADecrementHLIndirect => 2,
            Instruction::LoadDecrementHLAIndirect => 2,
            Instruction::LoadRegisterImmediate16(_, _) => 3,
            Instruction::LoadIndirectImmediate16SP(_) => 5,
            Instruction::LoadSPHL => 2,
            Instruction::Push(_) => 4,
            Instruction::Pop(_) => 3,
            Instruction::AddRegister(r) => Self::cycles_hl(r, 1, 2),
            Instruction::AddImmediate8(_) => 2,
            Instruction::AddCarryRegister(r) => Self::cycles_hl(r, 1, 2),
            Instruction::AddCarryImmediate8(_) => 2,
            Instruction::SubRegister(r) => Self::cycles_hl(r, 1, 2),
            Instruction::SubImmediate8(_) => 2,
            Instruction::SubCarryRegister(r) => Self::cycles_hl(r, 1, 2),
            Instruction::SubCarryImmediate8(_) => 1,
            Instruction::AndRegister8(r) => Self::cycles_hl(r, 1, 2),
            Instruction::AndImmediate8(_) => 1,
            Instruction::XorRegister(r) => Self::cycles_hl(r, 1, 2),
            Instruction::XorImmediate8(_) => 1,
            Instruction::OrRegister(r) => Self::cycles_hl(r, 1, 2),
            Instruction::OrImmediate8(_) => 1,
            Instruction::CompareRegister(r) => Self::cycles_hl(r, 1, 2),
            Instruction::CompareImmediate8(_) => 1,
            Instruction::IncRegister8(r) => Self::cycles_hl(r, 1, 3),
            Instruction::DecRegister8(r) => Self::cycles_hl(r, 1, 3),
            Instruction::DecimalAdjust => 1,
            Instruction::Complement => 1,
            Instruction::AddHLRegister(_) => 2,
            Instruction::IncRegister16(_) => 2,
            Instruction::DecRegister16(_) => 2,
            Instruction::AddSPImmediate(_) => 4,
            Instruction::LoadHLSPImmediate(_) => 3,
            Instruction::RotateALeft => 1,
            Instruction::RotateALeftThroughCarry => 1,
            Instruction::RotateARight => 1,
            Instruction::RotateARightThroughCarry => 1,
            Instruction::RotateLeftRegister(r) => Self::cycles_hl(r, 2, 4),
            Instruction::RotateLeftThroughCarryRegister(r) => Self::cycles_hl(r, 2, 4),
            Instruction::RotateRightRegister(r) => Self::cycles_hl(r, 2, 4),
            Instruction::RotateRightThroughCarryRegister(r) => Self::cycles_hl(r, 2, 4),
            Instruction::ShiftLeftRegister(r) => Self::cycles_hl(r, 2, 4),
            Instruction::ShiftRightArithmeticRegister(r) => Self::cycles_hl(r, 2, 4),
            Instruction::ShiftRightLogicalRegister(r) => Self::cycles_hl(r, 2, 4),
            Instruction::SwapRegister(r) => Self::cycles_hl(r, 2, 4),
            Instruction::BitRegister(_, r) => Self::cycles_hl(r, 2, 4),
            Instruction::SetRegister(_, r) => Self::cycles_hl(r, 2, 4),
            Instruction::ResRegister(_, r) => Self::cycles_hl(r, 2, 4),
            Instruction::Ccf => 1,
            Instruction::Scf => 1,
            Instruction::Nop => 1,
            Instruction::Halt => 1,
            Instruction::Stop => 1,
            Instruction::DI => 1,
            Instruction::EI => 1,
            Instruction::JumpImmediate(_) => 4,
            Instruction::JumpHL => 1,
            Instruction::JumpConditionalImmediate(_, _) => 4,
            Instruction::JumpRelative(_) => 3,
            Instruction::JumpConditionalRelative(_, _) => 3,
            Instruction::CallImmediate(_) => 6,
            Instruction::CallConditionalImmediate(_, _) => 6,
            Instruction::Return => 4,
            Instruction::ReturnConditional(_) => 5,
            Instruction::ReturnInterrupt => 4,
            Instruction::Reset(_) => 4,
        }
    }

    pub fn cycles_branch_not_taken(&self) -> usize {
        match self {
            Instruction::JumpConditionalImmediate(_, _) => 3,
            Instruction::JumpConditionalRelative(_, _) => 2,
            Instruction::CallConditionalImmediate(_, _) => 3,
            Instruction::ReturnConditional(_) => 2,
            _ => {
                unreachable!(
                    "Called cycles_branch_not_taken for an instruction that wasn't conditional"
                )
            }
        }
    }

    fn cycles_hl(reg: &CommonRegister, normal: usize, hl: usize) -> usize {
        if reg == &CommonRegister::HLIndirect {
            hl
        } else {
            normal
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum JumpCondition {
    NZ,
    Z,
    NC,
    C,
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
    pub(in crate::execution) fn address(&self) -> u16 {
        match self {
            ResetVector::Zero => 0x0000,
            ResetVector::One => 0x0008,
            ResetVector::Two => 0x0010,
            ResetVector::Three => 0x0018,
            ResetVector::Four => 0x0020,
            ResetVector::Five => 0x0028,
            ResetVector::Six => 0x0030,
            ResetVector::Seven => 0x0038,
        }
    }
}
