use std::{collections::HashMap, fs::File, result, sync::OnceLock};

use serde::Deserialize;

use crate::{gb::{bus::Bus, cpu::Cpu, instructions::MemTarget::{HighC, HighImm8}, registers::FlagBits}, objects::{OpcodeFile, RawFlags, RawOpcode, RawOperand}};

// macro_rules! instr {
//     ($opcode:expr, $name:expr, $cycles:expr, $size:expr, $flags:expr, $exec:expr) => {
//         Some(Instruction {
//             opcode: $opcode,
//             name: $name,
//             cycles: $cycles,
//             size: $size,
//             flags: $flags,
//             execute: $exec,
//         })
//     };
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandKind {
    None,
    Reg8(Reg8),
    Reg16(Reg16),
    Imm8,
    Imm16,
    Mem(MemTarget),
    Cond(Condition),
    Fixed,
    Bit(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemTarget {
    Reg16(Reg16),
    Imm16,
    HighImm8,
    HighC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operands {
    None,
    Imm8,
    Reg(Reg8),
    RegReg(Reg8, Reg8),
    RegImm8(Reg8),
    Reg16(Reg16),
    Reg16Reg16(Reg16, Reg16),
    Reg16Imm16(Reg16),
    MemReg(MemTarget, Reg8),
    RegMem(Reg8, MemTarget),
    Cond(Condition),
    CondImm8(Condition),
    CondImm16(Condition),
    Imm16,
    BitReg(u8, Reg8),
    BitMem(u8, MemTarget),
    Mem(MemTarget),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg8 {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg16 {
    BC,
    DE,
    HL(Delta),
    SP(Delta),
    AF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Delta {
    None,
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Z,
    NZ,
    C,
    NC,
}

#[derive(Debug, Clone, Copy)]
pub struct Instruction {
    pub opcode: u8,
    pub name: &'static str,
    pub cycles: u8,
    pub size: u8,
    pub flags: &'static [FlagBits],
    pub execute: fn(&Instruction, &mut Cpu, &mut Bus) -> u8,
    pub operands: Operands,
}

pub fn unimplemented(instr: &Instruction, cpu: &mut Cpu, _: &mut Bus) -> u8 {
    panic!("Unimplemented instruction: {:02X}, mnemonic: {}, operands: {:?}", instr.opcode, instr.name, instr.operands);
}

pub fn nop(instr: &Instruction, cpu: &mut Cpu, _: &mut Bus) -> u8 {
    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    4
}

// ld instructions

fn ld_r_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegReg(dst, src) = instr.operands else { unreachable!() };
    
    let val = cpu.registers.get8(src);
    cpu.registers.set8(dst, val);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size
    instr.cycles as u8
}

fn ld_r_n8(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegImm8(dst) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;// offsetting from instruction

    let val = bus.read(pc);
    cpu.registers.set8(dst, val);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size
    instr.cycles as u8
}

fn ld_rr_n16(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg16Imm16(dst) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;// offsetting from instruction

    let val = bus.read_u16(pc);
    cpu.registers.set16(dst, val);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size
    instr.cycles as u8
}

fn ld_r_mem(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegMem(dst, mem) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;

    let addr = match mem {
        MemTarget::Reg16(reg) => cpu.registers.get16(reg),
        MemTarget::Imm16 => bus.read_u16(pc),
        _ => unreachable!()
    };

    let val = bus.read(addr);

    cpu.registers.set8(dst, val);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn ld_mem_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::MemReg(mem, src) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;

    let addr = match mem {
        MemTarget::Reg16(reg) => cpu.registers.get16(reg),
        MemTarget::Imm16 => bus.read_u16(pc),
        _ => unreachable!()
    };

    let val = cpu.registers.get8(src);

    bus.write(addr, val);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn ld_rr_rr(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg16Reg16(dst, src) = instr.operands else { unreachable!() };

    let mut value = cpu.registers.get16(src);

    if src == Reg16::SP(Delta::Increment) {
        let pc = cpu.registers.get_pc() + 1;

        value = (value as i16 + bus.read(pc) as i16) as u16;

        // unfinished
    }

    cpu.registers.set16(dst, value);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn ld_deref_rr_a(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let val = cpu.registers.get_a();
    let addr = cpu.registers.get_by_index_u16(instr.opcode);
    
    bus.write(addr, val);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    instr.cycles as u8
}

fn ldh_r_mem(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegMem(dest, mem) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;// offsetting from instruction

    let offset = match mem {
        MemTarget::HighC => cpu.registers.get_c() as u16,
        MemTarget::HighImm8 => bus.read(pc) as u16,
        _ => unreachable!()
    };
    let addr = 0xFF00 + offset;

    cpu.registers.set8(dest, bus.read(addr));

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    instr.cycles as u8
}

fn ldh_mem_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::MemReg(mem, src) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;// offsetting from instruction

    let val = cpu.registers.get8(src);

    let offset = match mem {
        MemTarget::HighC => cpu.registers.get_c() as u16,
        MemTarget::HighImm8 => bus.read(pc) as u16,
        _ => unreachable!()
    };
    let addr = 0xFF00 + offset;

    bus.write(addr, val);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size
    instr.cycles as u8
}

// inc/dec instructions

fn inc_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg(src) = instr.operands else { unreachable!() };
    let old = cpu.registers.get8(src);

    let new = old.wrapping_add(1);

    cpu.registers.set8(src, new);

    cpu.registers.set_flag_to(FlagBits::Z, new == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, (old & 0x0F) + 1 > 0x0F);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    instr.cycles as u8
}

fn dec_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg(src) = instr.operands else { unreachable!() };
    let old = cpu.registers.get8(src);

    let new = old.wrapping_sub(1);

    cpu.registers.set8(src, new);

    cpu.registers.set_flag_to(FlagBits::Z, new == 0);
    cpu.registers.set_flag_to(FlagBits::N, true);
    cpu.registers.set_flag_to(FlagBits::H, old & 0x0F == 0);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    instr.cycles as u8
}

fn dec_mem(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Mem(mem) = instr.operands else { unreachable!() };

    let addr = match mem {
        MemTarget::Reg16(reg) => cpu.registers.get16(reg),
        _ => unreachable!()
    };

    let old = bus.read(addr);

    let new = old.wrapping_sub(1);

    bus.write(addr, new);

    cpu.registers.set_flag_to(FlagBits::Z, new == 0);
    cpu.registers.set_flag_to(FlagBits::N, true);
    cpu.registers.set_flag_to(FlagBits::H, old & 0x0F == 0);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    instr.cycles as u8
}

fn inc_rr(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg16(src) = instr.operands else { unreachable!() };
    let old = cpu.registers.get16(src);

    let new = old.wrapping_add(1);

    cpu.registers.set16(src, new);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    instr.cycles as u8
}

fn dec_rr(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg16(src) = instr.operands else { unreachable!() };
    let old = cpu.registers.get16(src);

    let new = old.wrapping_sub(1);

    cpu.registers.set16(src, new);

    cpu.registers.set_flag_to(FlagBits::Z, new == 0);
    cpu.registers.set_flag_to(FlagBits::N, true);
    cpu.registers.set_flag_to(FlagBits::H, old & 0x0F == 0);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    instr.cycles as u8
}

// add/sub

fn add_r_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegReg(dest, src) = instr.operands else { unreachable!() };

    let reg1 = cpu.registers.get8(dest);
    let reg2 = cpu.registers.get8(src);

    let sum16 = reg1 as u16 + reg2 as u16;
    let result = (sum16 & 0xFF) as u8;

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, (reg1 & 0x0F) + (reg2 & 0x0F) > 0x0F);
    cpu.registers.set_flag_to(FlagBits::C, sum16 > 0xFF);

    cpu.registers.set8(dest, result);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn add_r_d8(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegImm8(dest) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;

    let reg1 = cpu.registers.get8(dest);
    let reg2 = bus.read(pc);

    let sum16 = reg1 as u16 + reg2 as u16;
    let result = (sum16 & 0xFF) as u8;

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, (reg1 & 0x0F) + (reg2 & 0x0F) > 0x0F);
    cpu.registers.set_flag_to(FlagBits::C, sum16 > 0xFF);

    cpu.registers.set8(dest, result);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn adc_r_n8(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegImm8(dest) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;

    let carry = cpu.registers.get_flag(FlagBits::C) as u8;

    let reg1 = cpu.registers.get8(dest);
    let reg2 = bus.read(pc);

    let sum16 = (reg1 as u16 + reg2 as u16) + carry as u16;
    let result = (sum16 & 0xFF) as u8;

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, (reg1 & 0x0F) + (reg2 & 0x0F) + carry > 0x0F);
    cpu.registers.set_flag_to(FlagBits::C, sum16 > 0xFF);

    cpu.registers.set8(dest, result);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn add_rr_rr(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg16Reg16(dest, src) = instr.operands else { unreachable!() };

    let reg1 = cpu.registers.get16(dest);
    let reg2 = cpu.registers.get16(src);

    let sum32 = reg1 as u32 + reg2 as u32;
    let result = (sum32 & 0xFFFF) as u16;

    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, (reg1 & 0x0FFF) + (reg2 & 0x0FFF) > 0x0FFF);
    cpu.registers.set_flag_to(FlagBits::C, sum32 > 0xFFFF);

    cpu.registers.set16(dest, result);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn sub_r_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegReg(dest, src) = instr.operands else { unreachable!() };

    let reg1 = cpu.registers.get8(dest);
    let reg2 = cpu.registers.get8(src);

    let diff16 = (reg1 as u16).wrapping_sub(reg2 as u16);
    let result = (diff16 & 0xFF) as u8;

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, true);
    cpu.registers.set_flag_to(FlagBits::H, (reg1 & 0x0F) < (reg2 & 0x0F));
    cpu.registers.set_flag_to(FlagBits::C, reg1 < reg2);

    cpu.registers.set8(dest, result);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn sub_r_d8(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegImm8(dest) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;

    let reg1 = cpu.registers.get8(dest);
    let reg2 = bus.read(pc);

    let diff16 = (reg1 as u16).wrapping_sub(reg2 as u16);
    let result = (diff16 & 0xFF) as u8;

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, true);
    cpu.registers.set_flag_to(FlagBits::H, (reg1 & 0x0F) < (reg2 & 0x0F));
    cpu.registers.set_flag_to(FlagBits::C, reg1 < reg2);

    cpu.registers.set8(dest, result);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

// cp

fn cp_r_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegReg(dest, src) = instr.operands else { unreachable!() };

    let reg1 = cpu.registers.get8(dest);
    let reg2 = cpu.registers.get8(src);

    let diff16 = reg1 as u16 - reg2 as u16;
    let result = (diff16 & 0xFF) as u8;

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, true);
    cpu.registers.set_flag_to(FlagBits::H, (reg1 & 0x0F) < (reg2 & 0x0F));
    cpu.registers.set_flag_to(FlagBits::C, reg1 < reg2);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn cp_r_n8(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegImm8(dest) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;

    let reg1 = cpu.registers.get8(dest);
    let reg2 = bus.read(pc);

    let diff16 = (reg1 as u16).wrapping_sub(reg2 as u16);
    let result = (diff16 & 0xFF) as u8;

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, true);
    cpu.registers.set_flag_to(FlagBits::H, (reg1 & 0x0F) < (reg2 & 0x0F));
    cpu.registers.set_flag_to(FlagBits::C, reg1 < reg2);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

// or/and

fn or_r_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 { // freddy fazfunction
    let Operands::RegReg(dst, src) = instr.operands else { unreachable!() };

    let result = cpu.registers.get8(dst) | cpu.registers.get8(src);
    cpu.registers.set8(dst, result);

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, false);
    cpu.registers.set_flag_to(FlagBits::C, false);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn or_r_mem(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegMem(dst, mem) = instr.operands else { unreachable!() };

    let addr = match mem {
        MemTarget::Reg16(reg) => cpu.registers.get16(reg),
        _ => unreachable!()
    };

    let result = cpu.registers.get8(dst) | bus.read(addr);
    cpu.registers.set8(dst, result);

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, false);
    cpu.registers.set_flag_to(FlagBits::C, false);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn xor_r_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegReg(dst, src) = instr.operands else { unreachable!() };

    let result = cpu.registers.get8(dst) ^ cpu.registers.get8(src);
    cpu.registers.set8(dst, result);

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, false);
    cpu.registers.set_flag_to(FlagBits::C, false);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn xor_r_n8(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegImm8(dst) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;

    let result = cpu.registers.get8(dst) ^ bus.read(pc);
    cpu.registers.set8(dst, result);

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, false);
    cpu.registers.set_flag_to(FlagBits::C, false);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn xor_r_mem(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegMem(dst, mem) = instr.operands else { unreachable!() };

    let addr = match mem {
        MemTarget::Reg16(reg) => cpu.registers.get16(reg),
        _ => unreachable!(),
    };

    let result = cpu.registers.get8(dst) ^ bus.read(addr);
    cpu.registers.set8(dst, result);

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, false);
    cpu.registers.set_flag_to(FlagBits::C, false);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn and_r_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegReg(dst, src) = instr.operands else { unreachable!() };

    let result = cpu.registers.get8(dst) & cpu.registers.get8(src);
    cpu.registers.set8(dst, result);

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, true);
    cpu.registers.set_flag_to(FlagBits::C, false);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn and_r_n8(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::RegImm8(dst) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;

    let result = cpu.registers.get8(dst) & bus.read(pc);
    cpu.registers.set8(dst, result);

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, true);
    cpu.registers.set_flag_to(FlagBits::C, false);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

