use std::{fs::{self, OpenOptions}, io::Write};

use serde::{Deserialize, Serialize, de::value};

use crate::gb::{STAT_COUNT, TIMER_COUNT, VBLANK_COUNT, bus::{self, Bus}, cartridge::Cartridge, instructions::{self, Instruction, opcodes}, registers::Registers};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Cpu {
    pub registers: Registers,
    pub debug: bool,
    pub ime: bool,
    pub enable_ime_next: bool,
    pub halted: bool,
    pub halt_bug: bool,
    pub double_speed: bool,

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
            double_speed: false,
            opcode: 0,
            cycles: 0,
            debug
        }
    }

    pub fn fetch8(&mut self, bus: &mut Bus) -> u8 {
        let value = bus.read_cycle(self.registers.get_pc());
        self.registers.inc_pc_by(1);
        value
    }

    pub fn fetch16(&mut self, bus: &mut Bus) -> u16 {
        let lo = self.fetch8(bus)as u16;
        let hi = self.fetch8(bus) as u16;
        (hi << 8) | lo
    }

    pub fn step(&mut self, bus: &mut Bus) {
        if self.handle_interrupts(bus) {
            return;
        }

        if self.halted {
            bus.tick(self.double_speed);
            let pending = bus.get_ie() & bus.get_iflag();
            if pending != 0 {
                self.halted = false;
            }
            return;
        }

        let apply_halt_bug = self.halt_bug;
        self.halt_bug = false;

        #[cfg(debug_assertions)]
        let start = bus.tick_count;

        let pc_before = self.registers.get_pc();
        let opcode = self.fetch8(bus);

        let instr = opcodes()[opcode as usize].expect(&*format!("Unknown opcode {:#02X}", opcode));

        (instr.execute)(&instr, self, bus);// execute instruction


        #[cfg(debug_assertions)]
        {
            use crate::gb::instructions::Operands;

            let m_cycles_used = bus.tick_count - start;
            let t_cycles_used = m_cycles_used as u16 * 4;
            match instr.operands {
                Operands::Cond(_) | Operands::CondImm16(_) | Operands::CondImm8(_) => {// not rehandling branch timing rn im lazy
                    if t_cycles_used != instr.cycles as u16 {
                        println!("cycle mismatch: opcode {opcode:#04X} expected {} got {t_cycles_used}", instr.cycles);
                    }
                } 

                _ => {
                    if opcode != 0xCB {
                        debug_assert_eq!(
                            t_cycles_used, instr.cycles as u16,
                            "cycle mismatch: opcode {opcode:#04X} expected {} got {t_cycles_used}",
                            instr.cycles
                        );
                    }
                }
            }
        }

        if apply_halt_bug {
            self.registers.set_pc(pc_before);
        }

        if self.enable_ime_next {
            self.ime = true;
            self.enable_ime_next = false;
        }
    }

    pub fn handle_interrupts(&mut self, bus: &mut Bus) -> bool {
        if !self.ime {
            return false;
        }

        let ie = bus.read(0xFFFF);
        let mut iflag = bus.read(0xFF0F);
        let log_iflag = iflag;

        let pending = ie & iflag;
        if pending == 0 {
            return false;
        }

        for i in 0..5 {
            if pending & (1 << i) != 0 {
                bus.tick(self.double_speed);
                bus.tick(self.double_speed);

                bus.write(0xFF0F, iflag & !(1 << i));
                self.ime = false;

                let sp = self.registers.get_sp().wrapping_sub(1);
                self.registers.set_sp(sp);
                bus.write_cycle(sp, (self.registers.get_pc() >> 8) as u8);

                let sp = sp.wrapping_sub(1);
                self.registers.set_sp(sp);
                bus.write_cycle(sp, (self.registers.get_pc() & 0xFF) as u8);

                let vector = match i {
                    0 => { unsafe { VBLANK_COUNT += 1; } 0x0040 }
                    1 => { unsafe { STAT_COUNT  += 1; } 0x0048 }
                    2 => { unsafe { TIMER_COUNT += 1; } 0x0050 }
                    3 => 0x0058,
                    4 => 0x0060,
                    _ => unreachable!(),
                };

                self.registers.set_pc(vector);
                bus.tick(self.double_speed);

                return true;
            }
        }

        return false;
    }

    pub fn push16(&mut self, bus: &mut Bus, value: u16) {
        let sp = self.registers.get_sp().wrapping_sub(1);
        self.registers.set_sp(sp);
        bus.write_cycle(sp, (value >> 8) as u8);

        let sp = sp.wrapping_sub(1);
        self.registers.set_sp(sp);
        bus.write_cycle(sp, (value & 0xFF) as u8);
    }

    pub fn pop16(&mut self, bus: &mut Bus) -> u16 {
        let sp = self.registers.get_sp();
        let lo = bus.read_cycle(sp);
        let hi = bus.read_cycle(sp.wrapping_add(1));
        self.registers.set_sp(sp.wrapping_add(2));
        (hi as u16) << 8 | lo as u16
    }

    // fn write_to_log(&self, bus: &mut Bus) {// super brittle is temporary :)
    //     let file = OpenOptions::new()
    //         .write(true)
    //         .append(true)
    //         .open("log.txt");

    //     let (a, f, b, c, d, e, h, l, sp, pc) = (
    //         self.registers.get_a(),
    //         self.registers.get_f(),
    //         self.registers.get_b(),
    //         self.registers.get_c(),
    //         self.registers.get_d(),
    //         self.registers.get_e(),
    //         self.registers.get_h(),
    //         self.registers.get_l(),
    //         self.registers.get_sp(),
    //         self.registers.get_pc(),
    //     );

    //     let (pcmem0, pcmem1, pcmem2, pcmem3) = (
    //         bus.read(pc),
    //         bus.read(pc + 1),
    //         bus.read(pc + 2),
    //         bus.read(pc + 3),
    //     );

    //     let (ly, stat, lcdc, div) = (
    //         bus.get_ly(),
    //         bus.get_stat(),
    //         bus.get_lcdc(),
    //         bus.read_div(),
    //     );

    //     let line = format!("A:{a:02X} F:{f:02X} B:{b:02X} C:{c:02X} D:{d:02X} E:{e:02X} H:{h:02X} L:{l:02X} SP:{sp:04X} PC:{pc:04X} PCMEM:{pcmem0:02X},{pcmem1:02X},{pcmem2:02X},{pcmem3:02X} LY:{ly:02X} STAT:{stat:02X} LCDC:{lcdc:02X} DIV:{div:02X}\n");

    //     file.unwrap().write_all(line.as_bytes()).unwrap();
    // }
}

// fn flags_str(f: u8) -> String {
//     let z = if f & 0x80 != 0 { 'Z' } else { '-' };
//     let n = if f & 0x40 != 0 { 'N' } else { '-' };
//     let h = if f & 0x20 != 0 { 'H' } else { '-' };
//     let c = if f & 0x10 != 0 { 'C' } else { '-' };
//     format!("{z}{n}{h}{c}")
// }