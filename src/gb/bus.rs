use crate::gb::cartridge::Cartridge;

#[derive(Debug, Default)]
pub struct Joypad {
    pub a: bool, pub b: bool, pub select: bool, pub start: bool,
    pub up: bool, pub down: bool, pub left: bool, pub right: bool,
}

pub struct Bus {
    cart: Cartridge,

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

    // inaccurate bullshit
    dma_source: u8,
    pub joypad: Joypad,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,

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

            joyp: 0,

            lcdc: 0x00,
            bgp: 0xFC,
            opb0: 0xFF,
            opb1: 0xFF,
            scx: 0,
            scy: 0,
            wx: 0,
            wy: 0,

            serial_data: 0,

            dma_source: 0,
            joypad: Joypad::default(),
        }
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
                todo!()
            }

            0x8000..=0x9FFF => {
                self.vram[(addr - 0x8000) as usize]
            }

            0xC000..=0xDFFF => {
                self.wram[(addr - 0xC000) as usize]
            }

            0xFF00 => {
                let mut result = self.joyp & 0xF0 | 0x0F; // start with "nothing pressed" (all 1s) in low nibble

                if self.joyp & 0b0001_0000 == 0 { // bit 4 low = d-pad selected
                    if self.joypad.right { result &= !0b0001; }
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

                println!("button press checked, bits: {result:08b}");

                result
            }

            0xFF01 => self.serial_data,

            0xFF04 => self.read_div(),
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac | 0xF8,

            0xFF0F => self.iflag,

            0xFF26 => 0x00,

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

            0xFFFF => self.ie | 0xE0,

            _ => {
                panic!("Unknown read: {addr:04X} shitting the bed")
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7444 => self.cart.mapper.write(addr, value),// cartridge address range ignored by rom

            0xA000..=0xBFFF => {//external ram on the cartridge
                
            }

            0x8000..=0x9FFF => {
                self.vram[(addr - 0x8000) as usize] = value
            }

            0xC000..=0xDFFF => {
                self.wram[(addr - 0xC000) as usize] = value
            }

            0xFF00 => self.joyp = value & 0xF0,

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

            0xFF10..=0xFF26 => (println!("WARNING AUDIO IS UNFINISHED AND MAY CAUSE ERRORS")),

            0xFF40 => {
                // println!("LCDC UNFINISHED");
                self.lcdc = value
            },
            0xFF41 => self.stat = value,
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => (),

            0xFF47 => self.bgp = value,
            0xFF48 => self.opb0 = value,
            0xFF49 => self.opb1 = value,

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
}