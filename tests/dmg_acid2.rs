#![feature(slice_flatten)]
use gb_rs::{parse_into_cartridge, Color, ExecutionEvent, GameBoy};
use image::io::Reader;
use std::fs;
use std::path::Path;

const MAX_CYCLES: u64 = 5_000_000;

#[test]
#[ignore]
fn dmg_acid2() {
    let reference = Reader::open("vendored_test_roms/dmg-acid2/reference-dmg.png")
        .unwrap()
        .decode()
        .unwrap();
    let reference = reference.as_bytes();

    let file = Path::new("vendored_test_roms/dmg-acid2/dmg-acid2.gb");
    let rom = fs::read(file).unwrap();
    let cartridge = parse_into_cartridge(rom);

    let mut gb = GameBoy::new(cartridge);

    let mut buffer = None;

    loop {
        if gb.get_elapsed_cycles() > MAX_CYCLES {
            panic!("Test went over step limit!",)
        }
        let (events, res) = gb.execute_operation();
        res.unwrap();
        for e in events {
            match e {
                ExecutionEvent::FrameReady(buf) => buffer = Some(buf),
                ExecutionEvent::DebugTrigger => {
                    let buf = buffer.unwrap();
                    let colors = buf.flatten();
                    let bytes: Vec<u8> = colors
                        .iter()
                        .flat_map(|c| match c {
                            Color::White => [0xFF],
                            Color::LightGrey => [0xAA],
                            Color::DarkGrey => [0x55],
                            Color::Black => [0x00],
                        })
                        .collect();
                    assert_eq!(reference, &bytes);
                    return;
                }
                _ => {}
            }
        }
    }
}
