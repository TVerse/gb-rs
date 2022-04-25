use gb_rs::{parse_into_cartridge, GameBoy};
use paste::paste;
use std::fs;
use std::path::Path;

macro_rules! blargg_test {
    ($n:expr) => {
        paste! {
            #[test]
            fn [<blarg_cpu_instr_ $n>]() {
                let _ = env_logger::builder()
                    // Include all events in tests
                    .filter_level(log::LevelFilter::max())
                    // Ensure events are captured by `cargo test`
                    .is_test(true)
                    // Ignore errors initializing the logger if tests race to configure it
                    .try_init();
                let rom = load_rom($n);
                execute_test(rom);
            }
        }
    };
}

blargg_test!("01");
blargg_test!("02");
blargg_test!("03");
blargg_test!("04");
blargg_test!("05");
blargg_test!("06");
blargg_test!("07");
blargg_test!("08");
blargg_test!("09");
blargg_test!("10");
blargg_test!("11");

const MAX_CYCLES: u64 = 30_000_000;

fn load_rom(prefix: &str) -> Vec<u8> {
    let base_path = Path::new("gb-test-roms/cpu_instrs/individual");
    let file = fs::read_dir(base_path)
        .unwrap()
        .map(|d| d.unwrap())
        .find(|d| d.file_name().to_str().unwrap().starts_with(prefix))
        .unwrap()
        .path();

    fs::read(file).unwrap()
}

fn execute_test(rom: Vec<u8>) {
    let cartridge = parse_into_cartridge(rom);

    let mut gb = GameBoy::new(cartridge);

    let mut serial_out: Vec<_> = Vec::with_capacity(256);

    loop {
        if gb.get_elapsed_cycles() > MAX_CYCLES {
            let take = serial_out.len().min(100);
            panic!(
                "Test went over step limit! Got partial serial ({} characters): {}",
                take,
                String::from_utf8_lossy(&serial_out[0..take])
            )
        }
        gb.execute_operation().unwrap();
        if let Some(serial) = gb.get_serial_out() {
            serial_out.push(serial);
        }
        if serial_out.ends_with("Failed".as_bytes()) {
            panic!("{}", String::from_utf8_lossy(&serial_out))
        }
        if serial_out.ends_with("Passed".as_bytes()) {
            break;
        }
    }
}
