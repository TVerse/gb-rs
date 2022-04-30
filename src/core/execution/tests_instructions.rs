use super::*;
use crate::core::execution::instructions::Instruction;
use crate::core::interrupt_controller::Interrupt;
use crate::core::{
    ClockContext, EventContext, ExecutionEvent, HandleInterruptContext, InterruptContext,
    MemoryContext, KIB,
};

const FULL_ADDRESS_SPACE: usize = 64 * KIB;

#[derive(Debug)]
pub struct InstructionTestContext {
    pub cycles: usize,
    pub mem: [u8; FULL_ADDRESS_SPACE],
    pub instruction: Option<Instruction>,
}

impl Default for InstructionTestContext {
    fn default() -> Self {
        Self {
            cycles: 0,
            mem: [0; FULL_ADDRESS_SPACE],
            instruction: None,
        }
    }
}

impl InstructionTestContext {
    pub fn reset_cycles(&mut self) {
        self.cycles = 0;
    }
}

impl MemoryContext for InstructionTestContext {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.mem[addr as usize] = value
    }
}

impl EventContext for InstructionTestContext {
    fn push_event(&mut self, event: ExecutionEvent) {
        if let ExecutionEvent::InstructionExecuted { instruction, .. } = event {
            self.instruction = Some(instruction)
        }
    }
}

impl ClockContext for InstructionTestContext {
    fn tick(&mut self) {
        self.cycles += 1;
    }
}

impl InterruptContext for InstructionTestContext {
    fn raise_interrupt(&mut self, _interrupt: Interrupt) {}
}

impl HandleInterruptContext for InstructionTestContext {
    fn unraise_interrupt(&mut self, _interrupt: Interrupt) {}

    fn should_start_interrupt_routine(&self) -> bool {
        false
    }

    fn get_highest_priority_interrupt(&self) -> Option<Interrupt> {
        None
    }

    fn should_cancel_halt(&self) -> bool {
        false
    }

    fn schedule_ime_enable(&mut self) {}

    fn enable_interrupts(&mut self) {}

    fn disable_interrupts(&mut self) {}
}

#[test]
fn noop() {
    let mut cpu = Cpu::default();
    let mut context = InstructionTestContext::default();
    context.mem[1] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(context.instruction.unwrap(), Instruction::Nop);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 4);
}

#[test]
fn ld_inn_sp() {
    let mut cpu = Cpu::default();
    cpu.write_register16(Register16::SP, 0x1234);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0x08;
    context.mem[1] = 0x10;
    context.mem[2] = 0x00;
    context.mem[3] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::LoadIndirectImmediate16SP(Immediate16(0x0010))
    );
    assert_eq!(context.mem[0x0010], 0x34);
    assert_eq!(context.mem[0x0011], 0x12);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 20);
}

#[test]
fn ld_rp_nn() {
    let mut cpu = Cpu::default();
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0x31;
    context.mem[1] = 0x34;
    context.mem[2] = 0x12;
    context.mem[3] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
        .decode_execute_fetch(opcode)
        .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::LoadRegisterImmediate16(Register16::SP, Immediate16(0x1234))
    );
    assert_eq!(cpu.read_register16(Register16::SP), 0x1234);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 12);
}

#[test]
fn jr_positive() {
    let mut cpu = Cpu::default();
    cpu.write_register16(Register16::PC, 0x1234);
    let mut context = InstructionTestContext::default();
    context.mem[0x1234] = 0x18;
    context.mem[0x1235] = 0x05;
    context.mem[0x123B] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::JumpRelative(Immediate8(0x05))
    );
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 12);
}

#[test]
fn jr_negative() {
    let mut cpu = Cpu::default();
    cpu.write_register16(Register16::PC, 0x1234);
    let mut context = InstructionTestContext::default();
    context.mem[0x1234] = 0x18;
    context.mem[0x1235] = 0xFD;
    context.mem[0x1233] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::JumpRelative(Immediate8(0xFD))
    );
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 12);
}

