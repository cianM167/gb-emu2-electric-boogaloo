use serde::Deserialize;
use std::collections::HashMap;

use crate::gb::instructions::Delta;

#[derive(Deserialize)]
pub struct OpcodeFile {
    pub unprefixed: HashMap<String, RawOpcode>,
    pub cbprefixed: HashMap<String, RawOpcode>,
}

#[derive(Deserialize)]
pub struct RawOpcode {
    pub mnemonic: String,
    pub bytes: u8,
    pub cycles: Vec<u8>,
    pub operands: Vec<RawOperand>,
    pub immediate: bool,
    pub flags: RawFlags,
}

#[derive(Deserialize)]
pub struct RawOperand {
    pub name: String,
    #[serde(default)]
    pub immediate: bool,
    #[serde(default)]
    pub increment: bool,
    #[serde(default)]
    pub decrement: bool,
}

impl RawOperand {
    pub fn delta(&self) -> Delta {
        match (self.increment, self.decrement) {
            (true, false) => Delta::Increment,
            (false, true) => Delta::Decrement,
            (false, false) => Delta::None,
            (true, true) => panic!("operand {} has both increment and decrement", self.name),
        }
    }
}

#[derive(Deserialize)]
pub struct RawFlags {
    #[serde(rename = "Z")]
    pub z: String,
    #[serde(rename = "N")]
    pub n: String,
    #[serde(rename = "H")]
    pub h: String,
    #[serde(rename = "C")]
    pub c: String,
}