use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;


use crate::gb::{apu::Apu, bus_state::BusState, cartridge::Cartridge};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Joypad {
    pub a: bool, pub b: bool, pub select: bool, pub start: bool,
    pub up: bool, pub down: bool, pub left: bool, pub right: bool,
}

// #[derive(Serialize, Deserialize)]
pub struct Bus {
    pub cart: Cartridge,
    apu: Apu,

    ie: u8,
    iflag: u8,

    ly: u8,
    lyc: u8,
    stat: u8,

    div: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    timer_acc: u16,

    vram: [u8; 0x2000],
    wram: [u8; 0x2000],
    oam: [u8; 0xA0],
    hram: [u8; 0x7F],

    wave_ram: [u8; 0x10],

    joyp: u8,

    lcdc: u8,
    bgp: u8,
    opb0: u8,
    opb1: u8,
    scx: u8,
    scy : u8,
    wx: u8,
    wy: u8,

    serial_data: u8,
    rp: u8,

    // inaccurate bullshit
    dma_source: u8,
    pub joypad: Joypad,
    frame: u32,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            apu: Apu::default(),

            ie: 0,
            iflag: 0xE1,

            ly: 0,
            lyc: 0,
            stat: 0x80,

            div: 0x0000,
            tima: 0,
            tma: 0,
            tac: 0,
            timer_acc: 0,

