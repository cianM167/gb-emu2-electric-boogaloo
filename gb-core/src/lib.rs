#![cfg_attr(not(feature = "std"), no_std)]
#[macro_use]
extern crate alloc;

pub mod cpu;
pub mod instructions;
pub mod registers;
pub mod bus;
pub mod cartridge;
pub mod ppu;
pub mod bus_state;
pub mod apu;
pub mod gb;
pub mod objects;