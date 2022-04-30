mod buffer;

use crate::core::interrupt_controller::Interrupt;
use crate::core::{Addressable, EventContext, InterruptContext};
use crate::ExecutionEvent;
use bitflags::bitflags;
use std::mem;

pub use buffer::{Buffer, Line};

bitflags! {
    struct LCDC: u8 {
        const LCD_PPU_ENABLE = 0b10000000;
        const WINDOW_TILE_MAP_AREA = 0b01000000;
        const WINDOW_ENABLE = 0b00100000;
        const BG_WINDOW_TILE_DATA_AREA = 0b00010000;
        const BG_TILE_MAP_AREA = 0b00001000;
        const OBJ_SIZE = 0b00000100;
        const OBJ_ENABLE = 0b00000010;
        const BG_WINDOW_ENABLE = 0b00000001;
    }
}

bitflags! {
    struct Stat: u8 {
        const LYC_IS_LY_INTERRUPT = 0b01000000;
        const OAM_INTERRUPT = 0b00100000;
        const VBLANK_INTERRUPT = 0b00010000;
        const HBLANK_INTERRUPT = 0b00001000;
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ColorId {
    Zero,
    One,
    Two,
    Three,
}

impl ColorId {
    fn from_bits(lsb: bool, msb: bool) -> Self {
        // LSb first
        match (lsb, msb) {
            (false, false) => Self::Zero,
            (true, false) => Self::One,
            (false, true) => Self::Two,
            (true, true) => Self::Three,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Color {
    White,
    LightGrey,
    DarkGrey,
    Black,
}

impl Color {
    fn from_color_id(color_id: ColorId, palette_data: u8) -> Self {
        let bits = match color_id {
            ColorId::Zero => palette_data & 0b11,
            ColorId::One => (palette_data >> 2) & 0b11,
            ColorId::Two => (palette_data >> 4) & 0b11,
            ColorId::Three => (palette_data >> 6) & 0b11,
        };

        Self::from_bits(bits)
    }

    fn from_bits(bits: u8) -> Self {
        debug_assert!(bits <= 3);

        match bits {
            0b00 => Self::White,
            0b01 => Self::LightGrey,
            0b10 => Self::DarkGrey,
            0b11 => Self::Black,
            _ => unreachable!(),
        }
    }
}

struct TileData {
    data: [(u8, u8); 8],
}

impl TileData {
    fn index(&self, x: u8, y: u8) -> ColorId {
        let mask = 0x80 >> x;
        let lsb = self.data[y as usize].0 & mask > 0;
        let msb = self.data[y as usize].1 & mask > 0;

        ColorId::from_bits(lsb, msb)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Mode {
    HBlank0,
    VBlank1,
    OAMSearch2,
    LCDOn3,
}

impl Mode {
    fn bits(&self) -> u8 {
        match self {
            Mode::HBlank0 => 0b00,
            Mode::VBlank1 => 0b01,
            Mode::OAMSearch2 => 0b10,
            Mode::LCDOn3 => 0b11,
        }
    }
}

#[derive(Debug)]
pub struct Ppu {
    tile_data_1: [u8; 0x800],
    tile_data_2: [u8; 0x800],
    tile_data_3: [u8; 0x800],
    tile_map_1: [u8; 0x400],
    tile_map_2: [u8; 0x400],
    x_clock: u16,
    x_pixel: u8,
    ly: u8,
    mode: Mode,
    lcdc: LCDC,
    scx: u8,
    scy: u8,
    bg_palette: u8,
    frame_buffer: Box<Buffer>,
    lyc: u8,
    stat: Stat,
    lyc_is_ly: bool,
    previous_stat_interrupt: bool,
}

impl Ppu {
    /*
    144 visible scanlines + 10 vblank, 456 dots per line
    State move logic:
    Per line:
    Mode 2: 80 dots
    Mode 3: now 160 dots, inaccurate
    Mode 0: (456 - Mode2 - Mode3) dots, will be 212 now
    Then 10 lines of
    Mode 1: 456 dots

    0<=y<144: mode 230
    144<=y<154: mode 1

    0<=x_clock<80: mode 2
    0<=x_pixel<144: mode 3
    x_clock<456: mode 0
     */

    // TODO how do I sync with vblank if ppu is off? Cycle counting instead with initial known sync?
    pub fn tick<I: InterruptContext, E: EventContext>(&mut self, ctx: &mut I, event_ctx: &mut E) {
        if !self.lcdc.contains(LCDC::LCD_PPU_ENABLE) {
            return;
        }
        match self.mode {
            Mode::HBlank0 => {
                self.x_clock += 1;
                if self.x_clock == 456 {
                    self.x_clock = 0;
                    self.ly += 1;
                    if self.ly == 144 {
                        self.mode = Mode::VBlank1;
                        ctx.raise_interrupt(Interrupt::VBlank);
                        event_ctx.push_event(ExecutionEvent::FrameReady(mem::replace(
                            &mut self.frame_buffer,
                            Buffer::boxed(),
                        )));
                        event_ctx.push_event(ExecutionEvent::PpuModeSwitch {
                            mode: self.mode,
                            x: self.x_clock,
                            y: self.ly,
                        })
                    } else {
                        self.mode = Mode::OAMSearch2;
                        event_ctx.push_event(ExecutionEvent::PpuModeSwitch {
                            mode: self.mode,
                            x: self.x_clock,
                            y: self.ly,
                        })
                    }
                }
            }
            Mode::VBlank1 => {
                self.x_clock += 1;
                if self.x_clock == 456 {
                    self.ly += 1;
                    self.x_clock = 0;
                    if self.ly == 154 {
                        self.ly = 0;
                        self.mode = Mode::OAMSearch2;
                        event_ctx.push_event(ExecutionEvent::PpuModeSwitch {
                            mode: self.mode,
                            x: self.x_clock,
                            y: self.ly,
                        })
                    }
                }
            }
            Mode::OAMSearch2 => {
                // Pretend nothing happens here
                self.x_clock += 1;
                if self.x_clock == 80 {
                    self.mode = Mode::LCDOn3;
                    self.x_pixel = 0;
                    event_ctx.push_event(ExecutionEvent::PpuModeSwitch {
                        mode: self.mode,
                        x: self.x_clock,
                        y: self.ly,
                    })
                }
            }
            Mode::LCDOn3 => {
                // Pretend 1 cycle == 1 pixel
                let x = self.x_pixel.wrapping_add(self.scx);
                let y = self.ly.wrapping_add(self.scy);
                let color_id = self.get_current_pixel_color_id(x, y);
                let color = Color::from_color_id(color_id, self.bg_palette);
                self.frame_buffer[self.ly as usize][self.x_pixel as usize] = color;
                event_ctx.push_event(ExecutionEvent::PpuPixelPushed(
                    self.x_pixel,
                    self.ly,
                    color_id,
                ));
                self.x_clock += 1;
                self.x_pixel += 1;
                if self.x_pixel == 160 {
                    self.mode = Mode::HBlank0;
                    event_ctx.push_event(ExecutionEvent::PpuModeSwitch {
                        mode: self.mode,
                        x: self.x_clock,
                        y: self.ly,
                    })
                }
            }
        }
        self.lyc_is_ly = self.ly == self.lyc;
        let mut stat = false;
        if self.stat.contains(Stat::LYC_IS_LY_INTERRUPT) {
            stat |= self.lyc_is_ly;
        }
        if self.stat.contains(Stat::VBLANK_INTERRUPT) {
            stat |= self.mode == Mode::VBlank1;
        }
        if self.stat.contains(Stat::OAM_INTERRUPT) {
            stat |= self.mode == Mode::OAMSearch2;
        }
        if self.stat.contains(Stat::HBLANK_INTERRUPT) {
            stat |= self.mode == Mode::HBlank0;
        }
        if !self.previous_stat_interrupt && stat {
            ctx.raise_interrupt(Interrupt::LcdStat)
        }
        self.previous_stat_interrupt = stat;
    }

    /*
    (all additions wrapping)
    Current pixel is x_pixel + scx, ly + scy
    That's tile coordinate t_x = (x_pixel + scx) / 8, t_y = (ly + scy) / 8. (32 by 32)
    So tile map index is t_x + 32 * t_y.

    In-tile coordinate: p_x = (x_pixel + scx) % 8, p_y = (ly + scy) % 8
    In-tile index: p_x + 8 * p_y
     */
    fn get_current_pixel_color_id(&self, target_x: u8, target_y: u8) -> ColorId {
        let tile_x = target_x / 8;
        let tile_y = target_y / 8;
        let tile_map_idx = (tile_x as usize) + (32 * (tile_y as usize));

        let tile_idx = if self.lcdc.contains(LCDC::BG_TILE_MAP_AREA) {
            self.tile_map_2[tile_map_idx]
        } else {
            self.tile_map_1[tile_map_idx]
        };
        let tile_data = self.read_tile_data_bg_win(tile_idx);

        let pixel_x = target_x % 8;
        let pixel_y = target_y % 8;

        tile_data.index(pixel_x, pixel_y)
    }

    fn read_vram(&self, address: u16) -> u8 {
        if self.mode == Mode::LCDOn3 {
            0xFF
        } else {
            let a = address as usize;
            match address {
                0x8000..=0x87FF => self.tile_data_1[a - 0x8000],
                0x8800..=0x8FFF => self.tile_data_2[a - 0x8800],
                0x9000..=0x97FF => self.tile_data_3[a - 0x9000],
                0x9800..=0x9BFF => self.tile_map_1[a - 0x9800],
                0x9C00..=0x9FFF => self.tile_map_2[a - 0x9C00],
                _ => unreachable!(),
            }
        }
    }

    fn write_vram(&mut self, address: u16, value: u8) {
        if self.mode != Mode::LCDOn3 {
            let a = address as usize;
            match address {
                0x8000..=0x87FF => self.tile_data_1[a - 0x8000] = value,
                0x8800..=0x8FFF => self.tile_data_2[a - 0x8800] = value,
                0x9000..=0x97FF => self.tile_data_3[a - 0x9000] = value,
                0x9800..=0x9BFF => self.tile_map_1[a - 0x9800] = value,
                0x9C00..=0x9FFF => self.tile_map_2[a - 0x9C00] = value,
                _ => unreachable!(),
            }
        }
    }

    fn read_tile_data_bg_win(&self, offset: u8) -> TileData {
        let o = offset as usize;
        if self.lcdc.contains(LCDC::BG_WINDOW_TILE_DATA_AREA) {
            // Low, unsigned
            match offset {
                0..=127 => Self::read_tile_data_at_offset(&self.tile_data_1, o),
                128..=255 => Self::read_tile_data_at_offset(&self.tile_data_2, o - 128),
            }
        } else {
            // High, signed
            match offset {
                0..=127 => Self::read_tile_data_at_offset(&self.tile_data_3, o),
                128..=255 => Self::read_tile_data_at_offset(&self.tile_data_2, o - 128),
            }
        }
    }

    fn read_tile_data_at_offset(data: &[u8; 0x800], offset: usize) -> TileData {
        let tile_data = &data[(offset * 16)..((offset + 1) * 16)];
        let tile_data: [u8; 16] = tile_data.try_into().expect("Incorrect tile_data length");
        let tile_data: [(u8, u8); 8] = tile_data
            .chunks(2)
            .map(|c| (c[0], c[1]))
            .collect::<Vec<_>>()
            .try_into()
            .expect("Incorrect tile_data length after tupling");

        TileData { data: tile_data }
    }
}

impl Addressable for Ppu {
    fn read(&self, address: u16) -> Option<u8> {
        match address {
            0x8000..=0x9FFF => Some(self.read_vram(address)),
            0xFF40 => Some(self.lcdc.bits),
            0xFF41 => {
                let mut stat = self.stat.bits;
                if self.lyc_is_ly {
                    stat |= 0b00000100;
                }
                stat |= self.mode.bits();

                Some(stat)
            }
            0xFF42 => Some(self.scy),
            0xFF43 => Some(self.scx),
            0xFF44 => Some(self.ly),
            0xFF45 => Some(self.lyc),
            0xFF47 => Some(self.bg_palette),
            _ => None,
        }
    }

    fn write(&mut self, address: u16, value: u8) -> Option<()> {
        match address {
            0x8000..=0x9FFF => {
                self.write_vram(address, value);
                Some(())
            }
            0xFF40 => {
                self.lcdc = LCDC::from_bits_truncate(value);
                Some(())
            }
            0xFF41 => {
                self.stat = Stat::from_bits_truncate(value);
                Some(())
            }
            0xFF42 => {
                self.scy = value;
                Some(())
            }
            0xFF43 => {
                self.scx = value;
                Some(())
            }
            0xFF45 => {
                self.lyc = value;
                Some(())
            }
            0xFF47 => {
                self.bg_palette = value;
                Some(())
            }
            _ => None,
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            tile_data_1: [0; 0x800],
            tile_data_2: [0; 0x800],
            tile_data_3: [0; 0x800],
            tile_map_1: [0; 0x400],
            tile_map_2: [0; 0x400],
            x_clock: 0,
            x_pixel: 0,
            ly: 144,
            mode: Mode::VBlank1,
            lcdc: LCDC::empty(),
            scx: 0,
            scy: 0,
            bg_palette: 0,
            frame_buffer: Buffer::boxed(),
            lyc: 0,
            stat: Stat::empty(),
            lyc_is_ly: false,
            previous_stat_interrupt: false,
        }
    }
}

impl std::fmt::Display for Ppu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Mode: {:?}", self.mode)?;
        writeln!(f, "LCDC: {:?}", self.lcdc)?;
        writeln!(f, "STAT: {:?}", self.stat)
    }
}
