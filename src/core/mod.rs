mod cpu;
#[cfg(test)]
mod testsupport;

const KIB: usize = 1024;
const FULL_ADDRESS_SPACE: usize = 64 * KIB;

trait MemoryView {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

trait Clock {
    fn tick(&mut self);
}

trait ExecuteContext {
    fn push_event(&mut self, event: ExecutionEvent);
}

pub enum ExecutionEvent {
    SerialByteReceived(u8),
}
