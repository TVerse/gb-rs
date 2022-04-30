use gb_rs::{parse_into_cartridge, ExecutionEvent, GameBoy};
use std::fs;
use std::path::Path;

const MAX_CYCLES: u64 = 300_000_000;

#[test]
fn blargg_instr_timing() {
    let file = Path::new("vendored_test_roms/blargg/instr_timing/instr_timing.gb");
    let rom = fs::read(file).unwrap();
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
