use serde::{Deserialize, Serialize};

use crate::gb::{bus::Bus, instructions::{Condition, Delta, Reg8::{self, A, B, C, D, E, F, H, L}, Reg16::{self, AF, BC, DE, HL, SP}}, registers};

macro_rules! get_set  {
    ($reg:ident, $get_name:ident, $set_name:ident, $size:ty) => {
        pub fn $get_name(&self) -> $size {
            self.$reg
        }

        pub fn $set_name(&mut self, val: $size) {
            self.$reg = val;
        }
    };
}

macro_rules! get_set_dual {
    ($reg1:ident, $reg2:ident, $get_name:ident, $set_name:ident) => {
        pub fn $get_name(&self) -> u16 {
            (self.$reg1 as u16) << 8 | self.$reg2 as u16
        }

        pub fn $set_name(&mut self, val: u16) {
            self.$reg1 = (val >> 8) as u8;
            self.$reg2 = val as u8;
        }
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum FlagBits {
    Z = 0b1000_0000,
    N = 0b0100_0000,
    H = 0b0010_0000,
    C = 0b0001_0000,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
}

impl Registers {
    pub fn new() -> Self {
        Registers { 
            a: 0x01, 
            b: 0x00,
            c: 0x13,
            d: 0x00, 
            e: 0xD8, 
            f: 0xB0, 
            h: 0x01, 
            l: 0x4D, 
            sp: 0xFFFE, 
            pc: 0x0100 
        }
    }

    get_set!(a, get_a, set_a, u8);
    get_set!(b, get_b, set_b, u8);
    get_set!(c, get_c, set_c, u8);
    get_set!(d, get_d, set_d, u8);
    get_set!(e, get_e, set_e, u8);
    get_set!(h, get_h, set_h, u8);
    get_set!(l, get_l, set_l, u8);
    get_set!(sp, get_sp, set_sp, u16);
    get_set!(pc, get_pc, set_pc, u16);

    get_set_dual!(b, c, get_bc, set_bc);
    get_set_dual!(d, e, get_de, set_de);
    get_set_dual!(h, l, get_hl, set_hl);

    pub fn get_f(&self) -> u8 {
        self.f
    }

    pub fn set_f(&mut self, val: u8) {
        self.f = val & 0xF0
    }

    pub fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | self.f as u16
    }
    pub fn set_af(&mut self, val: u16) {
        self.a = (val >> 8) as u8;
        self.f = (val & 0x00F0) as u8;
    }

    pub fn get_and_inc_pc(&mut self) -> u16 { // rename me maybe???
        let ret_pc = self.pc;
        self.pc += 1;
        ret_pc
    }

    pub fn inc_pc(&mut self) -> u16 {
        self.pc += 1;
        self.pc
    }

    pub fn inc_pc_by(&mut self, value: u16) {
        self.pc = self.pc.wrapping_add(value)
    }

    pub fn get_by_index(&self, idx: u8) -> u8 {
        match idx & 0x07 {
            0 => self.get_b(),
            1 => self.get_c(),
            2 => self.get_d(),
            3 => self.get_e(),
            4 => self.get_h(),
            5 => self.get_l(),
            7 => self.get_a(),
            _ => unreachable!("index 6 is (HL), handled separately"),
        }
    }

    pub fn set_by_index(&mut self, idx: u8, val: u8) {
        match idx & 0x07 {
            0 => self.set_b(val),
            1 => self.set_c(val),
            2 => self.set_d(val),
            3 => self.set_e(val),
            4 => self.set_h(val),
            5 => self.set_l(val),
            7 => self.set_a(val),
            _ => unreachable!("index 6 is (HL), handled separately"),
        }
    }

    pub fn get8(&self, reg: Reg8) -> u8 {
        match reg {
            A => self.a,
            B => self.b,
            C => self.c,
            D => self.d,
            E => self.e,
            F => self.f,
            H => self.h,
            L => self.l,
        }
    }

    pub fn set8(&mut self, reg: Reg8, val: u8) {
        match reg {
            A => self.a = val,
            B => self.b = val,
            C => self.c = val,
            D => self.d = val,
            E => self.e = val,
            F => self.f = val & 0xF0,
            H => self.h = val,
            L => self.l = val,
        }
    }

    pub fn get16(&mut self, reg: Reg16) -> u16 {
        match reg {
            BC => self.get_bc(),
            DE => self.get_de(),
            HL(delta) => {
                match delta {
                    Delta::None => self.get_hl(),
                    Delta::Increment => {
                        let old = self.get_hl();
                        self.set_hl(old.wrapping_add(1));
                        old
                    }
                    Delta::Decrement => {
                        let old = self.get_hl();
                        self.set_hl(old.wrapping_sub(1));
                        old
                    }
                }
            }
            SP(_) => self.get_sp(),
            AF => self.get_af(),
        }
    }

    pub fn set16(&mut self, reg: Reg16 , val: u16) {
        match reg {
            BC => self.set_bc(val),
            DE => self.set_de(val),
            HL(_) => self.set_hl(val),
            SP(_) => self.set_sp(val),
            AF => self.set_af(val),
        }
    }

    pub fn get_by_index_u16(&self, idx: u8) -> u16 {
        match (idx & 0xF0) >> 4 {
            0 => self.get_bc(),
            1 => self.get_de(),
            2 => self.get_hl(),
            3 => self.get_sp(),
            _ => unreachable!("ruh roh"),
        }
    }

    pub fn set_by_index_u16(&mut self, idx: u8, val: u16) {
        match (idx & 0xF0) >> 4 {
            0 => self.set_bc(val),
            1 => self.set_de(val),
            2 => self.set_hl(val),
            3 => self.set_sp(val),
            _ => unreachable!("ruh roh"),
        }
    }

    pub fn inc_sp(&mut self, val: u16) {
        self.sp = self.sp.wrapping_add(val);
    }

    pub fn dec_sp(&mut self, val: u16) {
        self.sp = self.sp.wrapping_sub(val);
    }

    pub fn set_flag_to(&mut self, flag: FlagBits, cond: bool) {
        if cond {
            self.f |= flag as u8
        } else {
            self.f &= !(flag as u8)
        }
    }

    pub fn get_flag(&self, flag: FlagBits) -> bool {
        (self.f & flag as u8) != 0
    }

    pub fn check_conditions(&self, condition: Condition) -> bool {
        use crate::gb::instructions::Condition::{C, NC, Z, NZ};
        
        match condition {
            NZ => !self.get_flag(FlagBits::Z),
            Z => self.get_flag(FlagBits::Z),
            NC => !self.get_flag(FlagBits::C),
            C => self.get_flag(FlagBits::C),
            _ => unreachable!(),
        }
    }
    
}

