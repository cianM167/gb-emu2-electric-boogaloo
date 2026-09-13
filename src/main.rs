use std::{env, error::Error};

use crate::gb::{GameBoy, cartridge::load_rom, instructions::{opcodes, opcodes_cb, unimplemented}};

mod gb;
pub mod objects;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let cart = load_rom(&args[1])?;
    println!("Loaded ROM: {:?}", cart.header.title);

    let unimplemented_count = opcodes()
        .iter()
        .filter_map(|entry| entry.as_ref())
        .map(|instr| instr.execute as usize)
        .filter(|&addr| addr == unimplemented as usize)
        .count();

    let unimplemented_count_cb = opcodes_cb()
        .iter()
        .filter_map(|entry| entry.as_ref())
        .map(|instr| instr.execute as usize)
        .filter(|&addr| addr == unimplemented as usize)
        .count();

    println!("Normal instructions implemented: {}/245", 256 - unimplemented_count);
    println!("CB instructions implemented: {}/256", 256 - unimplemented_count_cb);

    println!("0x08 timing: {}", opcodes()[0x08].unwrap().cycles);

    let mut gb = GameBoy::new(cart);

    gb.run();

    Ok(())
}
