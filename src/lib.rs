mod cartridge;
mod cpu;
mod components;

pub use crate::cartridge::CartridgeRom;
use crate::cpu::Cpu;
use crate::cpu::ExecutingCpu;
use crate::components::Memory;
pub use crate::components::Bus;

const KIB: usize = 1024;

pub struct GameBoy {
    cpu: Cpu,
    memory_map: Bus,
}

impl GameBoy {
    pub fn new(rom: CartridgeRom) -> Self {
        Self {
            cpu: Cpu::new(),
            memory_map: Bus::new(rom),
        }
    }

    pub fn step(&mut self) {
        let mut executing_cpu = ExecutingCpu::new(&mut self.cpu, &mut self.memory_map);

        executing_cpu.step().unwrap();
    }

    pub fn get_serial(&mut self) -> Option<u8> {
        if self.memory_map.read_byte(0xFF02) == 81 {
            self.memory_map.write_byte(0xFF02, 0x01);
            self.memory_map.write_byte(0xFF0F, self.memory_map.read_byte(0xFF04) | 0x04);

            Some(self.memory_map.read_byte(0xFF01))
        } else {
            None
        }
    }
}
