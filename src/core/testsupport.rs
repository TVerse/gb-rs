use super::FULL_ADDRESS_SPACE;
use crate::core::{Clock, ExecuteContext, ExecutionEvent, MemoryView};

#[derive(Debug)]
pub struct TestMemoryView {
    pub mem: [u8; FULL_ADDRESS_SPACE],
}

impl MemoryView for TestMemoryView {
    fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.mem[addr as usize] = value
    }
}

impl Default for TestMemoryView {
    fn default() -> Self {
        Self {
            mem: [0; FULL_ADDRESS_SPACE],
        }
    }
}

#[derive(Default, Debug)]
pub struct CycleCountingClock {
    pub cycles: usize,
}

impl Clock for CycleCountingClock {
    fn tick(&mut self) {
        self.cycles += 1;
    }
}

#[derive(Default, Debug)]
pub struct NoopContext {}

impl ExecuteContext for NoopContext {
    fn push_event(&mut self, _event: ExecutionEvent) {}
}
