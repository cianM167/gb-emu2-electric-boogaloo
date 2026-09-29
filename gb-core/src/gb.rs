use serde::{Deserialize, Serialize};
use alloc::{vec::Vec};
use crate::{bus::Bus, bus_state::BusState, cartridge::Cartridge, cpu::Cpu, ppu::Ppu};

#[derive(Serialize, Deserialize)]
pub struct SaveState {
    pub cpu: Cpu,
    pub ppu: Ppu,
    pub bus_state: BusState,
}

impl SaveState {
    pub fn new(cpu: &Cpu, ppu: &Ppu, bus: &Bus) -> Self {
        Self {
            cpu: *cpu,
            ppu: *ppu,
            bus_state: bus.to_state(),
        }
    }
}

pub struct GameBoy {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl GameBoy {
    pub fn new(cart: Cartridge, cgb: bool) -> Self {
        Self {
            cpu: Cpu::new(false),
            bus: Bus::new(cart, cgb),
        }
    }

    pub fn save_state_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        let state = SaveState::new(&self.cpu, &self.bus.ppu, &self.bus);
        postcard::to_allocvec(&state)
    }

    pub fn load_state_bytes(&mut self, bytes: &[u8]) -> Result<(), postcard::Error> {
        let state: SaveState = postcard::from_bytes(bytes)?;
        self.cpu = state.cpu;
        self.bus.ppu = state.ppu;
        self.bus.load_state(state.bus_state);
        Ok(())
    }

    pub fn step(&mut self) {
        self.cpu.step(&mut self.bus);
    }

    pub fn emu_frame(&mut self) {
        while !self.bus.ppu.ready {
            self.step();
        }
        self.bus.ppu.ready = false;
    }
}