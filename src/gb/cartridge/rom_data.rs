use super::mapper::Mapper;

pub struct RomData {
    rom: Vec<u8>,
}

impl RomData {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            rom
        }
    }
}

impl Mapper for RomData {
    fn read(&self, addr: u16) -> u8 {
        self.rom[addr as usize]
    }

    fn write(&mut self, addr: u16, value: u8) {

    }
}