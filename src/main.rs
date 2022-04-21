use simplelog::*;
use std::fs::File;
use std::path::Path;
use std::{env, fs};
use std::error::Error;

use gb_rs::{parse_into_cartridge, ExecutionEvent, GameBoy, Instruction, RotationShiftOperation, CommonRegister, Register8};

fn main() -> Result<(), Box<dyn Error>> {
    let default_path: String = "gb-test-roms/cpu_instrs/individual/06-ld r,r.gb".to_owned();
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create("gb_rs.log").unwrap(),
        ),
    ])
    .unwrap();

    let args: Vec<String> = env::args().collect();

    let path = args.get(1).unwrap_or(&default_path);
    log::info!("Loading from path: {}", path);

    let cartridge = parse_into_cartridge(load_rom(path));

    let mut gb = GameBoy::new(cartridge);
    let mut serial_out = Vec::new();

    let mut in_step = false;

    loop {
        let res = gb.execute_instruction();
        if res.is_err() {
            gb.dump("crashdump");
        }
        res?;
        log::trace!("Events:");
        for e in gb.take_events() {
            if !in_step {
                match &e {
                    ExecutionEvent::InstructionExecuted {
                        new_pc,
                        registers: _,
                        instruction,
                        ..
                    } if new_pc.0 == 0xCC3E => {
                        log::info!("Stepping...");
                        // in_step = true;
                    }
                    ExecutionEvent::DebugTrigger => {
                        log::info!("Debug trigger!");
                        // in_step = true;
                    }
                    _ => {}
                }
            }

            if in_step {
                log::info!("{}", &e)
            }
            log::trace!("{}", &e);
        }
        if let Some(serial) = gb.get_serial_out() {
            // in_step = true;
            serial_out.push(serial);
            let serial = String::from_utf8_lossy(&serial_out);
            log::info!("Got serial:\n{}", serial);
            if serial.contains("Failed") {
                gb.dump("failure_dump");
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