// jp instructions

fn jp_a16(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    // read from rom
    // pc + 1, pc + 2
    let pc = cpu.registers.get_pc() + 1;// offsetting from instruction

    let dest = bus.read_u16(pc);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    cpu.registers.set_pc(dest);

    instr.cycles as u8
}

fn jp_cc_a16(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::CondImm16(cond) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;// offsetting from instruction

    let dest = bus.read_u16(pc);
    cpu.registers.inc_pc_by(instr.size as u16);

    if cpu.registers.check_conditions(cond) {
        cpu.registers.set_pc(dest);

        16
    } else {
        12
    }
}

fn jp_rr(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg16(src) = instr.operands else { unreachable!() };

    let dest = cpu.registers.get16(src);

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    cpu.registers.set_pc(dest);

    instr.size
}

fn jr_r8(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let pc = cpu.registers.get_pc() + 1;// offsetting from instruction

    let offset = bus.read(pc) as i8;

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    let dest = cpu.registers.get_pc().wrapping_add_signed(offset as i16);

    cpu.registers.set_pc(dest);

    instr.size
}

fn jr_cc_e8(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::CondImm8(cond) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;
    let offset = bus.read(pc) as i8;

    cpu.registers.inc_pc_by(instr.size as u16);// increment pc by instruction size

    let dest = cpu.registers.get_pc().wrapping_add_signed(offset as i16);

    if cpu.registers.check_conditions(cond) {
        cpu.registers.set_pc(dest);
        12
    } else {
        8
    }
}

