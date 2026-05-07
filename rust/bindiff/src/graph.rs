use crate::instruction::Instruction;
use crate::types::Address;
use crate::binexport::BinExport2;
use crate::prime_signature::get_prime;
use petgraph::graph::{DiGraph, NodeIndex, EdgeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashSet, VecDeque};
use std::cmp::min;
use anyhow::{Result, anyhow, bail};

pub const EDGE_DUPLICATE: u32 = 1;

pub const VERTEX_LIBRARY: u32 = 1 << 0;
pub const VERTEX_STUB: u32 = 1 << 1;
pub const VERTEX_NAME: u32 = 1 << 2;
pub const VERTEX_DEMANGLED_NAME: u32 = 1 << 3;

pub const EDGE_UNCONDITIONAL: u8 = 1 << 0;
pub const EDGE_TRUE: u8 = 1 << 1;
pub const EDGE_FALSE: u8 = 1 << 2;
pub const EDGE_SWITCH: u8 = 1 << 3;
pub const EDGE_DOMINATED: u8 = 1 << 4;

pub const VERTEX_LOOPENTRY: u32 = 1 << 31;

// CallGraph definitions
#[derive(Debug, Clone)]
pub struct CallGraphVertexInfo {
    pub address: Address,
    pub name: String,
    pub demangled_name: String,
    pub bfs_top_down: u32,
    pub bfs_bottom_up: u32,
    pub flags: u32,
}

#[derive(Debug, Clone)]
pub struct CallGraphEdgeInfo {
    pub flags: u32,
    pub md_index_proximity: f64,
    pub md_index_top_down: f64,
    pub md_index_bottom_up: f64,
}

pub type CallGraphType = DiGraph<CallGraphVertexInfo, CallGraphEdgeInfo, u32>;

#[derive(Debug, Clone)]
pub struct CallGraph {
    pub graph: CallGraphType,
    pub md_index: f64,
    pub exe_filename: String,
    pub exe_hash: String,
    pub filename: String,
}

impl Default for CallGraph {
    fn default() -> Self {
        Self {
            graph: DiGraph::default(),
            md_index: 0.0,
            exe_filename: String::new(),
            exe_hash: String::new(),
            filename: String::new(),
        }
    }
}

impl CallGraph {
    pub fn reset(&mut self) {
        self.graph.clear();
        self.md_index = 0.0;
        self.exe_filename.clear();
        self.exe_hash.clear();
        self.filename.clear();
    }

    pub fn get_vertex(&self, address: Address) -> Option<NodeIndex<u32>> {
        let mut first = 0;
        let last = self.graph.node_count();
        let mut count = last;
        while count > 0 {
            let count2 = count / 2;
            let mid = first + count2;
            let mid_idx = NodeIndex::new(mid);
            if self.graph[mid_idx].address < address {
                first = mid + 1;
                count -= count2 + 1;
            } else {
                count = count2;
            }
        }

        if first != last && self.graph[NodeIndex::new(first)].address == address {
            Some(NodeIndex::new(first))
        } else {
            None
        }
    }

    pub fn read(&mut self, proto: &BinExport2, filename: &str) -> Result<()> {
        self.filename = filename.replace('\\', "/");

        let meta = proto.meta_information.as_ref()
            .ok_or_else(|| anyhow!("Missing meta information in proto"))?;
        self.exe_hash = meta.executable_id().to_string();
        self.exe_filename = meta.executable_name().to_string();

        let proto_call_graph = proto.call_graph.as_ref()
            .ok_or_else(|| anyhow!("Missing call graph in proto"))?;

        let vertex_count = proto_call_graph.vertex.len();
        let mut temp_vertices = Vec::with_capacity(vertex_count);
        let mut temp_addresses = Vec::with_capacity(vertex_count);
        let mut last_address = 0;

        for proto_vertex in &proto_call_graph.vertex {
            let address = proto_vertex.address();
            if address < last_address {
                bail!("Call graph nodes not sorted: {:X} >= {:X}", address, last_address);
            }
            last_address = address;
            temp_addresses.push(address);

            let mut flags = 0;
            let mut name = String::new();
            let mut demangled_name = String::new();

            if proto_vertex.mangled_name.is_some() {
                flags |= VERTEX_NAME;
                name = proto_vertex.mangled_name().to_string();
            }
            if proto_vertex.demangled_name.is_some() {
                assert!(proto_vertex.mangled_name.is_some());
                flags |= VERTEX_NAME;
                flags |= VERTEX_DEMANGLED_NAME;
                demangled_name = proto_vertex.demangled_name().to_string();
            }
            if flags & VERTEX_NAME == 0 {
                name = format!("sub_{:X}", address);
            }

            if proto_vertex.r#type == Some(1) { // LIBRARY
                flags |= VERTEX_LIBRARY;
            } else if proto_vertex.r#type == Some(3) { // THUNK
                flags |= VERTEX_STUB;
            }

