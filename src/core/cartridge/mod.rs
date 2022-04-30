mod mbc1;
mod rom_only;

use crate::core::cartridge::mbc1::Mbc1Cartridge;
use crate::core::cartridge::rom_only::RomOnlyCartridge;

use crate::core::Addressable;
use std::fmt::Debug;

pub trait Cartridge: Addressable + Debug {}

pub fn parse_into_cartridge(rom: Vec<u8>) -> Box<dyn Cartridge> {
    let header = RawCartridgeHeader {
        nintendo_logo: rom[0x0104..=0x0133].try_into().unwrap(),
        title: rom[0x0134..=0x0143].try_into().unwrap(),
        new_licensee_code: rom[0x0144..=0x0145].try_into().unwrap(),
        sgb_flag: rom[0x0146],
        cartridge_type: rom[0x0147],
        rom_size: rom[0x0148],
        ram_size: rom[0x0149],
        destination_code: rom[0x014A],
        old_licensee_code: rom[0x014B],
        mask_rom_version: rom[0x014C],
        header_checksum: rom[0x01D],
        global_checksum: rom[0x014E..=0x014F].try_into().unwrap(),
    };
    log::info!("Raw header: {:?}", &header);

    // TODO errors, rom_size/ram_size
    match header.cartridge_type {
        0x00 => Box::new(RomOnlyCartridge::new(rom.try_into().unwrap())),
        0x01 => Box::new(Mbc1Cartridge::new(rom)),
        _ => panic!(
            "Unused or unsupported cartridge type {}",
            header.cartridge_type
        ),
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct RawCartridgeHeader {
    nintendo_logo: [u8; 48],
    title: [u8; 16],
    new_licensee_code: [u8; 2],
    sgb_flag: u8,
    cartridge_type: u8,
    rom_size: u8,
    ram_size: u8,
    destination_code: u8,
    old_licensee_code: u8,
    mask_rom_version: u8,
    header_checksum: u8,
    global_checksum: [u8; 2],
}
