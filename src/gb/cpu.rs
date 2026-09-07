use std::{fs::{self, OpenOptions}, io::Write};

use crate::gb::{bus::Bus, cartridge::Cartridge, instructions::{self, Instruction, opcodes}, registers::Registers};

pub struct Cpu {
    pub registers: Registers,
    debug: bool,
    pub ime: bool,
    pub opcode: u8,
    pub cycles: u64,
}

impl Cpu {
    pub fn new(debug: bool) -> Self {
        if debug {
            fs::write("log.txt", "");
        }

        Self {
            registers: Registers::new(),
            ime: false,
            opcode: 0,
            cycles: 0,
            debug
        }
    }

    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        let mut cycles: u8 = 0;

        if self.debug {
            self.write_to_log(bus);
        }

        let opcode = bus.read(self.registers.get_pc());
        // println!("opcode read: {:#02X} pc: {:#02X}", opcode, self.registers.get_pc());

        let instr = opcodes()[opcode as usize].expect(&*format!("Unknown opcode {:#02X}", opcode));

        cycles += (instr.execute)(&instr, self, bus);// execute instruction

        cycles
    }

    fn write_to_log(&self, bus: &mut Bus) {// super brittle is temporary :)
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("log.txt");

        let (a, f, b, c, d, e, h, l, sp, pc) = (
            self.registers.get_a(),
            self.registers.get_f(),
            self.registers.get_b(),
            self.registers.get_c(),
            self.registers.get_d(),
            self.registers.get_e(),
            self.registers.get_h(),
            self.registers.get_l(),
            self.registers.get_sp(),
            self.registers.get_pc(),
        );

        let (pcmem0, pcmem1, pcmem2, pcmem3) = (
            bus.read(pc),
            bus.read(pc + 1),
            bus.read(pc + 2),
            bus.read(pc + 3),
        );

        let line = format!("A:{a:02X} F:{f:02X} B:{b:02X} C:{c:02X} D:{d:02X} E:{e:02X} H:{h:02X} L:{l:02X} SP:{sp:04X} PC:{pc:04X} PCMEM:{pcmem0:02X},{pcmem1:02X},{pcmem2:02X},{pcmem3:02X}\n");

        file.unwrap().write_all(line.as_bytes()).unwrap();
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

// fn flags_str(f: u8) -> String {
//     let z = if f & 0x80 != 0 { 'Z' } else { '-' };
//     let n = if f & 0x40 != 0 { 'N' } else { '-' };
//     let h = if f & 0x20 != 0 { 'H' } else { '-' };
//     let c = if f & 0x10 != 0 { 'C' } else { '-' };
//     format!("{z}{n}{h}{c}")
// }