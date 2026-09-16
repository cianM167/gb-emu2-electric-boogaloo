use serde::{Deserialize, Serialize};

use crate::gb::bus::Joypad;

#[derive(Serialize, Deserialize)]
pub struct BusState {
    pub vram: Vec<u8>,
    pub wram: Vec<u8>,
    pub hram: Vec<u8>,
    pub oam: Vec<u8>,
    pub joyp: u8,
    pub iflag: u8,
    pub ie: u8,
    pub div: u16,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    pub lcdc: u8,
    pub stat: u8,
    pub scy: u8,
    pub scx: u8,
    pub ly: u8,
    pub bgp: u8,
    pub opb0: u8,
    pub opb1: u8,
    pub wy: u8,
    pub wx: u8,
    pub dma_source: u8,
    pub joypad: Joypad,
    // mapper_state: MapperState, // from your mapper's save_state()/load_state()
}