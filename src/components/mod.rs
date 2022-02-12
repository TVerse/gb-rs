pub mod bus;
pub mod cartridge;
pub mod controller;
pub mod cpu;
pub mod high_ram;
pub mod interrupt_controller;
#[allow(dead_code)]
pub mod ppu;
pub mod serial;
pub mod sound;
pub mod timer;
pub mod work_ram;

use crate::RawResult;

pub trait ByteAddressable {
    fn read_byte(&self, address: u16) -> RawResult<u8>;

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()>;
}
