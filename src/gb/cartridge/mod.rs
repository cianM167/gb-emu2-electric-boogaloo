

mod header;
mod mapper;
mod rom_data;
mod mbc1;
mod mbc2;
mod mbc3;
mod loader;
mod abi;

pub use loader::load_rom;
use serde::{Deserialize, Serialize};

use crate::gb::cartridge::{header::CartHeader, mapper::Mapper};

pub struct Cartridge {
    pub header: CartHeader,
    pub mapper: Box<dyn Mapper>,
}