#[test]
fn jr_cc_taken() {
    let mut cpu = Cpu::default();
    cpu.write_register16(Register16::PC, 0x1234);
    cpu.modify_flags(|f| f.insert(Flags::Z));
    let mut context = InstructionTestContext::default();
    context.mem[0x1234] = 0b00101000;
    context.mem[0x1235] = 0x05;
    context.mem[0x123B] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::JumpConditionalRelative(JumpCondition::Z, Immediate8(0x05))
    );
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 12);
}

#[test]
fn jr_cc_not_taken() {
    let mut cpu = Cpu::default();
    cpu.write_register16(Register16::PC, 0x1234);
    let mut context = InstructionTestContext::default();
    context.mem[0x1234] = 0b00101000;
    context.mem[0x1235] = 0x05;
    context.mem[0x1236] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::JumpConditionalRelative(JumpCondition::Z, Immediate8(0x05))
    );
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 8);
}

#[test]
fn add_hl_rp() {
    let mut cpu = Cpu::default();
    cpu.write_register16(Register16::HL, 0xFFFF);
    cpu.write_register16(Register16::BC, 0x0001);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0x09;
    context.mem[1] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::AddHLRegister(Register16::BC)
    );
    assert_eq!(cpu.read_register16(Register16::HL), 0);
    assert_eq!(cpu.flags(), Flags::H | Flags::C);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 8);

    let mut cpu = Cpu::default();
    cpu.write_register16(Register16::HL, 0x0EFF);
    cpu.write_register16(Register16::BC, 0x0001);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0x09;
    context.mem[1] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);
    cpu.modify_flags(|f| f.insert(Flags::Z));

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::AddHLRegister(Register16::BC)
    );
    assert_eq!(cpu.read_register16(Register16::HL), 0x0F00);
    assert_eq!(cpu.flags(), Flags::Z);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 8);
}

#[test]
fn sub_n() {
    let mut cpu = Cpu::default();
    cpu.write_register8(Register8::A, 10);
    cpu.write_register8(Register8::B, 5);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0x90;
    context.mem[1] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::AluRegister(
            ArithmeticOperation::Sub,
            CommonRegister::Register8(Register8::B),
        )
    );
    assert_eq!(cpu.read_register8(Register8::A), 5);
    assert_eq!(cpu.flags(), Flags::N);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 4);
}

#[test]
fn sub_n_carry() {
    let mut cpu = Cpu::default();
    cpu.write_register8(Register8::A, 5);
    cpu.write_register8(Register8::B, 10);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0x90;
    context.mem[1] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::AluRegister(
            ArithmeticOperation::Sub,
            CommonRegister::Register8(Register8::B),
        )
    );
    assert_eq!(cpu.read_register8(Register8::A), 251);
    assert_eq!(cpu.flags(), Flags::N | Flags::C | Flags::H);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 4);
}

#[test]
fn rst() {
    let mut cpu = Cpu::default();
    cpu.write_register8(Register8::A, 10);
    cpu.write_register8(Register8::B, 5);
    cpu.write_register16(Register16::SP, 0x1002);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0xD7;
    context.mem[0x10] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::Reset(ResetVector::Two)
    );
    assert_eq!(cpu.read_register16(Register16::SP), 0x1000);
    assert_eq!(context.mem[0x1000], 0x01, "0x1000");
    assert_eq!(context.mem[0x1001], 0x00, "0x1001");
    assert_eq!(cpu.read_register16(Register16::PC), 0x11);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 16);
}

#[test]
fn push_pop() {
    let mut cpu = Cpu::default();
    cpu.write_register8(Register8::B, 0x01);
    cpu.write_register8(Register8::C, 0x02);
    cpu.write_register16(Register16::SP, 0x4000);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0xC5;
    context.mem[1] = 0xD1;
    context.mem[2] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();
    assert_eq!(
        context.instruction.unwrap(),
        Instruction::Push(Register16::BC)
    );
    assert_eq!(context.cycles, 16, "push cycles");
    assert_eq!(context.mem[0x3FFF], 0x01);
    assert_eq!(context.mem[0x3FFE], 0x02);
    assert_eq!(cpu.read_register16(Register16::SP), 0x3FFE);

    let next_opcode = match next_operation {
        NextOperation::Opcode(opcode) => opcode,
        NextOperation::StartInterruptRoutine => panic!(),
    };

    context.reset_cycles();
    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(next_opcode)
    .unwrap();
    assert_eq!(
        context.instruction.unwrap(),
        Instruction::Pop(Register16::DE)
    );
    assert_eq!(context.cycles, 12, "pop cycles");
    assert_eq!(cpu.read_register16(Register16::SP), 0x4000);
    assert_eq!(cpu.read_register16(Register16::DE), 0x0102);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
}

