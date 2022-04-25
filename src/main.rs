#![allow(unused_variables)]
#![allow(unused_imports)]
use clap::ArgEnum;
use clap::Parser;
use simplelog::*;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::path::Path;

use gb_rs::{
    parse_into_cartridge, ArithmeticOperation, CommonRegister, ExecutionEvent, GameBoy, Immediate8,
    Instruction, Register16, Register8, RotationShiftOperation,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, arg_enum, default_value_t = LogLevel::Info)]
    file_log_level: LogLevel,

    #[clap(short, long, arg_enum, default_value_t = LogLevel::Info)]
    console_log_level: LogLevel,

    #[clap(default_value_t = String::from("gb-test-roms/instr_timing/instr_timing.gb"))]
    path: String,
}

#[derive(ArgEnum, Copy, Clone, Debug)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_level_filter(&self) -> LevelFilter {
        match self {
            LogLevel::Trace => LevelFilter::Trace,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Error => LevelFilter::Error,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Args::parse();

    CombinedLogger::init(vec![
        TermLogger::new(
            args.console_log_level.as_level_filter(),
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            args.file_log_level.as_level_filter(),
            Config::default(),
            File::create("gb_rs.log").unwrap(),
        ),
    ])
    .unwrap();

    let path = args.path;

    let cartridge = parse_into_cartridge(load_rom(path));

    let mut gb = GameBoy::new(cartridge);
    let mut serial_out = Vec::new();

    let mut in_step = false;

    loop {
        let res = gb.execute_operation();
        if res.is_err() {
            gb.dump("crashdump");
        }
        res?;
        log::trace!("Events:");
        for ref e in gb.take_events() {
            if let ExecutionEvent::SerialOut(b) = e {
                serial_out.push(b.0);
                let serial = String::from_utf8_lossy(&serial_out);
                log::info!("Got serial:\n{}", serial);
                if serial.contains("Failed") {
                    gb.dump("failure_dump");
                    return Err("Failed".to_string().into());
                }
            }
            if !in_step {
                match e {
                    ExecutionEvent::InstructionExecuted {
                        new_pc,
                        cpu,
                        instruction,
                        ..
                    } if *instruction
                        == Instruction::AluImmediate(ArithmeticOperation::Sub, Immediate8(10)) =>
                    {
                        log::info!("Instruction breakpoint...");
                        // in_step = true;
                    }
                    ExecutionEvent::DebugTrigger => {
                        log::info!("Debug trigger!");
                        // in_step = true;
                    }
                    ExecutionEvent::MemoryWritten { address, value }
                        if address.0 == 0xFF01 || address.0 == 0xFF02 =>
                    {
                        log::info!("Write breakpoint...");
                        // in_step = true;
                    }
                    ExecutionEvent::Halted => {
                        log::info!("Halted...");
                        // in_step = true;
                    }
                    ExecutionEvent::SerialOut(_) => {
                        log::info!("Serial...");
                        // in_step = true;
                    }
                    _ => {}
                }
            }

            if in_step {
                log::info!("{}", &e)
            } else {
                log::trace!("{}", &e);
            }
        }
        if in_step {
            loop {
                let mut read = String::with_capacity(1);
                std::io::stdin().read_line(&mut read).unwrap();
                if read.contains('c') {
                    in_step = false;
                    break;
                } else if read.contains('d') {
                    gb.dump("dump");
                } else if read.contains('q') {
                    return Ok(());
                } else if read.contains('s') {
                    log::info!("{}", gb)
                } else {
                    break;
                }
            }
        }
    }
}

fn load_rom<P: AsRef<Path>>(path: P) -> Vec<u8> {
    fs::read(path).unwrap()
}
