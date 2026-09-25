use serde::{Deserialize, Serialize};

use crate::gb::bus::Joypad;

#[derive(Serialize, Deserialize)]
pub struct BusState {
    pub wram: Vec<u8>,
    pub hram: Vec<u8>,
    pub joyp: u8,
    pub iflag: u8,
    pub ie: u8,
    pub div: u16,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    pub dma_source: u8,
    pub joypad: Joypad,
    // mapper_state: MapperState, // from your mapper's save_state()/load_state()
}