// call/ret

fn call_a16(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Imm16 = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;

    let addr = bus.read_u16(pc);

    cpu.registers.dec_sp(2);

    cpu.registers.inc_pc_by(instr.size as u16);
    bus.write_u16(cpu.registers.get_sp(), cpu.registers.get_pc());

    cpu.registers.set_pc(addr);

    instr.cycles as u8
}

fn call_cc_a16(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::CondImm16(cond) = instr.operands else { unreachable!() };
    let pc = cpu.registers.get_pc() + 1;
    let addr = bus.read_u16(pc);

    cpu.registers.inc_pc_by(instr.size as u16);

    if cpu.registers.check_conditions(cond) {
        cpu.registers.dec_sp(2);
        bus.write_u16(cpu.registers.get_sp(), cpu.registers.get_pc());

        cpu.registers.set_pc(addr);

        24
    } else {
        12
    }
}

fn ret(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let addr = bus.read_u16(cpu.registers.get_sp());
    cpu.registers.inc_sp(2);

    cpu.registers.set_pc(addr);

    instr.cycles as u8
}

fn ret_cc(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Cond(cond) = instr.operands else { unreachable!() };
    let addr = bus.read_u16(cpu.registers.get_sp());

    if cpu.registers.check_conditions(cond) {
        cpu.registers.inc_sp(2);
        cpu.registers.set_pc(addr);
        
        20
    } else {
        cpu.registers.inc_pc();

        8
    }

}

