use crate::gb::cartridge::Cartridge;

pub struct Bus {
    cart: Cartridge,

    ie: u8,
    iflag: u8,


    vram: [u8; 0x2000],
    wram: [u8; 0x2000],

    tac: u8,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,

            ie: 0,
            iflag: 0xE1,

            vram: [0; 0x2000],
            wram: [0; 0x2000],

            tac: 0,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7444 => self.cart.mapper.read(addr),// cartridge address range

            0xA000..=0xBFFF => {//external ram on the cartridge
                todo!()
            }

            0x8000..=0x9FFF => {
                self.vram[(addr - 0x8000) as usize]
            }

            0xC000..=0xDFFF => {
                self.wram[(addr - 0xC000) as usize]
            }

            0xFF07 => self.tac | 0xF8,

            0xFF0F => self.iflag | 0xE0,

            0xFF26 => 0x00,

            0xFFFF => self.ie | 0xE0,

            _ => {
                panic!("Unknown write :( shitting the bed")
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

            0xFF07 => self.tac = value & 0x07,

            0xFF0F => self.iflag = value | 0xE0,

            0xFF10..=0xFF26 => (println!("WARNING AUDIO IS UNFINISHED AND MAY CAUSE ERRORS")),
            
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