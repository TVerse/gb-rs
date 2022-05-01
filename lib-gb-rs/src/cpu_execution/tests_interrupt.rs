use super::*;
use crate::components::interrupt_controller::Interrupt;
use crate::KIB;

const FULL_ADDRESS_SPACE: usize = 64 * KIB;

#[derive(Debug)]
struct InterruptTestContext {
    mem: [u8; FULL_ADDRESS_SPACE],
    interrupt_unraised: Option<Interrupt>,
    should_start_routine: bool,
    master_enable: bool,
    cycles: u32,
}

impl Default for InterruptTestContext {
    fn default() -> Self {
        Self {
            mem: [0; FULL_ADDRESS_SPACE],
            interrupt_unraised: None,
            should_start_routine: true,
            master_enable: true,
            cycles: 0,
        }
    }
}

impl MemoryContext for InterruptTestContext {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.mem[addr as usize] = value;
    }
}

impl EventContext for InterruptTestContext {
    fn push_event(&mut self, _event: ExecutionEvent) {}
}

impl ClockContext for InterruptTestContext {
    fn tick(&mut self) {
        self.cycles += 1;
    }
}

impl HandleInterruptContext for InterruptTestContext {
    fn unraise_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_unraised = Some(interrupt);
    }

    fn should_start_interrupt_routine(&self) -> bool {
        self.should_start_routine
    }

    fn get_highest_priority_interrupt(&self) -> Option<Interrupt> {
        Some(Interrupt::Timer)
    }

    fn should_cancel_halt(&self) -> bool {
        false
    }

    fn schedule_ime_enable(&mut self) {
        panic!()
    }

    fn enable_interrupts(&mut self) {
        self.master_enable = true;
    }

    fn disable_interrupts(&mut self) {
        self.master_enable = false;
    }
}

#[test]
fn interrupt_trigger() {
    let mut cpu = Cpu::default();
    let mut context = InterruptTestContext::default();
    let next = handle_next(&mut cpu, NextOperation::Opcode(0x00), &mut context).unwrap();
    assert_eq!(next, NextOperation::StartInterruptRoutine);
}

#[test]
fn routine_start() {
    let mut cpu = Cpu::default();
    let mut context = InterruptTestContext::default();
    let next = handle_next(&mut cpu, NextOperation::StartInterruptRoutine, &mut context).unwrap();
    assert_eq!(next, NextOperation::Opcode(0x00), "op");
    assert_eq!(cpu.read_register16(Register16::PC), 0x51, "address");
    assert_eq!(context.cycles, 20, "cycles");
    assert_eq!(
        context.interrupt_unraised.unwrap(),
        Interrupt::Timer,
        "unraise"
    );
}
