use crate::core::cpu::instructions::Instruction;
use crate::core::{ExecuteContext, ExecutionEvent, KIB};

const FULL_ADDRESS_SPACE: usize = 64 * KIB;

#[derive(Debug)]
pub struct TestContext {
    pub cycles: usize,
    pub mem: [u8; FULL_ADDRESS_SPACE],
    pub instruction: Option<Instruction>,
}

impl Default for TestContext {
    fn default() -> Self {
        Self {
            cycles: 0,
            mem: [0; FULL_ADDRESS_SPACE],
            instruction: None,
        }
    }
}

impl ExecuteContext for TestContext {
    fn push_event(&mut self, event: ExecutionEvent) {
        if let ExecutionEvent::InstructionExecuted { instruction, .. } = event {
            self.instruction = Some(instruction)
        }
    }

    fn tick(&mut self) {
        self.cycles += 1;
    }

    fn read(&mut self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.mem[addr as usize] = value
    }
}
