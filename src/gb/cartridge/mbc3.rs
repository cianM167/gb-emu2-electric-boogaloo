use crate::gb::cartridge::mapper::{self, Mapper};

pub struct Mbc3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    ram_enabled: bool,
    rom_bank_low: u8,
    bank_hi: u8,
    banking_mode: u8,
    rom_bank_count: usize,
    ram_bank_count: usize,
}

impl Mbc3 {
    pub fn new(rom: Vec<u8>, ram_size: usize) -> Self {
        let rom_bank_count = rom.len() / 0x4000;
        Self {
            rom,
            ram: vec![0; ram_size],
            ram_enabled: false,
            rom_bank_low: 1,
            bank_hi: 0,
            banking_mode: 0,
            rom_bank_count,
            ram_bank_count: ram_size / 0x2000,
        }
    }

    fn rom_bank_for_low_window(&self) -> usize {
        if self.banking_mode == 1 && self.rom_bank_count > 32 {
            ((self.bank_hi as usize) << 5) % self.rom_bank_count
        } else {
            0
        }
    }

    fn rom_bank_for_high_window(&self) -> usize {
        let mut bank = self.rom_bank_low as usize;
        if bank == 0 { bank = 1; }

        if self.rom_bank_count > 32 {
            bank |= (self.bank_hi as usize) << 5;
        }
        bank % self.rom_bank_count
    }

    fn ram_bank(&self) -> usize {
        if self.banking_mode == 1 {
            (self.bank_hi as usize) % self.ram_bank_count.max(1)
        } else {
            0
        }
    }
}

impl Mapper for Mbc3 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                let bank = self.rom_bank_for_low_window();
                self.rom[bank * 0x4000 + addr as usize]
            }

            0x4000..=0x7FFF => {
                let bank = self.rom_bank_for_high_window();
                self.rom[bank * 0x4000 + (addr as usize - 0x4000)]
            }

            0xA000..=0xBFFF => {
                if self.ram_enabled && self.ram_bank_count > 0 {
                    let bank = self.ram_bank();
                    self.ram[bank * 0x2000 + (addr as usize - 0xA000)]
                } else {
                    0xFF
                }
            }

            _ => unreachable!()
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }

            0x2000..=0x3FFF => {
                self.rom_bank_low = value;
            }

            0x4000..=0x5FFF => {
                self.bank_hi = value & 0x0F;
            }

            0x6000..=0x7FFF => {
                self.banking_mode = value & 0x01;
            }

            0xA000..=0xBFFF => {
                if self.ram_enabled && self.ram_bank_count > 0 {
                    let bank = self.ram_bank();
                    self.ram[bank * 0x2000 + (addr as usize - 0xA000)] = value
                }
            }

            _ => ()
        }
    }
}