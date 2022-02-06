use crate::ByteAddressable;
use crate::components::AddressError;

pub struct Serial {
    sb: u8,
    sc: u8,
}

impl Serial {
    pub fn new() -> Self {
        Self { sb: 0, sc: 0 }
    }
}

impl ByteAddressable for Serial {
    fn read_byte(&self, address: u16) -> Result<u8, AddressError> {
        match address {
            0xFF01 => Ok(self.sb),
            0xFF02 => Ok(self.sc),
            _ => Err(AddressError::NonMappedAddress {address, description: "Serial read"})
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> Result<(), AddressError> {
        match address {
            0xFF01 => Ok(self.sb = byte),
            0xFF02 => Ok(self.sc = byte),
            _ => Err(AddressError::NonMappedAddress {address, description: "Serial write"})
        }
    }
}