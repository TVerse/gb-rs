use gb_rs::{GameBoy, GameBoyError, RomOnlyCartridge};
use simplelog::*;
use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;

fn main() {
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

    let rom = load_rom(path);
    let rom = TryFrom::try_from(rom).unwrap();
    let cartridge = RomOnlyCartridge::new(rom);

    let mut gb = GameBoy::new(Box::new(cartridge));

    let mut serial_out: Vec<_> = "Serial out: ".bytes().collect();

    loop {
        match step(&mut gb, &mut serial_out) {
            Ok(counter) => {
                if counter % 10000000 == 0 {
                    log::info!("Writing dump");
                    gb.dump("dump");
                    log::info!("Cpu: {}", gb.cpu());
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

fn step(gb: &mut GameBoy, serial_out: &mut Vec<u8>) -> Result<u64, GameBoyError> {
    let counter = gb.step()?;
    gb.get_serial()?.into_iter().for_each(|c| {
        serial_out.push(c);
        log::info!("{}", String::from_utf8_lossy(&serial_out));
    });
    Ok(counter)
}

fn load_rom<P: AsRef<Path>>(path: P) -> Vec<u8> {
    fs::read(path).unwrap()
}
