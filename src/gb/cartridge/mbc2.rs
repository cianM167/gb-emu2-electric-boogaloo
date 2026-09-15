use crate::gb::cartridge::mapper::{self, Mapper};

pub struct Mbc2 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    ram_enabled: bool,
    rom_bank: u8,
    rom_bank_count: usize,
}

impl Mbc2 {
    pub fn new(rom: Vec<u8>) -> Self {
        let rom_bank_count = rom.len() / 0x4000;
        Self {
            rom,
            ram: vec![0; 512],
            ram_enabled: false,
            rom_bank: 1,
            rom_bank_count,
        }
    }

    fn rom_bank_for_high_window(&self) -> usize {
        let mut bank = self.rom_bank as usize;
        if bank == 0 { bank = 1; }

        bank % self.rom_bank_count
    }
}

impl Mapper for Mbc2 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0xFFF => self.rom[addr as usize],

            0x4000..=0x7FFF => {
                let bank = self.rom_bank_for_high_window();
                self.rom[bank * 0x4000 + (addr as usize - 0x4000)]
            }

            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    let idx = (addr as usize - 0xA000) & 0x1FF;
                    self.ram[idx] | 0xF0
                } else {
                    0x0F
                }
            }

            _ => unreachable!()
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                if addr & 0x0100 == 0 {
                    self.ram_enabled = (value & 0x0F) == 0x0A;
                } else {
                    self.rom_bank = value & 0x0F;
                }
            }

            // 0x4000..=0x5FFF => {
            //     self.bank_hi = value & 0x03;
            // }

            // 0x6000..=0x7FFF => {
            //     self.banking_mode = value & 0x01;
            // }

            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    let idx = (addr as usize - 0xA000) & 0x1FF;
                    self.ram[idx] = value & 0x0F;
                }
            }

            _ => ()
        }
    }
}