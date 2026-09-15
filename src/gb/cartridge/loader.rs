use std::{fs};


use crate::gb::cartridge::{Cartridge, mbc1::Mbc1, mbc2::Mbc2, mbc3::Mbc3};

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
        0x00 => Box::new(RomData::new(rom)),// no mbc
        0x01 => Box::new(Mbc1::new(rom, 0)),// mbc 1
        0x02 => Box::new(Mbc1::new(rom, header.ram_size_bytes())),// mbc1 + memory
        0x03 => Box::new(Mbc1::new(rom, header.ram_size_bytes())),// mbc1 + ram + battery
        0x05 => Box::new(Mbc2::new(rom)),// mbc2
        0x06 => Box::new(Mbc2::new(rom)),// mbc2 + battery

        0x13 => Box::new(Mbc3::new(rom, header.ram_size_bytes())),//temporary not really implemented

        0x1B => Box::new(Mbc3::new(rom, header.ram_size_bytes())),// indescribably broken do not try at home
        _ => return Err(format!(
            "Unsupported cartridge type: {:#04X}",
            header.cartridge_type
        )),
    };

    Ok(Cartridge { header, mapper })
}