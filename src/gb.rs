use std::{fs::{self, OpenOptions}, io::Write, thread::sleep, time::{Duration, Instant}};

use crate::gb::{bus::Bus, cartridge::Cartridge, cpu::Cpu, ppu::Ppu};
use minifb::{Key::{self, Enter}, Window, WindowOptions};

pub mod ram;
pub mod cpu;
pub mod instructions;
pub mod registers;
pub mod bus;
pub mod cartridge;
mod ppu;

const FRAME_TIME: Duration = Duration::from_nanos(16_742_706);

pub struct GameBoy {
    cpu: Cpu,
    bus: Bus,
    ppu: Ppu,
    window: Window,
}

impl GameBoy {
    pub fn new(cart: Cartridge) -> Self {
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

        Self {
            cpu: Cpu::new(false),
            bus: Bus::new(cart),
            ppu: Ppu::new(),
            window: Window::new("GB Emulator", 640, 576, WindowOptions::default()).unwrap(),
        }
    }

    pub fn run(&mut self) {
        // if self.cpu.debug {
        //     fs::write("log.txt", "");
        // }

        let mut dots: u16 = 0;

        let mut previous_frame = 0;

        let mut next_frame_time: Instant = Instant::now() + FRAME_TIME;

        while self.window.is_open() {

            // if self.bus.get_frame() != previous_frame {
            //     if self.bus.get_frame() == 1 {
            //         start = std::time::Instant::now();
            //     }

            //     if self.bus.get_frame() == 60 {
            //         println!("60 frames took {:?}", start.elapsed());
            //     }
            // }

            let old_regs:(u8, u8, u8, u8, u8, u8, u8, u8, u16, u16) = (// for debugging
                self.cpu.registers.get_a(),
                self.cpu.registers.get_f(),
                self.cpu.registers.get_b(),
                self.cpu.registers.get_c(),
                self.cpu.registers.get_d(),
                self.cpu.registers.get_e(),
                self.cpu.registers.get_h(),
                self.cpu.registers.get_l(),
                self.cpu.registers.get_sp(),
                self.cpu.registers.get_pc(),
            );

            let ppu_stuff = (
                self.bus.get_ly(),
                self.bus.get_stat(),
                self.bus.get_lcdc(),
            );

            if self.cpu.registers.get_pc() == 0x022A {
                println!(":3");
            }

            let cycles = self.cpu.step(&mut self.bus);

            self.bus.step_timer(cycles);
            self.ppu.step(cycles, &mut self.bus);

            if self.ppu.ready {
                self.window.update_with_buffer(&self.ppu.frame_buffer, 160, 144).unwrap();
                self.ppu.ready = false;

                let now = Instant::now();
                if now < next_frame_time {
                    sleep(next_frame_time - now);
                }
                next_frame_time += FRAME_TIME;

                if Instant::now() > next_frame_time + FRAME_TIME {
                    next_frame_time = Instant::now() + FRAME_TIME;
                }
            }

            // self.write_to_log(old_regs, dots, ppu_stuff);

            dots += cycles as u16;

            if dots >= 456 {
                dots -= 456;
            }

            let keys = self.window.get_keys();
            self.bus.joypad.a      = keys.contains(&Key::X);
            self.bus.joypad.b      = keys.contains(&Key::Z);
            self.bus.joypad.start  = keys.contains(&Key::Enter);
            self.bus.joypad.select = keys.contains(&Key::C);
            self.bus.joypad.up     = keys.contains(&Key::Up);
            self.bus.joypad.down   = keys.contains(&Key::Down);
            self.bus.joypad.left   = keys.contains(&Key::Left);
            self.bus.joypad.right  = keys.contains(&Key::Right);
        }
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
}