            temp_vertices.push(CallGraphVertexInfo {
                address,
                name,
                demangled_name,
                bfs_top_down: 0,
                bfs_bottom_up: 0,
                flags,
            });
        }

        self.graph.clear();
        for vertex_info in temp_vertices {
            self.graph.add_node(vertex_info);
        }

        for proto_edge in &proto_call_graph.edge {
            let source_address = proto_call_graph.vertex[proto_edge.source_vertex_index() as usize].address();
            let target_address = proto_call_graph.vertex[proto_edge.target_vertex_index() as usize].address();

            let source_idx = temp_addresses.binary_search(&source_address)
                .map_err(|_| anyhow!("Source address not found: {:X}", source_address))?;
            let target_idx = temp_addresses.binary_search(&target_address)
                .map_err(|_| anyhow!("Target address not found: {:X}", target_address))?;

            self.graph.add_edge(
                NodeIndex::new(source_idx),
                NodeIndex::new(target_idx),
                CallGraphEdgeInfo {
                    flags: 0,
                    md_index_proximity: -1.0,
                    md_index_top_down: 0.0,
                    md_index_bottom_up: 0.0,
                },
            );
        }

        self.init();
        Ok(())
    }

    fn init(&mut self) {
        let mut seen_edges = HashSet::new();
        let edge_indices: Vec<_> = self.graph.edge_indices().collect();
        for edge_idx in edge_indices {
            let (source, target) = self.graph.edge_endpoints(edge_idx).unwrap();
            if !seen_edges.insert((source, target)) {
                self.graph[edge_idx].flags |= EDGE_DUPLICATE;
            }
        }

        self.calculate_topology();
        self.md_index = self.calculate_md_index(false);
        self.calculate_md_index(true);
    }

    fn calculate_topology(&mut self) {
        self.breadth_first_search();
        self.inverted_breadth_first_search();
    }

    fn breadth_first_search(&mut self) {
        let mut queue = VecDeque::new();
        let node_indices: Vec<_> = self.graph.node_indices().collect();
        for &node_idx in &node_indices {
            self.graph[node_idx].bfs_top_down = 0;
            if self.graph.edges_directed(node_idx, petgraph::Direction::Incoming).count() == 0 {
                queue.push_back(node_idx);
            }
        }

        while let Some(node_idx) = queue.pop_front() {
            let current_bfs = self.graph[node_idx].bfs_top_down;
            let neighbors: Vec<_> = self.graph.edges_directed(node_idx, petgraph::Direction::Outgoing)
                .map(|edge| edge.target())
                .collect();
            for neighbor in neighbors {
                if neighbor != NodeIndex::new(0) && self.graph[neighbor].bfs_top_down == 0 {
                    self.graph[neighbor].bfs_top_down = current_bfs + 1;
                    queue.push_back(neighbor);
                }
            }
        }
    }

    fn inverted_breadth_first_search(&mut self) {
        let mut queue = VecDeque::new();
        let node_indices: Vec<_> = self.graph.node_indices().collect();
        for &node_idx in &node_indices {
            self.graph[node_idx].bfs_bottom_up = 0;
            if self.graph.edges_directed(node_idx, petgraph::Direction::Outgoing).count() == 0 {
                queue.push_back(node_idx);
            }
        }

        while let Some(node_idx) = queue.pop_front() {
            let current_bfs = self.graph[node_idx].bfs_bottom_up;
            let neighbors: Vec<_> = self.graph.edges_directed(node_idx, petgraph::Direction::Incoming)
                .map(|edge| edge.source())
                .collect();
            for neighbor in neighbors {
                if self.graph[neighbor].bfs_bottom_up == 0 {
                    self.graph[neighbor].bfs_bottom_up = current_bfs + 1;
                    queue.push_back(neighbor);
                }
            }
        }
    }

    fn calculate_md_index(&mut self, inverted: bool) -> f64 {
        let weights = [2.0, 3.0, 5.0, 7.0, 11.0, 13.0];
        let edge_indices: Vec<_> = self.graph.edge_indices().collect();
        let mut md_indices = Vec::with_capacity(edge_indices.len());

        for &edge_idx in &edge_indices {
            let md = self.calculate_edge_md_index(edge_idx, inverted, &weights);
            md_indices.push(md);
            if inverted {
                self.graph[edge_idx].md_index_bottom_up = md;
            } else {
                self.graph[edge_idx].md_index_top_down = md;
            }
        }

        md_indices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        md_indices.iter().sum()
    }

    fn calculate_edge_md_index(&self, edge_idx: EdgeIndex<u32>, inverted: bool, weights: &[f64; 6]) -> f64 {
        let (source, target) = self.graph.edge_endpoints(edge_idx).unwrap();
        let in_degree_source = self.graph.edges_directed(source, petgraph::Direction::Incoming).count() as f64;
        let out_degree_source = self.graph.edges_directed(source, petgraph::Direction::Outgoing).count() as f64;
        let level_source = if inverted {
            self.graph[source].bfs_bottom_up as f64
        } else {
            self.graph[source].bfs_top_down as f64
        };

        let in_degree_target = self.graph.edges_directed(target, petgraph::Direction::Incoming).count() as f64;
        let out_degree_target = self.graph.edges_directed(target, petgraph::Direction::Outgoing).count() as f64;
        let level_target = if inverted {
            self.graph[target].bfs_bottom_up as f64
        } else {
            self.graph[target].bfs_top_down as f64
        };

        let md_index = weights[0].sqrt() * in_degree_source
            + weights[1].sqrt() * out_degree_source
            + weights[2].sqrt() * in_degree_target
            + weights[3].sqrt() * out_degree_target
            + weights[4].sqrt() * level_source
            + weights[5].sqrt() * level_target;

        if md_index != 0.0 {
            1.0 / md_index
        } else {
            0.0
        }
    }

    pub fn get_vertex_md_index(&self, vertex: NodeIndex<u32>, inverted: bool) -> f64 {
        let mut md_indices = Vec::new();
        for edge in self.graph.edges_directed(vertex, petgraph::Direction::Incoming) {
            let md = if inverted { edge.weight().md_index_bottom_up } else { edge.weight().md_index_top_down };
            md_indices.push(md);
        }
        for edge in self.graph.edges_directed(vertex, petgraph::Direction::Outgoing) {
            let md = if inverted { edge.weight().md_index_bottom_up } else { edge.weight().md_index_top_down };
            md_indices.push(md);
        }
        md_indices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        md_indices.iter().sum()
    }

    pub fn get_vertex_md_index_relaxed(&self, vertex: NodeIndex<u32>) -> f64 {
        let weights = [2.0, 3.0, 5.0, 7.0, 0.0, 0.0];
        let mut md_indices = Vec::new();
        for edge in self.graph.edges_directed(vertex, petgraph::Direction::Incoming) {
            let md = self.calculate_edge_md_index(edge.id(), false, &weights);
            md_indices.push(md);
        }
        for edge in self.graph.edges_directed(vertex, petgraph::Direction::Outgoing) {
            let md = self.calculate_edge_md_index(edge.id(), false, &weights);
            md_indices.push(md);
        }
        md_indices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        md_indices.iter().sum()
    }

    pub fn calculate_proximity_md_index(&self, edge: EdgeIndex<u32>) -> f64 {
        use std::collections::{HashSet, HashMap};
        
        let (source, target) = self.graph.edge_endpoints(edge).unwrap();
        let mut neighbors = HashSet::new();
        neighbors.insert(source);
        neighbors.insert(target);

        for n in self.graph.neighbors_directed(source, petgraph::Direction::Incoming) {
            neighbors.insert(n);
        }
        for n in self.graph.neighbors_directed(target, petgraph::Direction::Incoming) {
            neighbors.insert(n);
        }
        for n in self.graph.neighbors_directed(source, petgraph::Direction::Outgoing) {
            neighbors.insert(n);
        }
        for n in self.graph.neighbors_directed(target, petgraph::Direction::Outgoing) {
            neighbors.insert(n);
        }

        let mut degrees = HashMap::new();
        let mut internal_edges = HashSet::new();

        for &neighbor in &neighbors {
            let mut in_degree = 0;
            for in_edge in self.graph.edges_directed(neighbor, petgraph::Direction::Incoming) {
                if neighbors.contains(&in_edge.source()) {
                    in_degree += 1;
                    internal_edges.insert(in_edge.id());
                }
            }

            let mut out_degree = 0;
            for out_edge in self.graph.edges_directed(neighbor, petgraph::Direction::Outgoing) {
                if neighbors.contains(&out_edge.target()) {
                    out_degree += 1;
                    internal_edges.insert(out_edge.id());
                }
            }

            degrees.insert(neighbor, (in_degree as f64, out_degree as f64));
        }

        let mut md_indices = Vec::new();
        for edge_idx in internal_edges {
            let (src, tgt) = self.graph.edge_endpoints(edge_idx).unwrap();
            let &(src_in, src_out) = degrees.get(&src).unwrap();
            let &(tgt_in, tgt_out) = degrees.get(&tgt).unwrap();

            let md = 2.0f64.sqrt() * src_in
                + 3.0f64.sqrt() * src_out
                + 5.0f64.sqrt() * tgt_in
                + 7.0f64.sqrt() * tgt_out;

            if md != 0.0 {
                md_indices.push(1.0 / md);
            } else {
                md_indices.push(0.0);
            }
        }

        md_indices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        md_indices.iter().sum()
    }
}

