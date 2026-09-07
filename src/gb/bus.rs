use crate::gb::cartridge::Cartridge;

pub struct Bus {
    cart: Cartridge,
    vram: [u8; 0x2000],
    wram: [u8; 0x2000],
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            vram: [0; 0x2000],
            wram: [0; 0x2000]
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
                self.vram[(addr - 0xC000) as usize] = value
            }

            _ => {
                panic!("Unknown read: shitting the bed")
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