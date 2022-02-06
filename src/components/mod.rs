use crate::cartridge::CartridgeRom;
use crate::KIB;
use std::io::Read;

pub trait Memory {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, byte: u8);

    fn read_word(&self, address: u16) -> u16 {
        let lower = self.read_byte(address);
        let upper = self.read_byte(address.wrapping_add(1));

        ((upper as u16) << 8) | (lower as u16)
    }
}

pub struct Bus {
    cartridge: CartridgeRom,
    video_ram: VideoRam,
    external_ram: ExternalRam,
    work_ram: WorkRam,
    mirror: (),
    sprite_attribute_table: SpriteAttributeTable,
    not_usable: (),
    io: IORegisters,
    high_ram: HighRam,
    interrupt_enable: InterruptEnable,
}

impl Bus {
    pub fn new(rom: CartridgeRom) -> Self {
        Self {
            cartridge: rom,
            video_ram: VideoRam::new(),
            external_ram: ExternalRam::new(),
            work_ram: WorkRam::new(),
            mirror: (),
            sprite_attribute_table: SpriteAttributeTable::new(),
            not_usable: (),
            io: IORegisters::new(),
            high_ram: HighRam::new(),
            interrupt_enable: InterruptEnable::new(),
        }
    }
}

impl Memory for Bus {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0..=0x7FFF => self.cartridge.read_byte(address),
            0x8000..=0x9FFF => self.video_ram.read_byte(address - 0x8000),
            0xA000..=0xBFFF => self.external_ram.read_byte(address - 0xA000),
            0xC000..=0xDFFF => self.work_ram.read_byte(address - 0xC000),
            0xE000..=0xFDFF => self.work_ram.read_byte(address - 0xE000), // TODO log
            0xFE00..=0xFE9F => self.sprite_attribute_table.read_byte(address - 0xFE00),
            0xFEA0..=0xFEFF => 0xFF, // TODO log
            0xFF00..=0xFF7F => self.io.read_byte(address - 0xFF00),
            0xFF80..=0xFFFE => self.high_ram.read_byte(address - 0xFF80),
            0xFFFF..=0xFFFF => self.interrupt_enable.read_byte(address - 0xFFFF),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) {
        match address {
            0..=0x7FFF => self.cartridge.write_byte(address, byte),
            0x8000..=0x9FFF => self.video_ram.write_byte(address - 0x8000, byte),
            0xA000..=0xBFFF => self.external_ram.write_byte(address - 0xA000, byte),
            0xC000..=0xDFFF => self.work_ram.write_byte(address - 0xC000, byte),
            0xE000..=0xFDFF => self.work_ram.write_byte(address - 0xE000, byte), // TODO log
            0xFE00..=0xFE9F => self
                .sprite_attribute_table
                .write_byte(address - 0xFE00, byte),
            0xFEA0..=0xFEFF => (), // TODO log
            0xFF00..=0xFF7F => self.io.write_byte(address - 0xFF00, byte),
            0xFF80..=0xFFFE => self.high_ram.write_byte(address - 0xFF80, byte),
            0xFFFF..=0xFFFF => self.interrupt_enable.write_byte(address - 0xFFFF, byte),
        }
    }
}

struct VideoRam {
    ram: [u8; 8 * KIB],
    ppu_locked: bool,
}
impl VideoRam {
    pub fn new() -> Self {
        Self {
            ram: [0; 8 * KIB],
            ppu_locked: false,
        }
    }
}

impl Memory for VideoRam {
    fn read_byte(&self, address: u16) -> u8 {
        self.ram[address as usize]
    }

    fn write_byte(&mut self, address: u16, byte: u8) {
        self.ram[address as usize] = byte
    }
}

struct ExternalRam {
    ram: [u8; 8 * KIB],
}

impl ExternalRam {
    pub fn new() -> Self {
        Self { ram: [0; 8 * KIB] }
    }
}

impl Memory for ExternalRam {
    fn read_byte(&self, address: u16) -> u8 {
        self.ram[address as usize]
    }

    fn write_byte(&mut self, address: u16, byte: u8) {
        self.ram[address as usize] = byte
    }
}

struct WorkRam {
    ram: [u8; 2 * 4 * KIB],
}

impl WorkRam {
    pub fn new() -> Self {
        Self {
            ram: [0; 2 * 4 * KIB],
        }
    }
}

impl Memory for WorkRam {
    fn read_byte(&self, address: u16) -> u8 {
        self.ram[address as usize]
    }

    fn write_byte(&mut self, address: u16, byte: u8) {
        self.ram[address as usize] = byte
    }
}
struct SpriteAttributeTable {
    ram: [u8; 160],
}

impl SpriteAttributeTable {
    pub fn new() -> Self {
        Self { ram: [0; 160] }
    }
}

impl Memory for SpriteAttributeTable {
    fn read_byte(&self, address: u16) -> u8 {
        self.ram[address as usize]
    }

    fn write_byte(&mut self, address: u16, byte: u8) {
        self.ram[address as usize] = byte
    }
}
struct IORegisters {
    // controller: u8,
// communication: u16,
// timer: Timer,
// Sound,
// Waveform RAM,
// LCD,
// VRAM bank select,
// Boot rom,
// VRAM DMA,
// Palettes,
// WRAM bank,
}

impl IORegisters {
    pub fn new() -> Self {
        IORegisters {}
    }
}

impl Memory for IORegisters {
    fn read_byte(&self, address: u16) -> u8 {
        todo!()
    }

    fn write_byte(&mut self, address: u16, byte: u8) {
        todo!()
    }
}

struct HighRam {
    ram: [u8; 127],
}

impl HighRam {
    pub fn new() -> Self {
        Self { ram: [0; 127] }
    }
}

impl Memory for HighRam {
    fn read_byte(&self, address: u16) -> u8 {
        self.ram[address as usize]
    }

    fn write_byte(&mut self, address: u16, byte: u8) {
        self.ram[address as usize] = byte
    }
}

struct InterruptEnable {
    ie: u8,
}

impl InterruptEnable {
    pub fn new() -> Self {
        Self { ie: 0 }
    }
}

impl Memory for InterruptEnable {
    fn read_byte(&self, address: u16) -> u8 {
        debug_assert!(address == 0);
        self.ie
    }

    fn write_byte(&mut self, address: u16, byte: u8) {
        debug_assert!(address == 0);
        self.ie = byte
    }
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct FlatMemory {
    pub mem: Vec<u8>,
}

#[cfg(test)]
impl Memory for FlatMemory {
    fn read_byte(&self, address: u16) -> u8 {
        self.mem[address as usize]
    }

    fn write_byte(&mut self, address: u16, byte: u8) {
        self.mem[address as usize] = byte
    }
}