// FlowGraph definitions
#[derive(Debug, Clone)]
pub struct FlowGraphVertexInfo {
    pub prime: u64,
    pub flags: u32,
    pub string_hash: u32,
    pub basic_block_hash: u32,
    pub instruction_start: u32,
    pub call_target_start: u32,
    pub bfs_top_down: u16,
    pub bfs_bottom_up: u16,
}

#[derive(Debug, Clone)]
pub struct FlowGraphEdgeInfo {
    pub md_index_top_down: f64,
    pub md_index_bottom_up: f64,
    pub flags: u8,
}

pub type FlowGraphType = DiGraph<FlowGraphVertexInfo, FlowGraphEdgeInfo, u32>;

#[derive(Debug, Clone)]
pub struct FlowGraph {
    pub graph: FlowGraphType,
    pub md_index: f64,
    pub md_index_inverted: f64,
    pub entry_point_address: Address,
    pub prime: u64,
    pub byte_hash: u32,
    pub string_references: u32,
    pub num_loops: u16,
    pub instructions: Vec<Instruction>,
    pub call_targets: Vec<Address>,
    pub call_graph_vertex: Option<NodeIndex<u32>>,
    pub level_for_call: Vec<(Address, (u16, u16))>,
}

impl Default for FlowGraph {
    fn default() -> Self {
        Self {
            graph: DiGraph::default(),
            md_index: 0.0,
            md_index_inverted: 0.0,
            entry_point_address: 0,
            prime: 0,
            byte_hash: 0,
            string_references: 0,
            num_loops: 0,
            instructions: Vec::new(),
            call_targets: Vec::new(),
            call_graph_vertex: None,
            level_for_call: Vec::new(),
        }
    }
}

