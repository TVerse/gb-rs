use std::fs;
use std::path::Path;

use lib_gb_rs::{parse_into_cartridge, ExecutionEvent, GameBoy};
use paste::paste;

const MAX_CYCLES: u64 = 100_000_000;

#[test]
fn make_intellij_see_this_as_test_file() {}

macro_rules! blargg_test {
    ($n:expr) => {
        paste! {
            #[test]
            fn [<blargg_cpu_instr_ $n>]() {
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

fn load_rom(prefix: &str) -> Vec<u8> {
    let base_path = Path::new("vendored_test_roms/blargg/cpu_instrs/individual");
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
        let (events, res) = gb.execute_operation();
        res.unwrap();
        for e in events {
            if let ExecutionEvent::SerialOut(b) = e {
                serial_out.push(b.0)
            }
        }

        if serial_out.ends_with("Failed".as_bytes()) {
            panic!("{}", String::from_utf8_lossy(&serial_out))
        }
        if serial_out.ends_with("Passed".as_bytes()) {
            break;
        }
    }
}