#[test]
fn add_8bit_carry() {
    let mut cpu = Cpu::default();
    cpu.write_register8(Register8::A, 10);
    cpu.write_register8(Register8::B, 5);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0x88;
    context.mem[1] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::AluRegister(
            ArithmeticOperation::AdcA,
            CommonRegister::Register8(Register8::B),
        )
    );
    assert_eq!(cpu.read_register8(Register8::A), 15);
    assert_eq!(cpu.flags(), Flags::empty());
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 4);
}

#[test]
fn add_8bit_carry_carry_in() {
    let mut cpu = Cpu::default();
    cpu.write_register8(Register8::A, 10);
    cpu.write_register8(Register8::B, 5);
    cpu.modify_flags(|f| f.insert(Flags::C));
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0x88;
    context.mem[1] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::AluRegister(
            ArithmeticOperation::AdcA,
            CommonRegister::Register8(Register8::B),
        )
    );
    assert_eq!(cpu.read_register8(Register8::A), 16);
    assert_eq!(cpu.flags(), Flags::H);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 4);
}

#[test]
fn add_hl_bc_1() {
    let mut cpu = Cpu::default();
    cpu.write_register16(Register16::HL, 0x0FFF);
    cpu.write_register16(Register16::BC, 1);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0x09;
    context.mem[1] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::AddHLRegister(Register16::BC)
    );
    assert_eq!(cpu.read_register16(Register16::HL), 0x1000);
    assert_eq!(cpu.flags(), Flags::H);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 8);
}

#[test]
fn add_hl_bc_2() {
    let mut cpu = Cpu::default();
    cpu.write_register16(Register16::HL, 0xFFFF);
    cpu.write_register16(Register16::BC, 1);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0x09;
    context.mem[1] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::AddHLRegister(Register16::BC)
    );
    assert_eq!(cpu.read_register16(Register16::HL), 0x0000);
    assert_eq!(cpu.flags(), Flags::H | Flags::C);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 8);
}

#[test]
fn swap() {
    let mut cpu = Cpu::default();
    cpu.write_register8(Register8::C, 0x12);
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0xCB;
    context.mem[1] = 0x31;
    context.mem[2] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::RotateShiftRegister(
            RotationShiftOperation::Swap,
            CommonRegister::Register8(Register8::C),
        )
    );
    assert_eq!(cpu.read_register8(Register8::C), 0x21);
    assert_eq!(cpu.flags(), Flags::empty());
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 8);
}

#[test]
fn di() {
    let mut cpu = Cpu::default();
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0xF3;
    context.mem[1] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(context.instruction.unwrap(), Instruction::DI,);
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 4);
}

#[test]
fn call() {
    let mut cpu = Cpu::default();
    let mut context = InstructionTestContext::default();
    context.mem[0] = 0xCD;
    context.mem[1] = 0x34;
    context.mem[2] = 0x12;
    context.mem[0x1234] = 0xFF;

    let opcode = get_first_opcode(&mut cpu, &mut context);

    let next_operation = Execution {
        cpu: &mut cpu,
        context: &mut context,
    }
    .decode_execute_fetch(opcode)
    .unwrap();

    assert_eq!(
        context.instruction.unwrap(),
        Instruction::CallImmediate(Immediate16(0x1234)),
    );
    assert_eq!(next_operation, NextOperation::Opcode(0xFF));
    assert_eq!(context.cycles, 24);
}

#[test]
fn add_i8_to_u16_test() {
    let a: i8 = 127;
    let b: u16 = 127;
    assert_eq!(add_i8_to_u16(a, b), 254);

    let a: i8 = -50;
    let b: u16 = 5050;
    assert_eq!(add_i8_to_u16(a, b), 5000);
}