fn get_sdbm_hash(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0;
    for &b in bytes {
        hash = (b as u32)
            .wrapping_add(hash.wrapping_shl(6))
            .wrapping_add(hash.wrapping_shl(16))
            .wrapping_sub(hash);
    }
    hash
}

fn proto_to_flow_graph_edge_type(edge_type: i32) -> u8 {
    match edge_type {
        1 => EDGE_TRUE,
        2 => EDGE_FALSE,
        3 => EDGE_UNCONDITIONAL,
        4 => EDGE_SWITCH,
        _ => EDGE_UNCONDITIONAL,
    }
}

impl FlowGraph {
    pub fn get_call_targets(&self, vertex: NodeIndex<u32>) -> &[Address] {
        let start = self.graph[vertex].call_target_start as usize;
        let call_len = self.call_targets.len();
        let end = if vertex.index() + 1 < self.graph.node_count() {
            let next_start = self.graph[NodeIndex::new(vertex.index() + 1)].call_target_start as usize;
            if next_start == u32::MAX as usize {
                call_len
            } else {
                next_start
            }
        } else {
            call_len
        };
        &self.call_targets[start..std::cmp::max(start, end)]
    }

    pub fn get_instructions(&self, vertex: NodeIndex<u32>) -> &[Instruction] {
        let start = self.graph[vertex].instruction_start as usize;
        let inst_len = self.instructions.len();
        let end = if vertex.index() + 1 < self.graph.node_count() {
            let next_start = self.graph[NodeIndex::new(vertex.index() + 1)].instruction_start as usize;
            if next_start == u32::MAX as usize {
                inst_len
            } else {
                next_start
            }
        } else {
            inst_len
        };
        &self.instructions[start..std::cmp::max(start, end)]
    }

