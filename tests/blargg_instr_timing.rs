use gb_rs::{parse_into_cartridge, ExecutionEvent, GameBoy};
use std::fs;
use std::path::Path;

const MAX_CYCLES: u64 = 100_000_000;

fn load_rom() -> Vec<u8> {
    let file = Path::new("gb-test-roms/instr_timing/instr_timing.gb");
    fs::read(file).unwrap()
}

#[test]
#[ignore]
fn blargg_instr_timing() {
    let rom = load_rom();
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
        for e in gb.take_events() {
            match e {
                ExecutionEvent::SerialOut(b) => serial_out.push(b.0),
                _ => {}
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
