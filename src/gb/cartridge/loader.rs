use std::{fs};


use crate::gb::cartridge::Cartridge;

use super::{
    header::CartHeader,
    rom_data::RomData,
    mapper::Mapper,
};

pub fn load_rom(path: &str) -> Result<Cartridge, String> {
    let rom: Vec<u8> = fs::read(path)
        .map_err(|e| format!("Failed to read ROM: {}", e))?;

    //println!("{:?}", rom);

    let header: CartHeader = CartHeader::parse(&rom)?;

    let mapper: Box<dyn Mapper> = match header.cartridge_type {
        0x00 => Box::new(RomData::new(rom)),
        0x01 => Box::new(RomData::new(rom)),
        0x02 => Box::new(RomData::new(rom)),
        0x03 => Box::new(RomData::new(rom)),// also not finished
        0x13 => Box::new(RomData::new(rom)),//temporary not really implemented
        _ => return Err(format!(
            "Unsupported cartridge type: {:#04X}",
            header.cartridge_type
        )),
    };

    Ok(Cartridge { header, mapper })
}