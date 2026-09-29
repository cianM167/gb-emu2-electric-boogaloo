use std::{fs::{self, File}, io::{BufWriter, Write}, thread::sleep, time::{Duration, Instant}};

use gb_core::{bus::Bus, bus_state::BusState, cartridge::Cartridge, cpu::Cpu, gb::GameBoy, ppu::Ppu};
use gilrs::{Button, Event, EventType, Gilrs};
use minifb::{Key, Window, WindowOptions};

use gb_core::{
    bus::Joypad
};
use ringbuf::HeapProd;
use serde::{Deserialize, Serialize};

use crate::Args;  

const FRAME_TIME: Duration = Duration::from_nanos(16_742_706);

#[derive(Default)]
struct PadState {
    a: bool, b: bool, start: bool, select: bool,
    up: bool, down: bool, left: bool, right: bool,
}

struct InputScript {
    frames: Vec<[bool; 8]>,// right,left,up,down,a,b,select,start
}

impl InputScript {
    fn load(path: &str) -> Self {
        let text = fs::read_to_string(path).unwrap();
        let frames = text.lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(|l| {
                let l = l.trim();
                let mut b = [false; 8];
                for (i, c) in l.chars().take(8).enumerate() {
                    b[i] = c == '1';
                }
                b
            })
            .collect();
        Self { frames }
    }

    fn apply(&self, frame: u64, joypad: &mut Joypad) {
        let state = self.frames.get(frame as usize).copied().unwrap_or([false; 8]);
        joypad.right  = state[0];
        joypad.left   = state[1];
        joypad.up     = state[2];
        joypad.down   = state[3];
        joypad.a      = state[4];
        joypad.b      = state[5];
        joypad.select = state[6];
        joypad.start  = state[7];
    }
}

struct Tracer {
    writer: BufWriter<File>
}

impl Tracer {
    fn new(path: &str) -> Self {
        Self {
            writer: BufWriter::with_capacity(1 << 20, File::create(path).unwrap())
        }
    }

    fn log(
        &mut self,
        regs: (u8, u8, u8, u8, u8, u8, u8, u8, u16, u16),
        pcmem: (u8, u8, u8, u8),
        ppu: (u8, u8, u8),
        cycles: u64,
        total: u64,
    ) {
        let (a, f, b, c, d, e, h, l, sp, pc) = regs;
        let (m0, m1, m2, m3) = pcmem;
        let (ly, stat, lcdc) = ppu;

        writeln!(
            self.writer,
            "A:{a:02X} F:{f:02X} B:{b:02X} C:{c:02X} D:{d:02X} E:{e:02X} H:{h:02X} L:{l:02X} SP:{sp:04X} PC:{pc:04X} PCMEM:{m0:02X},{m1:02X},{m2:02X},{m3:02X} LCDC:{lcdc:02X} LY:{ly:02X} Cycles:{cycles} Total:{total}"
        ).unwrap()
    }

    fn flush(&mut self) {
        self.writer.flush().unwrap()
    }
}

struct Recorder {
    writer: BufWriter<File>
}

impl Recorder {
    fn new(path: &str) -> Self {
        Self {
            writer: BufWriter::with_capacity(1 << 20, File::create(path).unwrap())
        }
    }

    fn record(&mut self, joypad: &Joypad) {
        let right = joypad.right as u8;
        let left = joypad.left as u8;
        let up = joypad.up as u8;
        let down = joypad.down as u8;
        let a = joypad.a as u8;
        let b = joypad.b as u8;
        let select = joypad.select as u8;
        let start = joypad.start as u8;

        writeln!(
            self.writer,
            "{right}{left}{up}{down}{a}{b}{select}{start}"
        ).unwrap()
    }

    fn flush(&mut self) {
        self.writer.flush().unwrap()
    }
}

#[derive(Serialize, Deserialize)]
struct SaveState {
    cpu: Cpu,
    ppu: Ppu,
    bus_state: BusState,
}

impl SaveState {
    pub fn new(cpu: &Cpu, ppu: &Ppu, bus: &Bus) -> Self {
        Self {
            cpu: *cpu,
            ppu: *ppu,
            bus_state: bus.to_state(),
        }
    }
}

pub struct DesktopApp {
    gb: GameBoy,
    window: Window,
    gilrs: Gilrs,
    pad_state: PadState,
    audio_producer: HeapProd<f32>,
}

impl DesktopApp {
    pub fn new(cart: Cartridge, cgb: bool, audio_producer: HeapProd<f32>) -> Self {
        Self {
            gb: GameBoy::new(cart, cgb),
            window: Window::new("Ferro boy", 640, 576, WindowOptions::default()).unwrap(),
            gilrs: Gilrs::new().unwrap(),
            pad_state: PadState::default(),
            audio_producer,
        }
    }

    fn save_state_to_disk(&self) {
        let title = &self.gb.bus.cart.header.title;
        let path = format!("saves/{title}.stat");

        match self.gb.save_state_bytes() {
            Ok(bytes) => {
                std::fs::write(path, bytes).expect("ruh roh, file save failed");
                println!("State saved");
            }
            Err(e) => eprintln!("Serialization failed: {e}"),
        }
    }

    fn load_state_from_disk(&mut self) {
        let title = &self.gb.bus.cart.header.title;
        let path = format!("saves/{title}.stat");

        if let Ok(bytes) = std::fs::read(&path) {
            if self.gb.load_state_bytes(&bytes).is_ok() {
                println!("State loaded");
            } else {
                eprintln!("deserialize failed");
            }
        } else {
            eprintln!("error loading save state: {path}");
        }
    }

