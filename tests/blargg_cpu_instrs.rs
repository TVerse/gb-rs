use gb_rs::{GameBoy, RomOnlyCartridge};
use std::fs;
use std::path::Path;

#[test]
fn blargg_cpu_6() {
    let rom = load_rom("06-ld r,r.gb");
    let exact_rom = TryFrom::try_from(rom).unwrap();
    let cartridge = RomOnlyCartridge::new(exact_rom);

    let mut gb = GameBoy::new(Box::new(cartridge));

    let mut serial_out: Vec<_> = "Serial out: ".bytes().collect();

    loop {
        gb.step().unwrap();
        gb.get_serial().unwrap().into_iter().for_each(|c| {
            serial_out.push(c);
            println!("{}", String::from_utf8_lossy(&serial_out));
        });
    }
}

fn load_rom<P: AsRef<Path>>(path: P) -> Vec<u8> {
    let base_path = Path::new("gb-test-roms/cpu_instrs/individual");
    let mut full_path = base_path.to_path_buf();
    full_path.push(path);

    fs::read(full_path).unwrap()
}
