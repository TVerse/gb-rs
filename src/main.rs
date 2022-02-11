use gb_rs::{parse_into_cartridge, GameBoy};
use simplelog::*;
use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;

fn main() {
    let default_path: String = "gb-test-roms/cpu_instrs/individual/09-op r,r.gb".to_owned();
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
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

    loop {
        match gb.step() {
            Ok(res) => {
                if let Some(serial) = res.serial_byte {
                    serial_out.push(serial);
                }
                if serial_out.ends_with("Failed".as_bytes())
                    || serial_out.ends_with("Passed".as_bytes())
                    || serial_out.ends_with("Done".as_bytes())
                {
                    log::info!("Serial out: {}", String::from_utf8_lossy(&serial_out));
                    break;
                }
                if in_step {
                    log::info!("Context:\n{}", res.execution_context);
                    log::info!("Cpu: {}", gb.cpu);
                    std::io::stdin().read_line(&mut String::new()).unwrap();
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
