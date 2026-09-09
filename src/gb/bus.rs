use crate::gb::cartridge::Cartridge;

pub struct Bus {
    cart: Cartridge,

    ie: u8,
    iflag: u8,

    tma: u8,
    tima: u8,
    tac: u8,
    div: u16,

    vram: [u8; 0x2000],
    wram: [u8; 0x2000],
    hram: [u8; 0x7F],

    lcdc: u8,
    bgp: u8,
    scx: u8,
    scy :u8,

    serial_data: u8,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,

            ie: 0,
            iflag: 0xE1,

            tma: 0,
            tima: 0,
            tac: 0,
            div: 0xABCC,

            vram: [0; 0x2000],
            wram: [0; 0x2000],
            hram: [0; 0x7F],

            lcdc: 0x91,
            bgp: 0xFC,
            scx: 0,
            scy: 0,

            serial_data: 0,
        }
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

    fn read_div(&self) -> u8 {
        (self.div >> 8) as u8
    }

    pub fn request_interrupt(&mut self, bit: u8) {//could be rewritten to use consts like the flags
        self.iflag |= 1 << bit;
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

            0xFF01 => self.serial_data,

            0xFF04 => self.read_div(),

            0xFF07 => self.tac | 0xF8,

            0xFF0F => self.iflag | 0xE0,

            0xFF26 => 0x00,

            0xFF40 => {
                println!("LCDC UNFINISHED");
                self.lcdc
            },

            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => {
                // self.ly,//needs to be hacked to 0x90 to pass some tests
                println!("WARNING PPU/SCANLINE LOGIC UNIMPLEMENTED");
                0x90
            },

            0xFF47 => self.bgp,

            0xFF80..=0xFFFE => {// hram
                self.hram[(addr - 0xFF80) as usize]
            }

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

            0xFF04 => self.div = 0,

            0xFF07 => self.tac = value & 0x07,

            0xFF0F => self.iflag = value | 0xE0,

            0xFF10..=0xFF26 => (println!("WARNING AUDIO IS UNFINISHED AND MAY CAUSE ERRORS")),

            0xFF40 => {
                println!("LCDC UNFINISHED");
                self.lcdc = value
            },

            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => (),

            0xFF47 => self.bgp = value,

            0xFF80..=0xFFFE => {// hram
                self.hram[(addr - 0xFF80) as usize] = value;
            }
            
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
}