// push/pop

fn push_rr(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg16(src) = instr.operands else { unreachable!() };

    cpu.registers.dec_sp(2);
    bus.write_u16(cpu.registers.get_sp(), cpu.registers.get16(src));

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn pop_rr(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg16(dst) = instr.operands else { unreachable!() };

    let val = bus.read_u16(cpu.registers.get_sp());
    cpu.registers.inc_sp(2);

    cpu.registers.set16(dst, val);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

// misc

fn di(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    cpu.ime = false;

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn cb(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let pc = cpu.registers.get_pc() + 1;
    let opcode = bus.read(pc);

    let next = opcodes_cb()[opcode as usize].expect(&*format!("Unknown opcode {:#02X}", opcode));

    instr.cycles + ((next.execute)(&next, cpu, bus))
}

fn ei(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    cpu.enable_ime_next = true;

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles
}

fn daa(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let mut a = cpu.registers.get_a();
    let mut adjust = 0;
    let mut carry = false;

    if !cpu.registers.get_flag(FlagBits::N) {
        if cpu.registers.get_flag(FlagBits::H) || (a & 0x0F) > 9 {
            adjust |= 0x06;
        }

        if cpu.registers.get_flag(FlagBits::C) || a > 0x99 {
            adjust |= 0x60;
            carry = true;
        }

        a = a.wrapping_add(adjust);
    } else {
        if cpu.registers.get_flag(FlagBits::H) {
            adjust |= 0x06;
        }

        if cpu.registers.get_flag(FlagBits::C) {
            adjust |= 0x60;
        }
        a = a.wrapping_sub(adjust);
    }

    cpu.registers.set_a(a);

    cpu.registers.set_flag_to(FlagBits::Z, a == 0);
    cpu.registers.set_flag_to(FlagBits::H, false);
    cpu.registers.set_flag_to(FlagBits::C, carry || cpu.registers.get_flag(FlagBits::C));

    cpu.registers.inc_pc_by(instr.size as u16);

    instr.cycles
}

// CB instructions

// sr

fn srl_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg(dst) = instr.operands else { unreachable!() };

    let old = cpu.registers.get8(dst);
    let carry = old & 0x01;

    let result = old >> 1;
    cpu.registers.set8(dst, result);

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, false);
    cpu.registers.set_flag_to(FlagBits::C, carry != 0);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn rr_r(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let Operands::Reg(dst) = instr.operands else { unreachable!() };

    let old = cpu.registers.get8(dst);
    let old_carry = cpu.registers.get_flag(FlagBits::C);

    let carry = (old & 0x01) != 0;

    let result = (old >> 1) | ((old_carry as u8) << 7);
    cpu.registers.set8(dst, result);

    cpu.registers.set_flag_to(FlagBits::Z, result == 0);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, false);
    cpu.registers.set_flag_to(FlagBits::C, carry);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

fn rra(instr: &Instruction, cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let old = cpu.registers.get_a();
    let old_carry = cpu.registers.get_flag(FlagBits::C);

    let carry = (old & 0x01) != 0;

    let result = (old >> 1) | ((old_carry as u8) << 7);
    cpu.registers.set_a(result);

    cpu.registers.set_flag_to(FlagBits::Z, false);
    cpu.registers.set_flag_to(FlagBits::N, false);
    cpu.registers.set_flag_to(FlagBits::H, false);
    cpu.registers.set_flag_to(FlagBits::C, carry);

    cpu.registers.inc_pc_by(instr.size as u16);
    instr.cycles as u8
}

static OPCODES: OnceLock<[Option<Instruction>; 256]> = OnceLock::new();
static OPCODES_CB: OnceLock<[Option<Instruction>; 256]> = OnceLock::new();

pub fn opcodes() -> &'static [Option<Instruction>; 256] {
    OPCODES.get_or_init(|| {
        let file = load_opcodes_file();
        build_table(&file.unprefixed, false)
    })
}

pub fn opcodes_cb() -> &'static [Option<Instruction>; 256] {
    OPCODES_CB.get_or_init(|| {
        let file = load_opcodes_file();
        build_table(&file.cbprefixed, false) // is_cb handling — see below
    })
}

