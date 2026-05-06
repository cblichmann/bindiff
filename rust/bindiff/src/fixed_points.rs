use crate::types::Address;
use petgraph::graph::NodeIndex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionMatch {
    pub primary_address: Address,
    pub secondary_address: Address,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlockFixedPoint {
    pub primary_vertex: NodeIndex<u32>,
    pub secondary_vertex: NodeIndex<u32>,
    pub matching_step: String,
    pub instruction_matches: Vec<InstructionMatch>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FixedPoint {
    pub primary_address: Address,
    pub secondary_address: Address,
    pub matching_step: String,
    pub basic_block_fixed_points: Vec<BasicBlockFixedPoint>,
    pub confidence: f64,
    pub similarity: f64,
    pub flags: i32,
    pub comments_ported: bool,
}

pub type FixedPoints = Vec<FixedPoint>;
pub type FixedPointRefs<'a> = Vec<&'a mut FixedPoint>;