    pub fn get_vertex(&self, address: Address) -> Option<NodeIndex<u32>> {
        let mut first = 0;
        let last = self.graph.node_count();
        let mut count = last;
        while count > 0 {
            let count2 = count / 2;
            let mid = first + count2;
            let mid_idx = NodeIndex::new(mid);
            if self.get_address(mid_idx) < address {
                first = mid + 1;
                count -= count2 + 1;
            } else {
                count = count2;
            }
        }

        if first != last && self.get_address(NodeIndex::new(first)) == address {
            Some(NodeIndex::new(first))
        } else {
            None
        }
    }

    pub fn get_children(&self, vertex: NodeIndex<u32>) -> Vec<NodeIndex<u32>> {
        self.graph.neighbors_directed(vertex, petgraph::Direction::Outgoing).collect()
    }

    pub fn get_parents(&self, vertex: NodeIndex<u32>) -> Vec<NodeIndex<u32>> {
        self.graph.neighbors_directed(vertex, petgraph::Direction::Incoming).collect()
    }

    pub fn get_vertex_md_index(&self, vertex: NodeIndex<u32>, inverted: bool) -> f64 {
        let mut md_indices = Vec::new();
        for edge in self.graph.edges_directed(vertex, petgraph::Direction::Incoming) {
            let md = if inverted { edge.weight().md_index_bottom_up } else { edge.weight().md_index_top_down };
            md_indices.push(md);
        }
        for edge in self.graph.edges_directed(vertex, petgraph::Direction::Outgoing) {
            let md = if inverted { edge.weight().md_index_bottom_up } else { edge.weight().md_index_top_down };
            md_indices.push(md);
        }
        md_indices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        md_indices.iter().sum()
    }

    pub fn get_vertex_md_index_relaxed(&self, vertex: NodeIndex<u32>) -> f64 {
        let weights = [2.0, 3.0, 5.0, 7.0, 0.0, 0.0];
        let mut md_indices = Vec::new();
        for edge in self.graph.edges_directed(vertex, petgraph::Direction::Incoming) {
            let md = self.calculate_edge_md_index(edge.id(), false, &weights);
            md_indices.push(md);
        }
        for edge in self.graph.edges_directed(vertex, petgraph::Direction::Outgoing) {
            let md = self.calculate_edge_md_index(edge.id(), false, &weights);
            md_indices.push(md);
        }
        md_indices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        md_indices.iter().sum()
    }

    pub fn get_address(&self, vertex: NodeIndex<u32>) -> Address {
        self.get_instructions(vertex)[0].address
    }