fn load_opcodes_file() -> OpcodeFile {
    let json = include_str!("../../Opcodes.json");
    serde_json::from_str(json).expect("Opcodes.json malformed")
}

fn build_table(raw_map: &HashMap<String, RawOpcode>, is_cb: bool) -> [Option<Instruction>; 256] {
    let mut table = [None; 256];
    for (key, raw) in raw_map {
        let opcode = u8::from_str_radix(key.trim_start_matches("0x"), 16)
            .unwrap_or_else(|_| panic!("bad opcode key: {key}"));

        let operands = operands_from_raw(&raw.mnemonic, &raw.operands);

        table[opcode as usize] = Some(Instruction {
            opcode,
            name: raw.mnemonic.clone().leak(),
            cycles: raw.cycles[0],
            size: raw.bytes,
            flags: flags_from_raw(&raw.flags),
            execute: dispatch_for(opcode, is_cb, &raw.mnemonic, operands),
            operands,
        });
    }

    table
}

fn operands_from_raw(mnemonic: &str, operands: &Vec<RawOperand>) -> Operands {
    let is_branch =  matches!(mnemonic, "JP" | "JR" | "CALL" | "RET");
    
    let oper_tuple = if operands.len() > 1 {
        (
            operand_from_raw(&operands[0], is_branch && true),
            operand_from_raw(&operands[1], false)
        )
    } else if operands.len() == 1 {
        (
            operand_from_raw(&operands[0], is_branch && true),
            OperandKind::None
        )
    } else {
        (OperandKind::None, OperandKind::None)
    };

    // let operv: (OperandKind, OperandKind) = operands
    //     .iter()
    //     .enumerate()
    //     .map(|(i, op)| operand_from_raw(op, is_branch && i == 0))
    //     .collect();

    match oper_tuple {
        (OperandKind::Reg8(reg1), OperandKind::None) => Operands::Reg(reg1),
        (OperandKind::Reg8(reg1), OperandKind::Reg8(reg2)) => Operands::RegReg(reg1, reg2),
        (OperandKind::Reg8(reg1), OperandKind::Imm8) => Operands::RegImm8(reg1),
        (OperandKind::Reg16(reg1), OperandKind::Imm16) => Operands::Reg16Imm16(reg1),
        (OperandKind::Mem(target), OperandKind::Reg8(reg1)) => Operands::MemReg(target, reg1),
        (OperandKind::Reg8(reg1), OperandKind::Mem(target)) => Operands::RegMem(reg1, target),
        (OperandKind::Cond(condition), OperandKind::None) => Operands::Cond(condition),
        (OperandKind::Cond(condition), OperandKind::Imm8) => Operands::CondImm8(condition),
        (OperandKind::Cond(condition), OperandKind::Imm16) => Operands::CondImm16(condition),
        (OperandKind::Imm16, OperandKind::None) => Operands::Imm16,
        (OperandKind::Imm8, OperandKind::None) => Operands::Imm8,
        (OperandKind::Reg16(reg1), OperandKind::None) => Operands::Reg16(reg1),
        (OperandKind::Bit(bit), OperandKind::Reg8(reg1)) => Operands::BitReg(bit, reg1),
        (OperandKind::Bit(bit), OperandKind::Mem(mem)) => Operands::BitMem(bit, mem),
        (OperandKind::Mem(mem), OperandKind::None) => Operands::Mem(mem),
        (OperandKind::Reg16(reg1), OperandKind::Reg16(reg2)) => Operands::Reg16Reg16(reg1, reg2),

        _ => Operands::None
    }

    
}


