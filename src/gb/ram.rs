use std::io::Bytes;

pub struct Ram {
    memory: [u8; 65536],
}

impl Ram {
    pub fn new() -> Self {
        Ram { memory: [0; 65536] }
    }

    pub fn read(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    pub fn write(&mut self, address: u16, byte: u8) {
        self.memory[address as usize] = byte;
    }
}