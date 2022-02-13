use crate::components::ByteAddressable;
use crate::KIB;
use crate::{GameBoyError, RawResult};

#[derive(Debug, Clone)]
pub struct Ppu {
    vram: [u8; 8 * KIB],
    oam: [u8; 160],
    lcdc: Lcdc,
    stat: Stat,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    wy: u8,
    wx: u8,
    bgp: Palette,
    obp0: Palette,
    obp1: Palette,
    buf: PixelBuffer,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vram: [0; 8 * KIB],
            oam: [8; 160],
            lcdc: Lcdc { data: 0 },
            stat: Stat { data: 0 },
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            wy: 0,
            wx: 0,
            bgp: Palette::from_byte(0),
            obp0: Palette::from_byte(0),
            obp1: Palette::from_byte(0),
            buf: PixelBuffer::new(),
        }
    }

    pub fn vram_raw(&self) -> &[u8] {
        &self.vram
    }

    pub fn oam_raw(&self) -> &[u8] {
        &self.oam
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteAddressable for Ppu {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        let a = address as usize;
        match address {
            0x8000..=0x9FFF => Ok(self.vram[a - 0x8000]),
            0xFE00..=0xFE9F => Ok(self.oam[a - 0xFE00]),
            0xFF40 => Ok(self.lcdc.as_byte()),
            0xFF41 => Ok(self.stat.as_byte()),
            0xFF42 => Ok(self.scy),
            0xFF43 => Ok(self.scx),
            0xFF44 => Ok(self.ly),
            0xFF45 => Ok(self.lyc),
            0xFF4A => Ok(self.wy),
            0xFF4B => Ok(self.wx),
            0xFF47 => Ok(self.bgp.as_byte()),
            0xFF48 => Ok(self.obp0.as_byte()),
            0xFF49 => Ok(self.obp1.as_byte()),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "PPU read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()> {
        let a = address as usize;
        match address {
            0x8000..=0x9FFF => self.vram[a - 0x8000] = byte,
            0xFE00..=0xFE9F => self.oam[a - 0xFE00] = byte,
            0xFF40 => self.lcdc = Lcdc { data: byte },
            0xFF41 => self.stat.set_byte(byte),
            0xFF42 => self.scy = byte,
            0xFF43 => self.scx = byte,
            0xFF44 => {
                return Err(GameBoyError::NonMappedAddress {
                    address,
                    description: "PPU LY write",
                })
            }
            0xFF45 => self.lyc = byte,
            0xFF4A => self.wy = byte,
            0xFF4B => self.wx = byte,
            0xFF47 => self.bgp = Palette::from_byte(byte),
            0xFF48 => self.obp0 = Palette::from_byte(byte),
            0xFF49 => self.obp1 = Palette::from_byte(byte),
            _ => {
                return Err(GameBoyError::NonMappedAddress {
                    address,
                    description: "PPU write",
                })
            }
        };
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Lcdc {
    data: u8,
}

impl Lcdc {
    fn lcd_enable(&self) -> bool {
        self.data & 0x80 != 0
    }

    fn window_tile_map_area(&self) -> bool {
        self.data & 0x40 != 0
    }

    fn window_enable(&self) -> bool {
        self.data & 0x20 != 0
    }

    fn bg_window_tile_data_area(&self) -> bool {
        self.data & 0x10 != 0
    }

    fn bg_tile_map_area(&self) -> bool {
        self.data & 0x08 != 0
    }

    fn obj_size(&self) -> bool {
        self.data & 0x04 != 0
    }

    fn obj_enable(&self) -> bool {
        self.data & 0x02 != 0
    }

    fn bg_window_enable(&self) -> bool {
        self.data & 0x01 != 0
    }

    fn as_byte(&self) -> u8 {
        self.data
    }
}

#[derive(Debug, Clone)]
struct Stat {
    data: u8,
}

impl Stat {
    const READONLY_FIELD_MASK: u8 = 0x07;

    fn lyc_eq_ly_interrupt_source(&self) -> bool {
        self.data & 0x40 != 0
    }

    fn mode_2_oam_interrupt_source(&self) -> bool {
        self.data & 0x20 != 0
    }

    fn mode_1_vblank_interrupt_source(&self) -> bool {
        self.data & 0x10 != 0
    }

    fn mode_0_hblank_interrupt_source(&self) -> bool {
        self.data & 0x08 != 0
    }

    fn lyc_eq_ly_flag(&self) -> bool {
        self.data & 0x04 != 0
    }

    fn mode(&self) -> Mode {
        Mode::from_byte(self.data)
    }

    fn as_byte(&self) -> u8 {
        self.data
    }

    fn set_byte(&mut self, byte: u8) {
        let read_only_part = self.data & Self::READONLY_FIELD_MASK;
        let byte = byte & !Self::READONLY_FIELD_MASK;
        self.data = read_only_part | byte;
    }

    fn set_lyx_eq_ly_flag(&mut self, value: bool) {
        if value {
            self.data |= 0x04;
        } else {
            self.data &= !0x04;
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        self.data = (self.data & !0x03) | mode.to_byte()
    }
}

#[derive(Debug, Clone)]
enum Mode {
    HBlank,
    VBlank,
    SearchingOAM,
    TransferringData,
}

impl Mode {
    fn from_byte(byte: u8) -> Self {
        let byte = byte & 0x03;
        if byte == 0x00 {
            Self::HBlank
        } else if byte == 0x01 {
            Self::VBlank
        } else if byte == 0x02 {
            Self::SearchingOAM
        } else if byte == 0x03 {
            Self::TransferringData
        } else {
            unreachable!()
        }
    }

    fn to_byte(&self) -> u8 {
        match self {
            Mode::HBlank => 0x00,
            Mode::VBlank => 0x01,
            Mode::SearchingOAM => 0x02,
            Mode::TransferringData => 0x03,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Color {
    White,
    LightGray,
    DarkGray,
    Black,
}

impl Color {
    fn as_byte(&self) -> u8 {
        match self {
            Color::White => 0x00,
            Color::LightGray => 0x01,
            Color::DarkGray => 0x02,
            Color::Black => 0x03,
        }
    }

    fn from_byte(byte: u8) -> Self {
        let byte = byte & 0x03;
        if byte == 0x00 {
            Self::White
        } else if byte == 0x01 {
            Self::LightGray
        } else if byte == 0x02 {
            Self::DarkGray
        } else if byte == 0x03 {
            Self::Black
        } else {
            unreachable!()
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Palette {
    idx0: Color,
    idx1: Color,
    idx2: Color,
    idx3: Color,
}

impl Palette {
    fn as_byte(&self) -> u8 {
        (self.idx3.as_byte() << 6)
            | (self.idx2.as_byte() << 4)
            | (self.idx1.as_byte() << 2)
            | (self.idx0.as_byte())
    }

    fn from_byte(byte: u8) -> Self {
        Self {
            idx0: Color::from_byte(byte),
            idx1: Color::from_byte(byte >> 2),
            idx2: Color::from_byte(byte >> 4),
            idx3: Color::from_byte(byte >> 6),
        }
    }
}

#[derive(Debug, Clone)]
struct PixelBuffer {
    buf: [[Color; 160]; 144],
}

impl PixelBuffer {
    fn new() -> Self {
        Self {
            buf: [[Color::White; 160]; 144],
        }
    }

    fn set(&mut self, x: usize, y: usize, c: Color) {
        self.buf[y][x] = c
    }
}
