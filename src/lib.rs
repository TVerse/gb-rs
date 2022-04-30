#![feature(bigint_helper_methods)]
#![feature(slice_flatten)]

mod core;

pub use self::core::cpu::{Flags, Register16, Register8};
pub use self::core::execution::instructions::{
    ArithmeticOperation, CommonRegister, Immediate16, Immediate8, Instruction, ResetVector,
    RotationShiftOperation,
};
pub use crate::core::cartridge::parse_into_cartridge;
pub use crate::core::ppu::Buffer;
pub use crate::core::ppu::Color;
pub use crate::core::ppu::ColorId;
pub use crate::core::ExecutionEvent;
pub use crate::core::GameBoy;
