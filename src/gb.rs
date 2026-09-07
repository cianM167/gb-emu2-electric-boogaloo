use crate::gb::{bus::Bus, cartridge::Cartridge, cpu::Cpu};

pub mod ram;
pub mod cpu;
pub mod instructions;
pub mod registers;
pub mod bus;
pub mod cartridge;

pub struct GameBoy {
    cpu: Cpu,
    bus: Bus,
}

impl GameBoy {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cpu: Cpu::new(true),
            bus: Bus::new(cart),
        }
    }

    pub fn run(&mut self) {
        while true {
            self.step()
        }
    }

    fn step(&mut self) { // cpu step
        self.cpu.step(&mut self.bus);
    }
}