fn operand_from_raw(operand: &RawOperand, as_condition: bool) -> OperandKind {
    if as_condition {
        return match operand.name.as_str() {
            "Z" => OperandKind::Cond(Condition::Z),
            "NZ" => OperandKind::Cond(Condition::NZ),
            "C" => OperandKind::Cond(Condition::C),
            "NC" => OperandKind::Cond(Condition::NC),
            "a16" => OperandKind::Imm16,
            "e8" => OperandKind::Imm8,
            "HL" => OperandKind::Reg16(Reg16::HL(operand.delta())),
            other => panic!("unexpected condition {other:?}"),
        }
    }

    let base = match operand.name.as_str() {
        "A" => OperandKind::Reg8(Reg8::A),
        "B" => OperandKind::Reg8(Reg8::B),
        "C" => OperandKind::Reg8(Reg8::C),
        "D" => OperandKind::Reg8(Reg8::D),
        "E" => OperandKind::Reg8(Reg8::E),
        "F" => OperandKind::Reg8(Reg8::F),
        "H" => OperandKind::Reg8(Reg8::H),
        "L" => OperandKind::Reg8(Reg8::L),

        "BC" => OperandKind::Reg16(Reg16::BC),
        "DE" => OperandKind::Reg16(Reg16::DE),
        "HL" => OperandKind::Reg16(Reg16::HL(operand.delta())),
        "SP" => OperandKind::Reg16(Reg16::SP(operand.delta())),
        "AF" => OperandKind::Reg16(Reg16::AF),   

        "n8" | "d8" => OperandKind::Imm8,
        "n16" | "d16" | "a16" => OperandKind::Imm16,
        "a8" => OperandKind::Imm8,
        "e8" => OperandKind::Imm8,

        "$00" | "$08" | "$10" | "$18" | "$20" | "$28" | "$30" | "$38" => OperandKind::Fixed,

        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" => OperandKind::Bit(operand.name.parse().unwrap()),

        other => panic!("Uknown operand name {other:?}")
    };

    if operand.immediate {
        base
    } else { // kind of bad ignore
        match base {
            OperandKind::Reg16(target) => OperandKind::Mem(MemTarget::Reg16(target)),
            OperandKind::Imm16 => OperandKind::Mem(MemTarget::Imm16),
            OperandKind::Imm8 => OperandKind::Mem(HighImm8),
            OperandKind::Reg8(_) => OperandKind::Mem(HighC),
            _ => unreachable!("ruh roh")
        }
    }
}

fn flags_from_raw(raw: &RawFlags) -> &'static [FlagBits] {
    let mut v = Vec::new();
    if raw.z != "-" { v.push(FlagBits::Z); }
    if raw.n != "-" { v.push(FlagBits::N); }
    if raw.h != "-" { v.push(FlagBits::H); }
    if raw.c != "-" { v.push(FlagBits::C); }

    v.leak()
}

