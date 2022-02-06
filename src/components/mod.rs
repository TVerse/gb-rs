pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod ppu;
pub mod serial;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AddressError {
    #[error("Tried to use nonmapped address {address}: {description}")]
    NonMappedAddress {
        address: u16,
        description: &'static str,
    },
}

pub trait ByteAddressable {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError>;

    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError>;
}
