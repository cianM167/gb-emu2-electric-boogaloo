use std::{env, error::Error};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use minifb::KeyRepeat::No;
use ringbuf::{HeapCons, HeapProd, HeapRb, traits::{Consumer, Split}};
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
    custom_cart: Option<String>,
    cgb_mode: bool,
}

fn parse_args(raw: &[String]) -> Args {
    let mut a = Args {
        headless: false,
        frames: None,
        input_script: None,
        trace_out: None,
        recording: None,
        custom_cart: None,
        cgb_mode: false
    };

    let mut i = 2;
    while i < raw.len() {
        match raw[i].as_str() {
            "--headless" => a.headless = true,
            "--record" => { i += 1; a.recording = Some(raw[i].parse().unwrap()); }
            "--frames" => { i += 1; a.frames = Some(raw[i].parse().unwrap()); }
            "--input-script" => { i += 1; a.input_script = Some(raw[i].clone()); }
            "--trace-out" => { i += 1; a.trace_out = Some(raw[i].clone()); }
            "--custom-cart" => { i += 1; a.custom_cart = Some(raw[i].clone()); }
            "--cgb" => a.cgb_mode = true,
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

    let cart = load_rom(&rom_path, args.custom_cart.as_deref())?;

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

    // audio init
    let rb = HeapRb::<f32>::new(4096);
    let (producer, consumer) = rb.split();

    let (stream, sample_rate) = setup_audio(consumer);
    stream.play().unwrap();

    let mut gb = GameBoy::new(cart, producer, sample_rate, args.cgb_mode);

    gb.run(&args);

    Ok(())
}

fn setup_audio(mut consumer: HeapCons<f32>) -> (cpal::Stream, u32) {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device");
    let config = device.default_output_config().unwrap();
    let sample_rate = config.sample_rate();
    let channels = config.channels() as usize;

    let stream = device.build_output_stream(
        config.into(),
        move |data: &mut [f32], _| {
            for frame in data.chunks_mut(channels) {
                let sample = consumer.try_pop().unwrap_or(0.0);
                for out in frame.iter_mut() {
                    *out = sample; // duplicate mono sample to all channels for now
                }
            }
        },
        |err| eprintln!("audio stream error: {err}"),
        None,
    ).unwrap();

    (stream, sample_rate)
}