            vram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0xA0],
            hram: [0xFF; 0x7F],

            wave_ram: [0; 0x10],

            joyp: 0x3F,

            lcdc: 0x00,
            bgp: 0xFC,
            opb0: 0xFF,
            opb1: 0xFF,
            scx: 0,
            scy: 0,
            wx: 0,
            wy: 0,

            serial_data: 0,
            rp: 0,

            dma_source: 0,
            joypad: Joypad::default(),
            frame: 0,
        }
    }

    pub fn get_joyp(&self) -> u8 {
        self.joyp
    }

    pub fn inc_frame(&mut self) {
        self.frame += 1
    }

    pub fn get_frame(&self) -> u32 {
        self.frame
    }

    pub fn get_scy(&self) -> u8 {
        self.scy
    }

    pub fn get_scx(&self) -> u8 {
        self.scx
    }

    pub fn get_wy(&self) -> u8 {
        self.wy
    }

    pub fn get_wx(&self) -> u8 {
        self.wx
    }

    pub fn get_bgp(&self) -> u8 {
        self.bgp
    }

    pub fn get_lcdc(&self) -> u8 {
        self.lcdc
    }

    pub fn get_ly(&self) -> u8 {
        self.ly
    }

    pub fn set_ly(&mut self , value: u8) {
        self.ly = value
    }

    pub fn get_lyc(&self) -> u8 {
        self.lyc
    }

    pub fn set_lyc(&mut self , value: u8) {
        self.lyc = value
    }

    pub fn get_ie(&self) -> u8 {
        self.ie
    }

    pub fn get_iflag(&self) -> u8 {
        self.iflag
    }

    pub fn get_stat(&self) -> u8 {
        self.stat
    }

    pub fn set_stat(&mut self, value: u8) {
        self.stat = value
    }

    pub fn get_tac(&self) -> u8 {
        self.tac
    }

    pub fn get_tima(&self) -> u8 {
        self.tima
    }

    pub fn set_tima(&mut self, value: u8) {
        self.tima = value
    }

    pub fn get_tma(&self) -> u8 {
        self.tma
    }

    pub fn set_div(&mut self, value: u16) {
        self.div = value
    }

    pub fn get_div(&self) -> u16 {
        self.div
    }

    pub fn read_div(&self) -> u8 {
        (self.div >> 8) as u8
    }

    pub fn request_interrupt(&mut self, bit: u8) {//could be rewritten to use consts like the flags
        self.iflag |= 1 << bit;
    }

    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram[(addr - 0x8000) as usize]
    }

    pub fn read_oam(&self, addr: u16) -> u8 {
        self.oam[(addr - 0xFE00) as usize]
    }

    pub fn get_obp0(&self) -> u8 {
        self.opb0
    }

    pub fn get_obp1(&self) -> u8 {
        self.opb1
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.cart.mapper.read(addr),// cartridge address range

            0xA000..=0xBFFF => {//external ram on the cartridge
                self.cart.mapper.read(addr)
            }

            0x8000..=0x9FFF => {
                self.vram[(addr - 0x8000) as usize]
            }

            0xC000..=0xDFFF => {
                self.wram[(addr - 0xC000) as usize]
            }

            0xE000..=0xFDFF => {// leave commented out if can
                self.wram[(addr - 0xE000) as usize]
            }

            0xFF00 => {
                let mut result = (self.joyp & 0xF0) | 0x0F; // start with "nothing pressed" (all 1s) in low nibble

                if self.joyp & 0b0001_0000 == 0 { // bit 4 low = d-pad selected
                    if self.joypad.right { 
                        // print!(":))))");
                        result &= !0b0001; 
                    }
                    if self.joypad.left  { result &= !0b0010; }
                    if self.joypad.up    { result &= !0b0100; }
                    if self.joypad.down  { result &= !0b1000; }
                }
                if self.joyp & 0b0010_0000 == 0 { // bit 5 low = buttons selected
                    if self.joypad.a      { result &= !0b0001; }
                    if self.joypad.b      { result &= !0b0010; }
                    if self.joypad.select { result &= !0b0100; }
                    if self.joypad.start  { result &= !0b1000; }
                }

                // println!(
                //     "JOYP select:{:08b} -> result:{:08b} (r={} l={} a={} b={})",
                //     self.joyp, result, self.joypad.right, self.joypad.left, self.joypad.a, self.joypad.b
                // );

                // println!("button press checked, bits: {result:08b}");

                result
            }

            0xFF01 => self.serial_data,

            0xFF04 => self.read_div(),
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac | 0xF8,

            0xFF0F => self.iflag,

            0xFF10 => self.apu.ch1.sweep,
            0xFF11 => self.apu.ch1.duty_len,
            0xFF12 => self.apu.ch1.envelope,
            0xFF13 => unreachable!(),
            0xFF14 => self.apu.ch1.freq_hi_ctrl & 0x40,

            0xFF16 => self.apu.ch2.duty_len,
            0xFF17 => self.apu.ch2.envelope,
            0xFF18 => unreachable!(),
            0xFF19 => self.apu.ch2.freq_hi_ctrl & 0x40,

            0xFF1A => self.apu.ch3.dac,
            0xFF1B => unreachable!(),
            0xFF1C => self.apu.ch3.envelope,
            0xFF1D => unreachable!(),
            0xFF1E => self.apu.ch3.freq_hi_ctrl & 0x40,

            0xFF20 => self.apu.ch4.duty_len,
            0xFF21 => self.apu.ch4.envelope,
            0xFF22 => self.apu.ch4.freq_lo,
            0xFF23 => self.apu.ch4.freq_hi_ctrl & 0x40,

            0xFF24 => self.apu.nr50,
            0xFF25 => self.apu.nr51,
            0xFF26 => self.apu.nr52,

            0xFF30..=0xFF3F => self.wave_ram[addr as usize - 0xFF30],

            0xFF40 => {
                // println!("LCDC UNFINISHED");
                self.lcdc
            },
            0xFF41 => (self.stat & 0x7F) | 0x80,
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => {
                // self.ly,//needs to be hacked to 0x90 to pass some tests
                self.ly
            },
            0xFF45 => self.lyc,

            0xFF47 => self.bgp,
            0xFF48 => self.opb0,
            0xFF49 => self.opb1,

            0xFF4A => self.wy,
            0xFF4B => self.wx,

            0xFF80..=0xFFFE => {// hram
                self.hram[(addr - 0xFF80) as usize]
            }

            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],

            0xFEA0..=0xFEFF => 0xFF,

            0xFF46 => self.dma_source,// probably wrong

            0xFF75..=0xFF7F => panic!(),

            0xFFFF => self.ie, //| 0xE0,

            _ => {
                panic!("Unknown read: {addr:04X} shitting the bed")
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => self.cart.mapper.write(addr, value),// cartridge address range ignored by rom

            0xA000..=0xBFFF => {//external ram on the cartridge
                self.cart.mapper.write(addr, value);
            }

            0x8000..=0x9FFF => {
                self.vram[(addr - 0x8000) as usize] = value
            }

            0xC000..=0xDFFF => {
                self.wram[(addr - 0xC000) as usize] = value
            }

            // 0xE000..=0xFDFF => {
            //     self.wram[(addr - 0xE000) as usize] = value
            // }

            0xFF00 => {
                // println!("JOYP write: {:08b}", value);
                self.joyp = value & 0xF0
            },

            0xFF01 => self.serial_data = value,
            0xFF02 => {
                if value == 0x81 {
                    // Blargg convention: print immediately
                    let ch = self.serial_data as char;
                    print!("{}", ch);
                    use std::io::Write;
                    std::io::stdout().flush().unwrap();
                }
            }

            0xFF04 => {self.div = 0; self.timer_acc = 0; }
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac = value & 0x07,

            0xFF0F => self.iflag = value,

            0xFF10 => self.apu.ch1.sweep = value,
            0xFF11 => self.apu.ch1.duty_len = value & 0xC0,
            0xFF12 => self.apu.ch1.envelope = value,
            0xFF13 => self.apu.ch1.freq_lo = value,
            0xFF14 => self.apu.ch1.freq_hi_ctrl = value,

            0xFF16 => self.apu.ch2.duty_len = value,
            0xFF17 => self.apu.ch2.envelope = value & 0xC0,
            0xFF18 => self.apu.ch2.freq_lo = value,
            0xFF19 => self.apu.ch2.freq_hi_ctrl = value,

            0xFF1A => self.apu.ch3.dac = value,
            0xFF1B => self.apu.ch3.duty_len = value,
            0xFF1C => self.apu.ch3.envelope = value,
            0xFF1D => self.apu.ch3.freq_lo = value,
            0xFF1E => self.apu.ch3.freq_hi_ctrl = value,

            0xFF20 => self.apu.ch4.duty_len = value,
            0xFF21 => self.apu.ch4.envelope = value,
            0xFF22 => self.apu.ch4.freq_lo = value,
            0xFF23 => self.apu.ch4.freq_hi_ctrl = value,

            0xFF24 => self.apu.nr50 = value,
            0xFF25 => self.apu.nr51 = value,
            0xFF26 => self.apu.nr52 = value & 0x80,

            0xFF30..=0xFF3F => self.wave_ram[addr as usize - 0xFF30] = value,

            0xFF40 => {
                // println!("LCDC UNFINISHED");
                self.lcdc = value
            },
            0xFF41 => {
                // println!(":(");
                self.stat = value | 0x80
            },
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => (),
            0xFF45 => self.lyc = value,

            0xFF47 => self.bgp = value,
            0xFF48 => self.opb0 = value,
            0xFF49 => self.opb1 = value,

            0xFF56 => self.rp = value,// ir port

            0xFF4A => self.wy = value,
            0xFF4B => self.wx = value,


            0xFF80..=0xFFFE => {// hram
                self.hram[(addr - 0xFF80) as usize] = value;
            }

            0xFE00..=0xFE9F => self.oam[(addr -0xFE00) as usize] = value,

            0xFEA0..=0xFEFF => {
                // unusable ignore
            }

            0xFF46 => {
                // handle with enum later for accuracy
                self.dma_source = value;

                let src_base = (value as u16) << 8;
                for i in 0..0xA0u16 {
                    let data = self.read(src_base + i);
                    self.oam[i as usize] = data;
                }
            }

            0xFF75..=0xFF7F => (),// unused
            
            0xFFFF => self.ie = value,

            addr => {
                panic!("Unknown write: {addr:04X} shitting the bed")
            }
        }
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.read(addr)as u16;
        let hi = self.read(addr+1) as u16;
        (hi << 8) | lo
    }

    pub fn write_u16(&mut self, addr: u16, value: u16) {
        self.write(addr, (value & 0xFF) as u8);
        self.write(addr+1, (value >> 8) as u8);
    }

    pub fn step_timer(&mut self, cycles: u8) {
        self.div = self.div.wrapping_add(cycles as u16);

        if self.tac & 0x04 == 0 {
            return;
        }

        let threshold: u16 = match self.tac & 0x03 {
            0 => 1024,
            1 => 16,
            2 => 64,
            3 => 256,
            _ => unreachable!(),
        };

        self.timer_acc += cycles as u16;
        while self.timer_acc >= threshold {
            self.timer_acc -= threshold;
            let (new_tima, overflowed) = self.tima.overflowing_add(1);
            if overflowed {
                self.tima = self.tma;
                self.iflag |= 0x04;
            } else {
                self.tima = new_tima;
            }
        }
    }

    pub fn to_state(&self) -> BusState {
        BusState {
            vram: self.vram.to_vec(),
            wram: self.wram.to_vec(),
            hram: self.hram.to_vec(),
            oam: self.oam.to_vec(),
            joyp: self.joyp,
            iflag: self.iflag,
            ie: self.ie,
            div: self.div,
            tima: self.tima,
            tma: self.tma,
            tac: self.tac,
            lcdc: self.lcdc,
            stat: self.stat,
            scy: self.scy,
            scx: self.scx,
            ly: self.ly,
            bgp: self.bgp,
            opb0: self.opb0,
            opb1: self.opb1,
            wy: self.wy,
            wx: self.wx,
            dma_source: self.dma_source,
            joypad: self.joypad.clone(), 
        }
    }

    pub fn load_state(&mut self, state: BusState) {
        self.vram.copy_from_slice(&state.vram);
        self.wram.copy_from_slice(&state.wram);
        self.hram.copy_from_slice(&state.hram);
        self.oam.copy_from_slice(&state.oam);
        self.joyp = state.joyp;
        self.iflag = state.iflag;
        self.ie = state.ie;
        self.div = state.div;
        self.tima = state.tima;
        self.tma = state.tma;
        self.tac = state.tac;
        self.lcdc = state.lcdc;
        self.stat = state.stat;
        self.scy = state.scy;
        self.scx = state.scx;
        self.ly = state.ly;
        self.bgp = state.bgp;
        self.opb0 = state.opb0;
        self.opb1 = state.opb1;
        self.wy = state.wy;
        self.wx = state.wx;
        self.dma_source = state.dma_source;
        self.joypad = state.joypad;
    }
}