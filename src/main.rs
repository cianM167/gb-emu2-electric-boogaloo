use std::{env, error::Error};

use crate::gb::{GameBoy, cartridge::load_rom, instructions::{opcodes, unimplemented}};

mod gb;
pub mod objects;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let cart = load_rom(&args[1])?;
    println!("Loaded ROM: {:?}", cart.header.title);

    println!("num: {:x}", (0x05 >> 3)& 0x07);

    let unimplemented_count = opcodes()
        .iter()
        .filter_map(|entry| entry.as_ref())
        .map(|instr| instr.execute as usize)
        .filter(|&addr| addr == unimplemented as usize)
        .count();

    println!("Normal instructions implemented: {}/245", 256 - unimplemented_count);

    let mut gb = GameBoy::new(cart);

    gb.run();

    Ok(())
}
