use gb_rs::{parse_into_cartridge, GameBoy, StepType};
use simplelog::*;
use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;

fn main() {
    let default_path: String = "gb-test-roms/cpu_instrs/individual/02-interrupts.gb".to_owned();
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Trace,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Trace,
            Config::default(),
            File::create("gb_rs.log").unwrap(),
        ),
    ])
    .unwrap();

    let args: Vec<String> = env::args().collect();

    let path = args.get(1).unwrap_or(&default_path);
    log::info!("Loading from path: {}", path);

    let rom = load_rom(path);
    let cartridge = parse_into_cartridge(rom);

    let mut gb = GameBoy::new(cartridge);

    let mut serial_out: Vec<_> = "".bytes().collect();

    let mut in_step = false;

    let mut steps = 0;

    loop {
        match gb.step() {
            Ok(res) => {
                steps += 1;
                match res.step_type {
                    StepType::InstructionExecuted(execution_context) => {
                        // let breakpoints: &[u16] = &[0xC460, 0xC486, 0xC78D];
                        let breakpoints: &[u16] = &[0xc316, 0xc321, 0xc32f, 0xc33d, 0xc350];
                        if breakpoints.contains(&execution_context.pc) {
                            in_step = true;
                        }
                        if steps > 10_000_000 {
                            in_step = true;
                        }
                        match execution_context.instruction {
                            // Instruction::LoadIOIndirectImmediate8A(Immediate8(0x07)) => in_step = true,
                            // Instruction::LoadIOAIndirectImmediate8(Immediate8(0x07)) => in_step = true,
                            // Instruction::DI => in_step = true,
                            _ => {}
                        }
                        if in_step {
                            log::info!("Context:\n{}", execution_context);
                        }
                    }
                    StepType::InterruptStarted => in_step = true,
                    StepType::Halted => {}
                }
                if let Some(serial) = res.serial_byte {
                    serial_out.push(serial);
                }
                if serial_out.ends_with("Failed".as_bytes())
                    || serial_out.ends_with("Passed".as_bytes())
                    || serial_out.ends_with("Done".as_bytes())
                {
                    log::info!("Serial out: {}", String::from_utf8_lossy(&serial_out));
                    gb.dump("dump_done");
                    break;
                }
                if in_step {
                    log::info!("Cpu:\n{}", gb.cpu);
                    loop {
                        let mut read = String::with_capacity(1);
                        std::io::stdin().read_line(&mut read).unwrap();
                        if read.contains('c') {
                            in_step = false;
                            break;
                        } else if read.contains('d') {
                            gb.dump("dump");
                        } else if read.contains('q') {
                            return;
                        } else {
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Error: {}", e);
                gb.dump("dump");
                std::process::exit(1)
            }
        }
    }
}

fn load_rom<P: AsRef<Path>>(path: P) -> Vec<u8> {
    fs::read(path).unwrap()
}
