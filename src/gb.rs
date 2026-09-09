use crate::gb::{bus::Bus, cartridge::Cartridge, cpu::Cpu, timer::Timer};

pub mod ram;
pub mod cpu;
pub mod instructions;
pub mod registers;
pub mod bus;
pub mod cartridge;
mod ppu;
mod timer;

pub struct GameBoy {
    cpu: Cpu,
    bus: Bus,
    timer: Timer,
}

impl GameBoy {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cpu: Cpu::new(true),
            bus: Bus::new(cart),
            timer: Timer::new(),
        }
    }

    pub fn run(&mut self) {
        while true {
            self.step();
        }
    }

    fn step(&mut self) { // cpu step
        let mut cycles = 0;
        cycles += self.cpu.step(&mut self.bus);
        self.timer.step(cycles, &mut self.bus);
    }
}