    pub fn calculate_call_levels(&mut self) {
        self.level_for_call.clear();
        let node_indices: Vec<_> = self.graph.node_indices().collect();
        for node_idx in node_indices {
            let calls = self.get_call_targets(node_idx).to_vec();
            if calls.is_empty() {
                continue;
            }

            let level = self.graph[node_idx].bfs_top_down;
            for (sequence, &target) in calls.iter().enumerate() {
                self.level_for_call.push((target, (level, sequence as u16)));
            }
        }
        self.level_for_call.shrink_to_fit();
        self.level_for_call.sort_by_key(|x| x.0);
    }

    pub fn get_level_for_call_address(&self, address: Address) -> (u16, u16) {
        let mut min_level = (u16::MAX, u16::MAX);
        let idx = match self.level_for_call.binary_search_by_key(&address, |x| x.0) {
            Ok(i) => {
                let mut first = i;
                while first > 0 && self.level_for_call[first - 1].0 == address {
                    first -= 1;
                }
                first
            }
            Err(_) => return min_level,
        };

        for i in idx..self.level_for_call.len() {
            let entry = &self.level_for_call[i];
            if entry.0 != address {
                break;
            }
            let lvl = entry.1;
            if lvl.0 < min_level.0 || (lvl.0 == min_level.0 && lvl.1 < min_level.1) {
                min_level = lvl;
            }
        }
        min_level
    }

