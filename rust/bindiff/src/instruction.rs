use crate::types::Address;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub address: Address,
    pub prime: u32,
    pub mnemonic_index: u32,
}

pub type InstructionCache = HashMap<u32, String>;

pub type Instructions = Vec<Instruction>;
pub type InstructionMatches<'a> = Vec<(&'a Instruction, &'a Instruction)>;
