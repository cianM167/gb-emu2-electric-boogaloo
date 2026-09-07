

mod header;
mod mapper;
mod rom_data;
mod loader;

pub use loader::load_rom;

use crate::gb::cartridge::{header::CartHeader, mapper::Mapper};

pub struct Cartridge {
    pub header: CartHeader,
    pub mapper: Box<dyn Mapper>,
}