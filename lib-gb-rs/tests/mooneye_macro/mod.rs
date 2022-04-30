use std::fs;

use lib_gb_rs::{parse_into_cartridge, ExecutionEvent, GameBoy, Register8};

const MAX_CYCLES: u64 = 10_000_000;

#[macro_export]
macro_rules! mooneye_tests {
    ($name_prefix:expr, $base_path:expr) => {};
    ($name_prefix:expr, $base_path:expr, $test_name:expr) => {
        paste::paste! {
            #[test]
            fn [<$name_prefix _ $test_name>]() {
                mooneye_macro::test_rom(&format!("vendored_test_roms/mts-20211031-2031-86d1acf/{}{}.gb", $base_path, $test_name))
            }
        }
    };
    ($name_prefix:expr, $base_path:expr, $test_name:expr, $($rest:expr),+) => {
        mooneye_tests!($name_prefix, $base_path, $test_name, $($rest),+,);
    };
    ($name_prefix:expr, $base_path:expr, $test_name:expr, $($rest:expr),+,) => {
        mooneye_tests!($name_prefix, $base_path, $test_name);

        mooneye_tests!($name_prefix, $base_path, $($rest),*);
    };
}

pub fn test_rom(rom: &str) {
    let rom = fs::read(rom).unwrap();
    let cartridge = parse_into_cartridge(rom);

    let mut gb = GameBoy::new(cartridge);

    loop {
        if gb.get_elapsed_cycles() > MAX_CYCLES {
            panic!("Test went over step limit!",)
        }
        let (events, res) = gb.execute_operation();
        res.unwrap();
        for e in events {
            if let ExecutionEvent::DebugTrigger = e {
                let b = gb.cpu().read_register8(Register8::B);
                let c = gb.cpu().read_register8(Register8::C);
                let d = gb.cpu().read_register8(Register8::D);
                let e = gb.cpu().read_register8(Register8::E);
                let h = gb.cpu().read_register8(Register8::H);
                let l = gb.cpu().read_register8(Register8::L);
                assert_eq!([b, c, d, e, h, l], [3, 5, 8, 13, 21, 34]);
                return;
            }
        }
    }
}