    fn set_pad_button(&mut self, button: Button, pressed: bool) {
        // println!("pad updated button: {button:?}");
        match button {
            Button::South       => self.pad_state.a = pressed,// A/Cross
            Button::East        => self.pad_state.b = pressed,// B/Circle
            Button::Start       => self.pad_state.start = pressed,
            Button::Select      => self.pad_state.select = pressed,
            Button::DPadUp      => self.pad_state.up = pressed,
            Button::DPadDown    => self.pad_state.down = pressed,
            Button::DPadLeft    => self.pad_state.left = pressed,
            Button::DPadRight   => self.pad_state.right = pressed,
            _ => {}
        }
    }

    pub fn run(&mut self,args: &Args) {
        let script = args.input_script.as_ref().map(|p| InputScript::load(p));
        let mut tracer = args.trace_out.as_ref().map(|p| Tracer::new(p));
        let mut recorder = args.recording.as_ref().map(|p| Recorder::new(p));

        if args.cgb_mode {
            self.gb.cpu.registers.set_a(0x11);
        }

        let mut frame_count: u64 = 0;
        let mut next_frame_time: Instant = Instant::now() + FRAME_TIME;
        let mut turbo: Option<u32> = None;

        loop {
            if !args.headless && !self.window.is_open() { break; }
            if let Some(max) = args.frames { if frame_count >= max { break; } }
            
            if let Some(t) = tracer.as_mut() {

                while !self.gb.bus.ppu.ready {
                    let pc = self.gb.cpu.registers.get_pc();

                    let regs = (
                        self.gb.cpu.registers.get_a(),
                        self.gb.cpu.registers.get_f(),
                        self.gb.cpu.registers.get_b(),
                        self.gb.cpu.registers.get_c(),
                        self.gb.cpu.registers.get_d(),
                        self.gb.cpu.registers.get_e(),
                        self.gb.cpu.registers.get_h(),
                        self.gb.cpu.registers.get_l(),
                        self.gb.cpu.registers.get_sp(),
                        self.gb.cpu.registers.get_pc(),
                    );

                    let pcmem = (
                        self.gb.bus.read(pc),
                        self.gb.bus.read(pc + 1),
                        self.gb.bus.read(pc + 2),
                        self.gb.bus.read(pc + 3),
                    );
                    let ppu_stuff = (self.gb.bus.ppu.regs.ly, self.gb.bus.ppu.regs.stat, self.gb.bus.ppu.regs.lcdc);

                    let ticks_before = self.gb.bus.tick_count;
                    self.gb.cpu.step(&mut self.gb.bus);
                    let m_cycles = self.gb.bus.tick_count - ticks_before;

                    let unit_mult = if self.gb.cpu.double_speed { 4 } else { 8 };
                    let cycles = m_cycles * unit_mult;
                    let total = self.gb.bus.tick_count * unit_mult;

                    t.log(regs, pcmem, ppu_stuff, cycles, total);
                }
                self.gb.bus.ppu.ready = false;

            } else {
                self.gb.emu_frame();
            }

            let frame_time = if let Some(turbo) = turbo {
                FRAME_TIME / turbo
            } else {
                FRAME_TIME
            };

            if !args.headless {
                self.window.update_with_buffer(&self.gb.bus.ppu.frame_buffer, 160, 144).unwrap();

                let now = Instant::now();
                if now < next_frame_time {
                    sleep(next_frame_time - now);
                }
                next_frame_time += frame_time;

                if Instant::now() > next_frame_time + frame_time {
                    next_frame_time = Instant::now() + frame_time;
                }
            }
            self.gb.bus.ppu.ready = false;

            while let Some(Event { event, .. }) = self.gilrs.next_event() {
            match event {
                EventType::ButtonPressed(button, _) => self.set_pad_button(button, true),
                EventType::ButtonReleased(button, _) => self.set_pad_button(button, false),
                _ => {}
            }
        }

            if let Some(script) = &script {
                // println!("applying");
                script.apply(frame_count, &mut self.gb.bus.joypad);
            } else if !args.headless {
                let keys = self.window.get_keys();

                self.gb.bus.joypad.a      = keys.contains(&Key::W)     || self.pad_state.a;
                self.gb.bus.joypad.b      = keys.contains(&Key::Q)     || self.pad_state.b;
                self.gb.bus.joypad.start  = keys.contains(&Key::Enter) || self.pad_state.start;
                self.gb.bus.joypad.select = keys.contains(&Key::E)     || self.pad_state.select;
                self.gb.bus.joypad.up     = keys.contains(&Key::Up)    || self.pad_state.up;
                self.gb.bus.joypad.down   = keys.contains(&Key::Down)  || self.pad_state.down;
                self.gb.bus.joypad.left   = keys.contains(&Key::Left)  || self.pad_state.left;
                self.gb.bus.joypad.right  = keys.contains(&Key::Right) || self.pad_state.right;

                if keys.contains(&Key::F5) {
                    self.save_state_to_disk();
                }

                if keys.contains(&Key::F9) {
                    self.load_state_from_disk();
                }

                let keys_pressed = self.window.get_keys_pressed(minifb::KeyRepeat::No);

                if keys_pressed.contains(&Key::F3) {
                    turbo = Some(turbo.map_or(2, |t| t * 2));
                }

                if keys_pressed.contains(&Key::F2) {
                    turbo = turbo.and_then(|t| {
                        let new_speed = t / 2;
                        if new_speed != 0 { Some(new_speed) } else { None }
                    });
                }
            }

            if let Some(r) = recorder.as_mut() { r.record(&self.gb.bus.joypad); }

            frame_count += 1;
        }
    }
}