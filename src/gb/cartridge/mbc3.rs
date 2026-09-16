use crate::gb::cartridge::mapper::{self, Mapper};

pub struct Mbc3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    ram_timer_enabled: bool,
    rom_bank: u8,
    ram_bank: u8,
    latch_clock: bool,
    rom_bank_count: usize,
    ram_bank_count: usize,

    // rtc regs
    rtc_s: u8,
    rtc_m: u8,
    rtc_h: u8,
    rtc_dl: u8,
    rtc_dh: u8,

    latch_prev: u8,
}

impl Mbc3 {
    pub fn new(rom: Vec<u8>, ram_size: usize) -> Self {
        let rom_bank_count = rom.len() / 0x4000;
        Self {
            rom,
            ram: vec![0; ram_size],
            ram_timer_enabled: false,
            rom_bank: 0,
            ram_bank: 0,
            latch_clock: false,
            rom_bank_count,
            ram_bank_count: ram_size / 0x2000,

            rtc_s: 0,
            rtc_m: 0,
            rtc_h: 0,
            rtc_dl: 0,
            rtc_dh: 0,

            latch_prev: 0xFF,
        }
    }

    // fn rom_bank_for_low_window(&self) -> usize {
    //     if self.banking_mode == 1 && self.rom_bank_count > 32 {
    //         ((self.bank_hi as usize) << 5) % self.rom_bank_count
    //     } else {
    //         0
    //     }
    // }

    // fn rom_bank_for_high_window(&self) -> usize {
    //     let mut bank = self.rom_bank_low as usize;
    //     if bank == 0 { bank = 1; }

    //     if self.rom_bank_count > 32 {
    //         bank |= (self.bank_hi as usize) << 5;
    //     }
    //     bank % self.rom_bank_count
    // }

    // fn ram_bank(&self) -> usize {
    //     if self.banking_mode == 1 {
    //         (self.bank_hi as usize) % self.ram_bank_count.max(1)
    //     } else {
    //         0
    //     }
    // }
}

impl Mapper for Mbc3 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                self.rom[addr as usize]
            }

            0x4000..=0x7FFF => {
                let bank = self.rom_bank as usize % self.rom_bank_count;
                self.rom[bank * 0x4000 + (addr as usize - 0x4000)]
            }

            0xA000..=0xBFFF => {
                if self.ram_timer_enabled {
                    match self.ram_bank {
                        0x00..=0x07 => {
                            let bank = self.ram_bank as usize;
                            self.ram[bank * 0x2000 + (addr as usize - 0xA000)]
                        }

                        0x08 => self.rtc_s,
                        0x09 => self.rtc_m,
                        0x0A => self.rtc_h,
                        0x0B => self.rtc_dl,
                        0x0C => self.rtc_dh,

                        _ => 0xFF
                    }
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
                self.ram_timer_enabled = (value & 0x0F) == 0x0A;
            }

            0x2000..=0x3FFF => {
                let new = if value == 0 { 1 } else {value};// bumping to avoid selecting bank 0

                self.rom_bank = new & 0x7F;
            }

            0x4000..=0x5FFF => {
                self.ram_bank = value & 0x0F;// also decides clock register
            }

            0x6000..=0x7FFF => {// latch clock
                if self.latch_prev == 0 && value == 1{
                    self.latch_clock = !self.latch_clock;
                }

                self.latch_prev = value
            }

            0xA000..=0xBFFF => {
                if self.ram_timer_enabled {
                    match self.ram_bank {
                        0x00..=0x07 => {
                            let bank = self.ram_bank as usize;
                            self.ram[bank * 0x2000 + (addr as usize - 0xA000)] = value;
                        }

                        0x08 => self.rtc_s = value,
                        0x09 => self.rtc_m = value,
                        0x0A => self.rtc_h = value,
                        0x0B => self.rtc_dl = value,
                        0x0C => self.rtc_dh = value,

                        _ => ()
                    }
                }
            }

            _ => ()
        }
    }
}