    pub fn read(
        &mut self,
        proto: &BinExport2,
        proto_flow_graph: &crate::binexport::bin_export2::FlowGraph,
        call_graph: &CallGraph,
        instruction_addresses: &[Address],
    ) -> Result<()> {
        let entry_bb_idx = proto_flow_graph.entry_basic_block_index.unwrap_or(0) as usize;
        let entry_bb = &proto.basic_block[entry_bb_idx];
        let entry_instr_idx = entry_bb.instruction_index[0].begin_index.unwrap_or(0) as usize;
        self.entry_point_address = instruction_addresses[entry_instr_idx];

        self.call_graph_vertex = call_graph.get_vertex(self.entry_point_address);

        self.prime = 0;
        self.string_references = 1;

        let mut computed_instruction_address = 0;
        let mut last_instruction_index: i32 = -2;
        let mut function_bytes = Vec::new();

        let bb_count = proto_flow_graph.basic_block_index.len();
        let mut temp_vertices = Vec::with_capacity(bb_count);
        let mut temp_addresses = Vec::with_capacity(bb_count);

        for &bb_idx in &proto_flow_graph.basic_block_index {
            let proto_basic_block = &proto.basic_block[bb_idx as usize];
            let mut basic_block_bytes = Vec::new();

            let mut vertex_info = FlowGraphVertexInfo {
                prime: 0,
                flags: 0,
                string_hash: 0,
                basic_block_hash: 0,
                instruction_start: self.instructions.len() as u32,
                call_target_start: u32::MAX,
                bfs_top_down: 0,
                bfs_bottom_up: 0,
            };

            for interval in &proto_basic_block.instruction_index {
                let begin = interval.begin_index.unwrap_or(0) as usize;
                let end = if interval.end_index.is_some() {
                    interval.end_index() as usize
                } else {
                    begin + 1
                };

                for instr_idx in begin..end {
                    let proto_instruction = &proto.instruction[instr_idx];
                    let instruction_address;

                    if last_instruction_index == (instr_idx as i32 - 1) && proto_instruction.address.is_none() {
                        instruction_address = computed_instruction_address;
                    } else {
                        instruction_address = instruction_addresses[instr_idx];
                    }

                    computed_instruction_address = instruction_address + proto_instruction.raw_bytes.as_deref().unwrap_or(&[]).len() as u64;
                    last_instruction_index = instr_idx as i32;

                    let mnemonic = proto.mnemonic[proto_instruction.mnemonic_index.unwrap_or(0) as usize].name();
                    let instruction_prime = get_prime(mnemonic);
                    vertex_info.prime += instruction_prime as u64;

                    self.instructions.push(Instruction {
                        address: instruction_address,
                        prime: instruction_prime,
                        mnemonic_index: proto_instruction.mnemonic_index.unwrap_or(0) as u32,
                    });

                    basic_block_bytes.extend_from_slice(proto_instruction.raw_bytes.as_deref().unwrap_or(&[]));

                    if !proto_instruction.call_target.is_empty() && vertex_info.call_target_start == u32::MAX {
                        vertex_info.call_target_start = self.call_targets.len() as u32;
                    }
                    for &target in &proto_instruction.call_target {
                        self.call_targets.push(target);
                    }
                }
            }

            temp_addresses.push(self.instructions[vertex_info.instruction_start as usize].address);
            self.prime += vertex_info.prime;
            vertex_info.basic_block_hash = get_sdbm_hash(&basic_block_bytes);
            function_bytes.extend_from_slice(&basic_block_bytes);
            temp_vertices.push(vertex_info);
        }

        self.byte_hash = get_sdbm_hash(&function_bytes);

        // Verify sorted
        for i in 0..temp_addresses.len() - 1 {
            if temp_addresses[i] > temp_addresses[i + 1] {
                bail!("Basic blocks not sorted by address");
            }
        }

        let edge_count = proto_flow_graph.edge.len();
        let mut edges = Vec::with_capacity(edge_count);
        let mut edge_properties = Vec::with_capacity(edge_count);

        for proto_edge in &proto_flow_graph.edge {
            let source_bb_idx = proto_edge.source_basic_block_index.unwrap_or(0) as usize;
            let target_bb_idx = proto_edge.target_basic_block_index.unwrap_or(0) as usize;
            let source_bb = &proto.basic_block[source_bb_idx];
            let target_bb = &proto.basic_block[target_bb_idx];

            let source_address = instruction_addresses[source_bb.instruction_index[0].begin_index.unwrap_or(0) as usize];
            let target_address = instruction_addresses[target_bb.instruction_index[0].begin_index.unwrap_or(0) as usize];

            let source_idx = temp_addresses.binary_search(&source_address)
                .map_err(|_| anyhow!("Source basic block address not found: {:X}", source_address))?;
            let target_idx = temp_addresses.binary_search(&target_address)
                .map_err(|_| anyhow!("Target basic block address not found: {:X}", target_address))?;

            edges.push((source_idx as u32, target_idx as u32));
            edge_properties.push(FlowGraphEdgeInfo {
                md_index_top_down: 0.0,
                md_index_bottom_up: 0.0,
                flags: proto_to_flow_graph_edge_type(proto_edge.r#type.unwrap_or(3)),
            });
        }

        // Construct graph
        self.graph.clear();
        for vertex_info in temp_vertices {
            self.graph.add_node(vertex_info);
        }
        for (i, (source, target)) in edges.into_iter().enumerate() {
            self.graph.add_edge(NodeIndex::new(source as usize), NodeIndex::new(target as usize), edge_properties[i].clone());
        }

        self.init();
        Ok(())
    }

    fn init(&mut self) {
        self.instructions.shrink_to_fit();
        self.call_targets.shrink_to_fit();

        let inst_len = self.instructions.len() as u32;
        let call_len = self.call_targets.len() as u32;

        for node_idx in self.graph.node_indices() {
            self.graph[node_idx].instruction_start = min(self.graph[node_idx].instruction_start, inst_len);
            self.graph[node_idx].call_target_start = min(self.graph[node_idx].call_target_start, call_len);
        }

        self.calculate_topology();
        self.md_index = self.calculate_md_index(false);
        self.md_index_inverted = self.calculate_md_index(true);
        self.calculate_call_levels();
        self.mark_loops();
    }

    fn calculate_topology(&mut self) {
        self.breadth_first_search();
        self.inverted_breadth_first_search();
    }

    fn breadth_first_search(&mut self) {
        let mut queue = VecDeque::new();
        for node_idx in self.graph.node_indices() {
            self.graph[node_idx].bfs_top_down = 0;
            if self.graph.edges_directed(node_idx, petgraph::Direction::Incoming).count() == 0 {
                queue.push_back(node_idx);
            }
        }

        while let Some(node_idx) = queue.pop_front() {
            let current_bfs = self.graph[node_idx].bfs_top_down;
            let neighbors: Vec<_> = self.graph.edges_directed(node_idx, petgraph::Direction::Outgoing)
                .map(|edge| edge.target())
                .collect();
            for neighbor in neighbors {
                if neighbor != NodeIndex::new(0) && self.graph[neighbor].bfs_top_down == 0 {
                    self.graph[neighbor].bfs_top_down = current_bfs + 1;
                    queue.push_back(neighbor);
                }
            }
        }
    }

    fn inverted_breadth_first_search(&mut self) {
        let mut queue = VecDeque::new();
        for node_idx in self.graph.node_indices() {
            self.graph[node_idx].bfs_bottom_up = 0;
            if self.graph.edges_directed(node_idx, petgraph::Direction::Outgoing).count() == 0 {
                queue.push_back(node_idx);
            }
        }

        while let Some(node_idx) = queue.pop_front() {
            let current_bfs = self.graph[node_idx].bfs_bottom_up;
            let neighbors: Vec<_> = self.graph.edges_directed(node_idx, petgraph::Direction::Incoming)
                .map(|edge| edge.source())
                .collect();
            for neighbor in neighbors {
                if self.graph[neighbor].bfs_bottom_up == 0 {
                    self.graph[neighbor].bfs_bottom_up = current_bfs + 1;
                    queue.push_back(neighbor);
                }
            }
        }
    }

    fn calculate_md_index(&mut self, inverted: bool) -> f64 {
        let weights = [2.0, 3.0, 5.0, 7.0, 11.0, 13.0];
        let edge_indices: Vec<_> = self.graph.edge_indices().collect();
        let mut md_indices = Vec::with_capacity(edge_indices.len());

        for &edge_idx in &edge_indices {
            let md = self.calculate_edge_md_index(edge_idx, inverted, &weights);
            md_indices.push(md);
            if inverted {
                self.graph[edge_idx].md_index_bottom_up = md;
            } else {
                self.graph[edge_idx].md_index_top_down = md;
            }
        }

        md_indices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        md_indices.iter().sum()
    }

    fn calculate_edge_md_index(&self, edge_idx: EdgeIndex<u32>, inverted: bool, weights: &[f64; 6]) -> f64 {
        let (source, target) = self.graph.edge_endpoints(edge_idx).unwrap();
        let in_degree_source = self.graph.edges_directed(source, petgraph::Direction::Incoming).count() as f64;
        let out_degree_source = self.graph.edges_directed(source, petgraph::Direction::Outgoing).count() as f64;
        let level_source = if inverted {
            self.graph[source].bfs_bottom_up as f64
        } else {
            self.graph[source].bfs_top_down as f64
        };

        let in_degree_target = self.graph.edges_directed(target, petgraph::Direction::Incoming).count() as f64;
        let out_degree_target = self.graph.edges_directed(target, petgraph::Direction::Outgoing).count() as f64;
        let level_target = if inverted {
            self.graph[target].bfs_bottom_up as f64
        } else {
            self.graph[target].bfs_top_down as f64
        };

        let md_index = weights[0].sqrt() * in_degree_source
            + weights[1].sqrt() * out_degree_source
            + weights[2].sqrt() * in_degree_target
            + weights[3].sqrt() * out_degree_target
            + weights[4].sqrt() * level_source
            + weights[5].sqrt() * level_target;

        if md_index != 0.0 {
            1.0 / md_index
        } else {
            0.0
        }
    }

    fn mark_loops(&mut self) {
        if self.graph.node_count() == 0 {
            return;
        }
        let dominators = petgraph::algo::dominators::simple_fast(&self.graph, NodeIndex::new(0));
        let edge_indices: Vec<_> = self.graph.edge_indices().collect();
        for edge_idx in edge_indices {
            let (source, target) = self.graph.edge_endpoints(edge_idx).unwrap();
            if let Some(mut doms) = dominators.dominators(source) {
                if doms.any(|dom| dom == target) {
                    self.graph[edge_idx].flags |= EDGE_DOMINATED;
                    self.graph[target].flags |= VERTEX_LOOPENTRY;
                    self.num_loops += 1;
                }
            }
        }
    }
}
