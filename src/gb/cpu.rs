use crate::gb::{bus::Bus, cartridge::Cartridge, instructions::{self, Instruction, opcodes}, registers::Registers};

pub struct Cpu {
    pub registers: Registers,
    pub ime: bool,
    pub opcode: u8,
    pub cycles: u64,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            registers: Registers::new(),
            ime: false,
            opcode: 0,
            cycles: 0
        }
    }

    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        let mut cycles: u8 = 0;

        let opcode = bus.read(self.registers.get_pc());
        println!("opcode read: {:#x} pc: {:#x}", opcode, self.registers.get_pc());

        let instr = opcodes()[opcode as usize].expect(&*format!("Unknown opcode {:#x}", opcode));

        cycles += (instr.execute)(&instr, self, bus);// execute instruction

        cycles
    }

    pub fn fetch_next(&mut self) -> u8 {
        todo!()
    }

    pub fn decode(opcode: u8, cb_opcode: bool) -> Option<Instruction> {
        todo!()
    }

    pub fn execute_next(&mut self) -> u64 {
        todo!()
    }
}