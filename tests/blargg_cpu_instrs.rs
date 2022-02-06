use gb_rs::{GameBoy, RomOnlyCartridge};
use std::fs;
use std::path::Path;
use paste::paste;

macro_rules! blargg_test {
    ($n:expr) => {
        paste! {
           #[test]
           fn [<blarg_cpu_instr_ $n>]() {
                let rom = load_rom($n);
                execute_test(rom);
            }
        }
    }
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
    let base_path = Path::new("gb-test-roms/cpu_instrs/individual");
    let file = fs::read_dir(base_path).unwrap().map(|d| d.unwrap()).find(|d| d.file_name().to_str().unwrap().starts_with(prefix)).unwrap().path();

    fs::read(file).unwrap()
}

fn execute_test(rom: Vec<u8>) {
    let exact_rom = TryFrom::try_from(rom).unwrap();
    let cartridge = RomOnlyCartridge::new(exact_rom);

    let mut gb = GameBoy::new(Box::new(cartridge));

    let mut serial_out: Vec<_> = "Serial out: ".bytes().collect();

    loop {
        gb.step().unwrap();
        gb.get_serial().unwrap().into_iter().for_each(|c| {
            serial_out.push(c);
        });
        if serial_out.ends_with("Failed".as_bytes()) {
            panic!("{}", String::from_utf8_lossy(&serial_out))
        }
    }
}