fn dispatch_for(opcode: u8, is_cb: bool, mnemonic: &String, operands: Operands) -> fn(&Instruction, &mut Cpu, &mut Bus) -> u8 {
    match (mnemonic.as_str() ,operands) {
        ("NOP", Operands::None) => nop,

        ("LD", Operands::RegReg(_, _)) => ld_r_r,
        ("LD", Operands::RegImm8(_)) => ld_r_n8,
        ("LD", Operands::Reg16Reg16(_, _)) => ld_rr_rr,
        ("LD", Operands::Reg16Imm16(_)) => ld_rr_n16,
        ("LD", Operands::RegMem(_, _)) => ld_r_mem,
        ("LD", Operands::MemReg(_, _)) => ld_mem_r,
        
        ("LDH", Operands::RegMem(_, _)) => ldh_r_mem,
        ("LDH", Operands::MemReg(_, _)) => ldh_mem_r,

        ("INC", Operands::Reg(_)) => inc_r,
        ("INC", Operands::Reg16(_)) => inc_rr,

        ("DEC", Operands::Reg(_)) => dec_r,
        ("DEC", Operands::Mem(_)) => dec_mem,
        ("DEC", Operands::Reg16(_)) => dec_rr,

        ("CP", Operands::RegReg(_, _)) => cp_r_r,
        ("CP", Operands::RegImm8(_)) => cp_r_n8,

        ("ADD", Operands::RegReg(_, _)) => add_r_r,
        ("ADD", Operands::RegImm8(_)) => add_r_d8,
        ("ADD", Operands::Reg16Reg16(_, _)) => add_rr_rr,

        ("ADC", Operands::RegImm8(_)) => adc_r_n8,

        ("SUB", Operands::RegReg(_, _)) => sub_r_r,
        ("SUB", Operands::RegImm8(_)) => sub_r_d8,

        ("OR", Operands::RegReg(_, _)) => or_r_r,
        ("OR", Operands::RegMem(_, _)) => or_r_mem,

        ("XOR", Operands::RegReg(_, _)) => xor_r_r,
        ("XOR", Operands::RegImm8(_)) => xor_r_n8,
        ("XOR", Operands::RegMem(_, _)) => xor_r_mem,

        ("AND", Operands::RegReg(_, _)) => and_r_r,
        ("AND", Operands::RegImm8(_)) => and_r_n8,

        ("JP", Operands::Imm16) => jp_a16,
        ("JP", Operands::CondImm16(_)) => jp_cc_a16,
        ("JP", Operands::Reg16(_)) => jp_rr,
        ("JR", Operands::Imm8) => jr_r8,
        ("JR", Operands::CondImm8(_)) => jr_cc_e8,

        ("CALL", Operands::Imm16) => call_a16,
        ("CALL", Operands::CondImm16(_)) => call_cc_a16,

        ("RET", Operands::None) => ret,
        ("RET", Operands::Cond(_)) => ret_cc,

        ("PUSH", Operands::Reg16(_)) => push_rr,

        ("POP", Operands::Reg16(_)) => pop_rr,

        ("DI", Operands::None) => di,
        ("PREFIX", Operands::None) => cb,
        ("EI", Operands::None) => ei,
        ("DAA", Operands::None) => daa,

        // CB instructions

        ("SRL", Operands::Reg(_)) => srl_r,

        ("RR", Operands::Reg(_)) => rr_r,
        ("RRA", Operands::None) => rra,

        (_, _) => unimplemented,
        (m, ops) => panic!("no handler for {m} {ops:?}, opcode: {opcode:x}"),
    }
}

// const fn create_opcodes() -> [Option<Instruction>; 256] {
//     let mut optable = [None; 256];

//     // optable[{opcode}] = instr!({opcode}, "{opcode_name}", {cycles}, {byte size}, &[{flags}], {function})
//     optable[0x00] = instr!(0x00, "NOP", 4, 1, &[], nop);

//     optable[0x01] = instr!(0x02, "LD BC, n16", 12, 3, &[], ld_rr_n16);
//     optable[0x11] = instr!(0x03, "LD DE, n16", 12, 3, &[], ld_rr_n16);
//     optable[0x21] = instr!(0x02, "LD HL, n16", 12, 3, &[], ld_rr_n16);
//     optable[0x31] = instr!(0x03, "LD SP, n16", 12, 3, &[], ld_rr_n16);

//     optable[0x02] = instr!(0x02, "LD [BC], A", 8, 1, &[], ld_deref_rr_a);
//     optable[0x12] = instr!(0x02, "LD [DE], A", 8, 1, &[], ld_deref_rr_a);

//     optable[0x0E] = instr!(0x02, "LD C, n16", 12, 3, &[], ld_r_n8);
//     optable[0x1E] = instr!(0x03, "LD E, n16", 12, 3, &[], ld_r_n8);
//     optable[0x2E] = instr!(0x02, "LD L, n16", 12, 3, &[], ld_r_n8);
//     optable[0x3E] = instr!(0x03, "LD A, n16", 12, 3, &[], ld_r_n8);

