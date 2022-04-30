use crate::{Buffer, ColorId, Cpu, Instruction, Interrupt, Mode};

pub struct HexWord(pub u16);

impl std::fmt::Debug for HexWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for HexWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#06x}", self.0)
    }
}

pub struct HexByte(pub u8);

impl std::fmt::Debug for HexByte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for HexByte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#04x}", self.0)
    }
}

#[derive(Debug)]
pub enum ExecutionEvent {
    MemoryRead {
        address: HexWord,
        value: HexByte,
    },
    MemoryWritten {
        address: HexWord,
        value: HexByte,
    },
    ReadFromNonMappedAddress(HexWord),
    WriteToNonMappedAddress(HexWord),
    InstructionExecuted {
        opcode: HexByte,
        instruction: Instruction,
        new_pc: HexWord,
        cpu: Cpu,
    },
    InterruptRaised(Interrupt),
    InterruptRoutineStarted,
    InterruptRoutineFinished(Interrupt),
    SerialOut(HexByte),
    FrameReady(Box<Buffer>),
    PpuModeSwitch {
        mode: Mode,
        x: u16,
        y: u8,
    },
    PpuPixelPushed(u8, u8, ColorId),
    Halted,
    DebugTrigger,
}

impl std::fmt::Display for ExecutionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFromNonMappedAddress(a) => {
                write!(f, "ReadFromNonMappedAddress({})", a)
            }
            Self::WriteToNonMappedAddress(a) => {
                write!(f, "WriteToNonMappedAddress({})", a)
            }
            Self::InstructionExecuted {
                opcode,
                instruction,
                new_pc,
                cpu,
            } => {
                writeln!(f, "InstructionExecuted")?;
                writeln!(f, "Opcode: {}", opcode)?;
                writeln!(f, "{}", instruction)?;
                writeln!(f, "PC after instruction: {}", new_pc)?;
                writeln!(f, "Registers:")?;
                write!(f, "{}", cpu)
            }
            Self::DebugTrigger => write!(f, "DebugTrigger"),
            Self::MemoryRead { address, value } => {
                write!(f, "MemoryRead{{address: {}, value: {}}}", address, value)
            }
            Self::MemoryWritten { address, value } => {
                write!(f, "MemoryWritten{{address: {}, value: {}}}", address, value)
            }
            Self::InterruptRoutineStarted => write!(f, "InterruptRoutineStarted"),
            Self::InterruptRoutineFinished(interrupt) => {
                write!(f, "InterruptRoutineFinished({})", interrupt)
            }
            Self::InterruptRaised(interrupt) => {
                write!(f, "InterruptRaised({})", interrupt)
            }
            Self::SerialOut(b) => write!(f, "SerialOut({})", b),
            Self::FrameReady(_) => write!(f, "FrameReady"),
            Self::PpuModeSwitch { mode, x, y } => {
                write!(f, "PpuModeSwitch{{mode: {:?}, x: {}, y: {}}}", mode, x, y)
            }
            Self::PpuPixelPushed(x, y, c) => write!(f, "PpuPixelPushed({}, {}, {:?})", x, y, c),
            Self::Halted => write!(f, "Halted"),
        }
    }
}
