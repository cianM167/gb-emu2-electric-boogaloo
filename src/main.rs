use std::{env, error::Error};

use rfd::FileDialog;

use crate::gb::{GameBoy, cartridge::load_rom, instructions::{opcodes, opcodes_cb, unimplemented}};

mod gb;
pub mod objects;

struct Args {
    headless: bool,
    frames: Option<u64>,
    input_script: Option<String>,
    trace_out: Option<String>,
    recording: Option<String>,
}

fn parse_args(raw: &[String]) -> Args {
    let mut a = Args {
        headless: false,
        frames: None,
        input_script: None,
        trace_out: None,
        recording: None
    };

    let mut i = 2;
    while i < raw.len() {
        match raw[i].as_str() {
            "--headless" => a.headless = true,
            "--record" => { i += 1; a.recording = Some(raw[i].parse().unwrap()); }
            "--frames" => { i += 1; a.frames = Some(raw[i].parse().unwrap()); }
            "--input-script" => { i += 1; a.input_script = Some(raw[i].clone()); }
            "--trace-out" => { i += 1; a.trace_out = Some(raw[i].clone()); }
            _ => {}
        }
        i += 1;
    }

    a
}

fn pick_rom() -> Option<String> {
    FileDialog::new()
        .add_filter("Game Boy ROM", &["gb", "gbc"])
        .add_filter("All files", &["*"])
        .set_title("Open ROM")
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

fn main() -> Result<(), Box<dyn Error>> {
    let raw_args: Vec<String> = env::args().collect();

    let args = parse_args(&raw_args);

    let rom_path = if raw_args.len() > 1 {
        raw_args[1].clone()
    } else {
        pick_rom().ok_or("No ROM selected")?
    };

    let cart = load_rom(&rom_path)?;

    println!("Loaded ROM: {:?}", cart.header.title);
    println!("Cart type: {:02X}", cart.header.cartridge_type);

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

    gb.run(&args);

    Ok(())
}