//     // inc/dec ops

//     optable[0x04] = instr!(0x04, "INC B", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], inc_r);
//     optable[0x14] = instr!(0x14, "INC D", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], inc_r);
//     optable[0x24] = instr!(0x24, "INC H", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], inc_r);

//     optable[0x05] = instr!(0x05, "DEC B", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], dec_r);
//     optable[0x15] = instr!(0x15, "DEC D", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], dec_r);
//     optable[0x25] = instr!(0x25, "DEC H", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], dec_r);
    
//     optable[0x0C] = instr!(0x0C, "INC C", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], inc_r);
//     optable[0x1C] = instr!(0x1C, "INC E", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], inc_r);
//     optable[0x2C] = instr!(0x2C, "INC L", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], inc_r);
//     optable[0x3C] = instr!(0x3C, "INC A", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], inc_r);

//     optable[0x0D] = instr!(0x0D, "DEC C", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], dec_r);
//     optable[0x1D] = instr!(0x1D, "DEC E", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], dec_r);
//     optable[0x2D] = instr!(0x2D, "DEC L", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], dec_r);
//     optable[0x3D] = instr!(0x3D, "DEC A", 4, 1, &[FlagBits::Z, FlagBits::H, FlagBits::N], dec_r);

//     // jp ops

//     optable[0x20] = instr!(0x20, "JR NZ, e8", 12, 2, &[], jr_cc_e8);
//     optable[0x30] = instr!(0x30, "JR NC, e8", 12, 2, &[], jr_cc_e8);

//     optable[0xC3] = instr!(0xC3, "JP a16", 16, 3, &[], jp_a16);

//     // generate ld r r opcodes
//     let mut opcode = 0x40u8;
//     while opcode <= 0x7F {
//         if opcode != 0x76 {
//             let cycles = if opcode & 0x07 == 6 || (opcode >> 3) & 0x07 == 6 {
//                 8
//             } else {
//                 4
//             };
//             optable[opcode as usize] = instr!(opcode, ld_r_r_name(opcode), cycles, 1, &[], ld_r_r);
//         }

//         opcode += 1;
//     }

//     optable
// }

// const fn create_cb_opcodes() -> [Option<Instruction>; 256] {
//     let mut optable = [None; 256];

//     optable
// }

// pub const OPCODES: [Option<Instruction>; 256] = create_opcodes();
// pub const OPCODES_CB: [Option<Instruction>; 256] = create_cb_opcodes();

// const fn ld_r_r_name(opcode: u8) -> &'static str {
//     match opcode {
//         0x40 => "LD B,B", 0x41 => "LD B,C", 0x42 => "LD B,D", 0x43 => "LD B,E",
//         0x44 => "LD B,H", 0x45 => "LD B,L",                   0x47 => "LD B,A",
//         0x48 => "LD C,B", 0x49 => "LD C,C", 0x4A => "LD C,D", 0x4B => "LD C,E",
//         0x4C => "LD C,H", 0x4D => "LD C,L",                   0x4F => "LD C,A",
//         0x50 => "LD D,B", 0x51 => "LD D,C", 0x52 => "LD D,D", 0x53 => "LD D,E",
//         0x54 => "LD D,H", 0x55 => "LD D,L",                   0x57 => "LD D,A",
//         0x58 => "LD E,B", 0x59 => "LD E,C", 0x5A => "LD E,D", 0x5B => "LD E,E",
//         0x5C => "LD E,H", 0x5D => "LD E,L",                   0x5F => "LD E,A",
//         0x60 => "LD H,B", 0x61 => "LD H,C", 0x62 => "LD H,D", 0x63 => "LD H,E",
//         0x64 => "LD H,H", 0x65 => "LD H,L",                   0x67 => "LD H,A",
//         0x68 => "LD L,B", 0x69 => "LD L,C", 0x6A => "LD L,D", 0x6B => "LD L,E",
//         0x6C => "LD L,H", 0x6D => "LD L,L",                   0x6F => "LD L,A",
//         0x78 => "LD A,B", 0x79 => "LD A,C", 0x7A => "LD A,D", 0x7B => "LD A,E",
//         0x7C => "LD A,H", 0x7D => "LD A,L",                   0x7F => "LD A,A",
//         _ => "LD r,(HL)/(HL),r", // 0x46,0x4E,0x56,0x5E,0x66,0x6E,0x70-0x75,0x77
//     }
// }