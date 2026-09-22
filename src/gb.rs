use std::{fs::{self, File, OpenOptions}, io::{BufWriter, Write}, result, thread::sleep, time::{Duration, Instant}};

use crate::{Args, gb::{bus::{Bus, Joypad}, bus_state::BusState, cartridge::Cartridge, cpu::Cpu, ppu::Ppu}};
use gilrs::{Button, Event, EventType, Gilrs};
use minifb::{Key::{self, Enter}, Window, WindowOptions};
use ringbuf::{HeapProd, traits::Producer};
use serde::{Deserialize, Serialize, de};

pub mod ram;
pub mod cpu;
pub mod instructions;
pub mod registers;
pub mod bus;
pub mod cartridge;
mod ppu;
mod bus_state;
pub mod apu;

const FRAME_TIME: Duration = Duration::from_nanos(16_742_706);

static mut VBLANK_COUNT: u64 = 0;
static mut STAT_COUNT: u64 = 0;
static mut TIMER_COUNT: u64 = 0;

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
        ppu: (u8,u8,u8), 
    ) {
        let (a,f,b,c,d,e,h,l,sp,pc) = regs;
        let (m0,m1,m2,m3) = pcmem;
        let (ly,stat,lcdc) = ppu;

        writeln!(
            self.writer,
            "A:{a:02X} F:{f:02X} B:{b:02X} C:{c:02X} D:{d:02X} E:{e:02X} H:{h:02X} L:{l:02X} SP:{sp:04X} PC:{pc:04X} PCMEM:{m0:02X},{m1:02X},{m2:02X},{m3:02X} LCDC:{lcdc:02X} LY:{ly:02X}"
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

pub struct GameBoy {
    cpu: Cpu,
    bus: Bus,
    ppu: Ppu,
    window: Window,
    gilrs: Gilrs,
    pad_state: PadState,

    audio_producer: HeapProd<f32>,
    sample_rate: u32,
    cycles_per_sample: f64,
    sample_cycle_accum: f64,
}

impl GameBoy {
    pub fn new(cart: Cartridge, audio_producer: HeapProd<f32>, sample_rate: u32) -> Self {
        // let mut bus = Bus::new(cart);
        // let mut ppu = Ppu::new();

        // const BOOT_ROM_CYCLES: u32 = 23_440;

        // let mut remaining = BOOT_ROM_CYCLES;
        // while remaining > 0 {
        //     let chunk = remaining.min(4) as u8; 
        //     bus.step_timer(chunk);
        //     ppu.step(chunk, &mut bus);
        //     remaining -= chunk as u32;
        // }
        const GB_CLOCK_HZ: f64 = 4_194_304.0;
        Self {
            cpu: Cpu::new(false),
            bus: Bus::new(cart),
            ppu: Ppu::new(),
            window: Window::new("Ferro boy", 640, 576, WindowOptions::default()).unwrap(),
            gilrs: Gilrs::new().unwrap(),
            pad_state: PadState::default(),
            audio_producer,
            sample_rate,
            cycles_per_sample: GB_CLOCK_HZ / sample_rate as f64,
            sample_cycle_accum: 0.0,
        }
    }

    fn save_state(&self) {
        let state = SaveState::new(&self.cpu, &self.ppu, &self.bus);
        
        let title = &self.bus.cart.header.title;
        let path = format!("saves/{title}.stat");

        let bytes = postcard::to_allocvec(&state).expect("Serialize failed");
        std::fs::write(path, bytes).expect("ruh roh, file save failed");
        println!("State saved");
    }

    fn load_state(&mut self) {
        let title = &self.bus.cart.header.title;
        let path = &format!("saves/{title}.stat");

        if let Ok(bytes) = std::fs::read(path) {
            let state: SaveState = postcard::from_bytes(&bytes).expect("deserialize failed");

            self.cpu = state.cpu;
            self.ppu = state.ppu;
            self.bus.load_state(state.bus_state);
            println!("State loaded");

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

    pub fn run(&mut self, args: &Args) {
        let script = args.input_script.as_ref().map(|p| InputScript::load(p));
        let mut tracer = args.trace_out.as_ref().map(|p| Tracer::new(p));
        let mut recorder = args.recording.as_ref().map(|p| Recorder::new(p));
        let mut frame_count: u64 = 0;

        let mut next_frame_time: Instant = Instant::now() + FRAME_TIME;
        
        let mut last_report = Instant::now();
        let mut cycles_this_second: u32 = 0;
        let mut frames_this_second = 0;

        let mut turbo:Option<u32> = None;
        

        loop {
            if !args.headless && !self.window.is_open() { break; }
            if let Some(max) = args.frames { if frame_count >= max { break; } }

            // let now = Instant::now();
            // if now.duration_since(last_report) >= Duration::from_secs(1) {
            //     let (vblank, stat, timer) = unsafe {
            //         (VBLANK_COUNT, STAT_COUNT, TIMER_COUNT)
            //     };
            //     println!("cycles/sec: {}, frames/sec: {}, vblank count/sec: {}, stat count/sec: {}, timer count: {}", cycles_this_second, frames_this_second, vblank, stat, timer);
            //     cycles_this_second = 0;
            //     frames_this_second = 0;
            //     unsafe {
            //         VBLANK_COUNT = 0;
            //         STAT_COUNT = 0;
            //         TIMER_COUNT = 0;
            //     }
            //     last_report = now;
                    
            // }

            if let Some(t) = tracer.as_mut() {
                let pc = self.cpu.registers.get_pc();

                let regs = (
                    self.cpu.registers.get_a(),
                    self.cpu.registers.get_f(),
                    self.cpu.registers.get_b(),
                    self.cpu.registers.get_c(),
                    self.cpu.registers.get_d(),
                    self.cpu.registers.get_e(),
                    self.cpu.registers.get_h(),
                    self.cpu.registers.get_l(),
                    self.cpu.registers.get_sp(),
                    self.cpu.registers.get_pc() + 1,
                );

                let pcmem = (                   
                    self.bus.read(pc),
                    self.bus.read(pc + 1),
                    self.bus.read(pc + 2),
                    self.bus.read(pc + 3),
                );
                let ppu_stuff = (self.bus.get_ly(), self.bus.get_stat(), self.bus.get_lcdc());
                t.log(regs, pcmem, ppu_stuff);
            }

            let cycles = self.cpu.step(&mut self.bus);
            cycles_this_second += cycles as u32;

            self.bus.step_timer(cycles);
            self.ppu.step(cycles, &mut self.bus);
            self.bus.apu.step(cycles as u32);

            self.generate_audio_sample(cycles);

            if self.ppu.ready {
                let frame_time = if let Some(turbo) = turbo {
                    FRAME_TIME / turbo
                } else {
                    FRAME_TIME
                };

                if !args.headless {
                    self.window.update_with_buffer(&self.ppu.frame_buffer, 160, 144).unwrap();

                    let now = Instant::now();
                    if now < next_frame_time {
                        sleep(next_frame_time - now);
                    }
                    next_frame_time += frame_time;

                    if Instant::now() > next_frame_time + frame_time {
                        next_frame_time = Instant::now() + frame_time;
                    }

                    frames_this_second += 1;
                }
                self.ppu.ready = false;

                while let Some(Event { event, .. }) = self.gilrs.next_event() {
                match event {
                    EventType::ButtonPressed(button, _) => self.set_pad_button(button, true),
                    EventType::ButtonReleased(button, _) => self.set_pad_button(button, false),
                    _ => {}
                }
            }

                if let Some(script) = &script {
                    // println!("applying");
                    script.apply(frame_count, &mut self.bus.joypad);
                } else if !args.headless {
                    let keys = self.window.get_keys();

                    self.bus.joypad.a      = keys.contains(&Key::W)     || self.pad_state.a;
                    self.bus.joypad.b      = keys.contains(&Key::Q)     || self.pad_state.b;
                    self.bus.joypad.start  = keys.contains(&Key::Enter) || self.pad_state.start;
                    self.bus.joypad.select = keys.contains(&Key::E)     || self.pad_state.select;
                    self.bus.joypad.up     = keys.contains(&Key::Up)    || self.pad_state.up;
                    self.bus.joypad.down   = keys.contains(&Key::Down)  || self.pad_state.down;
                    self.bus.joypad.left   = keys.contains(&Key::Left)  || self.pad_state.left;
                    self.bus.joypad.right  = keys.contains(&Key::Right) || self.pad_state.right;

                    if keys.contains(&Key::F5) {
                        self.save_state();
                    }

                    if keys.contains(&Key::F9) {
                        self.load_state();
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

                if let Some(r) = recorder.as_mut() { r.record(&self.bus.joypad); }

                frame_count += 1;
            }
        }

        if let Some(t) = tracer.as_mut() { t.flush(); }
        if let Some(r) = recorder.as_mut() { r.flush(); }

        // while self.window.is_open() {

        //     // if self.bus.get_frame() != previous_frame {
        //     //     if self.bus.get_frame() == 1 {
        //     //         start = std::time::Instant::now();
        //     //     }

        //     //     if self.bus.get_frame() == 60 {
        //     //         println!("60 frames took {:?}", start.elapsed());
        //     //     }
        //     // }

        //     let old_regs:(u8, u8, u8, u8, u8, u8, u8, u8, u16, u16) = (// for debugging
        //         self.cpu.registers.get_a(),
        //         self.cpu.registers.get_f(),
        //         self.cpu.registers.get_b(),
        //         self.cpu.registers.get_c(),
        //         self.cpu.registers.get_d(),
        //         self.cpu.registers.get_e(),
        //         self.cpu.registers.get_h(),
        //         self.cpu.registers.get_l(),
        //         self.cpu.registers.get_sp(),
        //         self.cpu.registers.get_pc(),
        //     );

        //     let ppu_stuff = (
        //         self.bus.get_ly(),
        //         self.bus.get_stat(),
        //         self.bus.get_lcdc(),
        //     );

        //     if self.cpu.registers.get_pc() == 0x022A {
        //         println!(":3");
        //     }

        //     let cycles = self.cpu.step(&mut self.bus);

        //     self.bus.step_timer(cycles);
        //     self.ppu.step(cycles, &mut self.bus);

        //     if self.ppu.ready {
        //         self.window.update_with_buffer(&self.ppu.frame_buffer, 160, 144).unwrap();
        //         self.ppu.ready = false;

        //         let now = Instant::now();
        //         if now < next_frame_time {
        //             sleep(next_frame_time - now);
        //         }
        //         next_frame_time += FRAME_TIME;

        //         if Instant::now() > next_frame_time + FRAME_TIME {
        //             next_frame_time = Instant::now() + FRAME_TIME;
        //         }
        //     }

        //     // self.write_to_log(old_regs, dots, ppu_stuff);

        //     let keys = self.window.get_keys();
        //     self.bus.joypad.a      = keys.contains(&Key::X);
        //     self.bus.joypad.b      = keys.contains(&Key::Z);
        //     self.bus.joypad.start  = keys.contains(&Key::Enter);
        //     self.bus.joypad.select = keys.contains(&Key::C);
        //     self.bus.joypad.up     = keys.contains(&Key::Up);
        //     self.bus.joypad.down   = keys.contains(&Key::Down);
        //     self.bus.joypad.left   = keys.contains(&Key::Left);
        //     self.bus.joypad.right  = keys.contains(&Key::Right);
        // }
    }

    fn write_to_log(&self, old_regs: (u8, u8, u8, u8, u8, u8, u8, u8, u16, u16), dots: u16, ppu_stuff: (u8, u8, u8)) {// super brittle is temporary :)
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("log.txt");

        let (a, f, b, c, d, e, h, l, sp, pc) = old_regs;

        let (pcmem0, pcmem1, pcmem2, pcmem3) = (
            self.bus.read(pc),
            self.bus.read(pc + 1),
            self.bus.read(pc + 2),
            self.bus.read(pc + 3),
        );

        let (ly, stat, lcdc) = ppu_stuff;

        let new_pc = pc + 1;

        let line = format!("A:{a:02X} F:{f:02X} B:{b:02X} C:{c:02X} D:{d:02X} E:{e:02X} H:{h:02X} L:{l:02X} SP:{sp:04X} PC:{new_pc:04X} PCMEM:{pcmem0:02X},{pcmem1:02X},{pcmem2:02X},{pcmem3:02X} LY:{ly:02X} STAT:{stat:02X} LCDC:{lcdc:02X} DOT:{dots:03}\n");

        // LY:{ly:02X} STAT:{stat:02X} LCDC:{lcdc:02X} DOT:{dots:03}

        file.unwrap().write_all(line.as_bytes()).unwrap();
    }

    fn generate_audio_sample(&mut self, cycles: u8) {
        self.sample_cycle_accum += cycles as f64;

        while self.sample_cycle_accum >= self.cycles_per_sample {
            self.sample_cycle_accum -= self.cycles_per_sample;

            let sample = self.bus.apu.mix_output();
            let err = self.audio_producer.try_push(sample);// ignore failure lol

            // if sample == 0f32 {
            //     println!("audio error: {:?}, sample: {}, apu ch2: {:?}", err, sample, self.bus.apu.ch2);
            // }
        }
    }
}