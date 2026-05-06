pub type Address = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowGraphInfo {
    pub address: Address,
    pub name: Option<String>,
    pub demangled_name: Option<String>,
    pub basic_block_count: i32,
    pub edge_count: i32,
    pub instruction_count: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FixedPointInfo {
    pub primary: Address,
    pub secondary: Address,
    pub basic_block_count: i32,
    pub edge_count: i32,
    pub instruction_count: i32,
    pub similarity: f64,
    pub confidence: f64,
    pub flags: i32,
    pub algorithm: Option<String>,
    pub evaluate: bool,
    pub comments_ported: bool,
}
