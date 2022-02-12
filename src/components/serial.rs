use crate::components::interrupt_controller::InterruptController;
use crate::components::ByteAddressable;
use crate::GameBoyError;
use crate::RawResult;

pub struct Serial {
    sb: u8,
    sc: u8,
}

impl Serial {
    pub fn new() -> Self {
        Self { sb: 0, sc: 0 }
    }

    pub fn step(
        &mut self,
        _cycles: usize,
        interrupt_controller: &mut InterruptController,
    ) -> Option<u8> {
        if self.sc == 0x80 {
            todo!("External clock")
        }

        if self.sc == 0x81 {
            self.sc = 0x01;
            interrupt_controller.set_serial_interrupt();
            Some(self.sb)
        } else {
            None
        }
    }
}

impl Default for Serial {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteAddressable for Serial {
    fn read_byte(&self, address: u16) -> RawResult<u8> {
        match address {
            0xFF01 => Ok(self.sb),
            0xFF02 => Ok(self.sc),
            _ => Err(GameBoyError::NonMappedAddress {
                address,
                description: "Serial read",
            }),
        }
    }

    fn write_byte(&mut self, address: u16, byte: u8) -> RawResult<()> {
        match address {
            0xFF01 => self.sb = byte,
            0xFF02 => self.sc = byte,
            _ => {
                return Err(GameBoyError::NonMappedAddress {
                    address,
                    description: "Serial write",
                });
            }
        };
        Ok(())
    }
}
