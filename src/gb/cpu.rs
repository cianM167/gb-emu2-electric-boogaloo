use std::{fs::{self, OpenOptions}, io::Write};

use serde::{Deserialize, Serialize};

use crate::gb::{bus::{self, Bus}, cartridge::Cartridge, instructions::{self, Instruction, opcodes}, registers::Registers};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Cpu {
    pub registers: Registers,
    pub debug: bool,
    pub ime: bool,
    pub enable_ime_next: bool,
    pub halted: bool,
    pub halt_bug: bool,

    pub opcode: u8,
    pub cycles: u64,
}

impl Cpu {
    pub fn new(debug: bool) -> Self {
        // if debug {
        //     fs::write("log.txt", "");
        // }

        Self {
            registers: Registers::new(),
            ime: false,
            enable_ime_next: false,
            halted: false,
            halt_bug: false,
            opcode: 0,
            cycles: 0,
            debug
        }
    }

    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        // if self.debug {
        //     self.write_to_log(bus);
        // }

        let mut cycles = self.handle_interrupts(bus);
        // if cycles > 0 {
        //     return cycles;// to align with same boy logs
        // }

        if self.halted {
            let pending = bus.get_ie() & bus.get_iflag();

            if pending != 0 {
                cycles += 4;

                if !self.ime {
                    self.halt_bug = true;
                }

                self.halted = false;
                return cycles;
            } else {
                cycles +=4;
                return cycles;
            }
        }

        let opcode = bus.read(self.registers.get_pc());
        // println!("opcode read: {:#02X} pc: {:#02X}", opcode, self.registers.get_pc());

        let instr = opcodes()[opcode as usize].expect(&*format!("Unknown opcode {:#02X}", opcode));

        cycles += (instr.execute)(&instr, self, bus);// execute instruction

        if self.enable_ime_next {
            self.ime = true;
            self.enable_ime_next = false;
        }

        cycles
    }

    pub fn handle_interrupts(&mut self, bus: &mut Bus) -> u8 {
        if !self.ime {
            return 0;
        }

        let ie = bus.read(0xFFFF);
        let mut iflag = bus.read(0xFF0F);
        let log_iflag = iflag;

        

        let pending = ie & iflag;
        // eprintln!("ie={ie:08b} iflag={iflag:08b} pending={pending:02X} pc={:04X}", self.registers.get_pc());
        if pending == 0 {
            return 0;
        }

        for i in 0..5 {
            if pending & (1 << i) != 0 {
                iflag &= !(1 << i);
                bus.write(0xFF0F, iflag);

                self.ime = false;

                self.registers.set_sp(self.registers.get_sp().wrapping_sub(2));
                bus.write_u16(self.registers.get_sp(), self.registers.get_pc());

                let pc = match i {
                    0 => {// vblank
                        // println!(
                        //     "FRAME {} | PC:{:04X} IME:{} IE:{:02X} IF:{:02X} JOYP:{:02X} DIV:{:02X} TIMA:{:02X}",
                        //     bus.get_frame(), self.registers.get_pc(), true, ie, log_iflag, bus.get_joyp(), bus.get_div(), bus.get_tima()
                        // );
                        0x0040
                    },
                    1 => 0x0048,// lcd
                    2 => 0x0050,// timer
                    3 => 0x0058,// serial
                    4 => 0x0060,// joypad
                    _ => unreachable!(),
                };

                self.registers.set_pc(pc);

                return 20;
            }
        }

        return 0;
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

        let (ly, stat, lcdc, div) = (
            bus.get_ly(),
            bus.get_stat(),
            bus.get_lcdc(),
            bus.read_div(),
        );

        let line = format!("A:{a:02X} F:{f:02X} B:{b:02X} C:{c:02X} D:{d:02X} E:{e:02X} H:{h:02X} L:{l:02X} SP:{sp:04X} PC:{pc:04X} PCMEM:{pcmem0:02X},{pcmem1:02X},{pcmem2:02X},{pcmem3:02X} LY:{ly:02X} STAT:{stat:02X} LCDC:{lcdc:02X} DIV:{div:02X}\n");

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