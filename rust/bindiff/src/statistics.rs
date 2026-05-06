use crate::graph::{CallGraph, FlowGraph, VERTEX_LIBRARY};
use crate::fixed_points::{FixedPoints, FixedPoint};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct Counts {
    pub basic_block_matches_library: u64,
    pub basic_block_matches_non_library: u64,
    pub basic_blocks_primary_library: u64,
    pub basic_blocks_primary_non_library: u64,
    pub basic_blocks_secondary_library: u64,
    pub basic_blocks_secondary_non_library: u64,
    pub flow_graph_edge_matches_library: u64,
    pub flow_graph_edge_matches_non_library: u64,
    pub flow_graph_edges_primary_library: u64,
    pub flow_graph_edges_primary_non_library: u64,
    pub flow_graph_edges_secondary_library: u64,
    pub flow_graph_edges_secondary_non_library: u64,
    pub function_matches_library: u64,
    pub function_matches_non_library: u64,
    pub functions_primary_library: u64,
    pub functions_primary_non_library: u64,
    pub functions_secondary_library: u64,
    pub functions_secondary_non_library: u64,
    pub instruction_matches_library: u64,
    pub instruction_matches_non_library: u64,
    pub instructions_primary_library: u64,
    pub instructions_primary_non_library: u64,
    pub instructions_secondary_library: u64,
    pub instructions_secondary_non_library: u64,

    pub basic_blocks_library: u64,
    pub basic_blocks_non_library: u64,
    pub edges_library: u64,
    pub edges_non_library: u64,
    pub functions_library: u64,
    pub functions_non_library: u64,
    pub instructions_library: u64,
    pub instructions_non_library: u64,
}

pub type Histogram = HashMap<String, usize>;

impl Counts {
    pub fn get_display_entries(&self) -> Vec<(&'static str, u64)> {
        vec![
            ("Basic Block Matches (Library)", self.basic_block_matches_library),
            ("Basic Block Matches (Non-Library)", self.basic_block_matches_non_library),
            ("Basic Blocks Primary (Library)", self.basic_blocks_primary_library),
            ("Basic Blocks Primary (Non-Library)", self.basic_blocks_primary_non_library),
            ("Basic Blocks Secondary (Library)", self.basic_blocks_secondary_library),
            ("Basic Blocks Secondary (Non-Library)", self.basic_blocks_secondary_non_library),
            ("Flow Graph Edge Matches (Library)", self.flow_graph_edge_matches_library),
            ("Flow Graph Edge Matches (Non-Library)", self.flow_graph_edge_matches_non_library),
            ("Flow Graph Edges Primary (Library)", self.flow_graph_edges_primary_library),
            ("Flow Graph Edges Primary (Non-Library)", self.flow_graph_edges_primary_non_library),
            ("Flow Graph Edges Secondary (Library)", self.flow_graph_edges_secondary_library),
            ("Flow Graph Edges Secondary (Non-Library)", self.flow_graph_edges_secondary_non_library),
            ("Function Matches (Library)", self.function_matches_library),
            ("Function Matches (Non-Library)", self.function_matches_non_library),
            ("Functions Primary (Library)", self.functions_primary_library),
            ("Functions Primary (Non-Library)", self.functions_primary_non_library),
            ("Functions Secondary (Library)", self.functions_secondary_library),
            ("Functions Secondary (Non-Library)", self.functions_secondary_non_library),
            ("Instruction Matches (Library)", self.instruction_matches_library),
            ("Instruction Matches (Non-Library)", self.instruction_matches_non_library),
            ("Instructions Primary (Library)", self.instructions_primary_library),
            ("Instructions Primary (Non-Library)", self.instructions_primary_non_library),
            ("Instructions Secondary (Library)", self.instructions_secondary_library),
            ("Instructions Secondary (Non-Library)", self.instructions_secondary_non_library),
        ]
    }
}

pub fn is_library(fg: &FlowGraph, cg: &CallGraph) -> bool {
    if let Some(node_idx) = fg.call_graph_vertex {
        cg.graph[node_idx].flags & VERTEX_LIBRARY != 0
    } else {
        false
    }
}

pub fn count_graphs(flow_graphs: &[FlowGraph], cg: &CallGraph, counts: &mut Counts) {
    for fg in flow_graphs {
        let lib = is_library(fg, cg);
        if lib {
            counts.functions_library += 1;
            counts.basic_blocks_library += fg.graph.node_count() as u64;
            counts.edges_library += fg.graph.edge_count() as u64;
            counts.instructions_library += fg.instructions.len() as u64;
        } else {
            counts.functions_non_library += 1;
            counts.basic_blocks_non_library += fg.graph.node_count() as u64;
            counts.edges_non_library += fg.graph.edge_count() as u64;
            counts.instructions_non_library += fg.instructions.len() as u64;
        }
    }
}

pub fn count_fixed_point(
    fp: &FixedPoint,
    cg1: &CallGraph,
    cg2: &CallGraph,
    fgs1: &[FlowGraph],
    fgs2: &[FlowGraph],
    counts: &mut Counts,
    histogram: &mut Histogram,
) {
    let fg1 = fgs1.iter().find(|fg| fg.entry_point_address == fp.primary_address).unwrap();
    let fg2 = fgs2.iter().find(|fg| fg.entry_point_address == fp.secondary_address).unwrap();

    let library = is_library(fg1, cg1) || is_library(fg2, cg2);

    *histogram.entry(fp.matching_step.clone()).or_insert(0) += 1;

    if library {
        counts.function_matches_library += 1;
        counts.basic_block_matches_library += fp.basic_block_fixed_points.len() as u64;
        for bb in &fp.basic_block_fixed_points {
            *histogram.entry(bb.matching_step.clone()).or_insert(0) += 1;
            counts.instruction_matches_library += bb.instruction_matches.len() as u64;
        }
    } else {
        counts.function_matches_non_library += 1;
        counts.basic_block_matches_non_library += fp.basic_block_fixed_points.len() as u64;
        for bb in &fp.basic_block_fixed_points {
            *histogram.entry(bb.matching_step.clone()).or_insert(0) += 1;
            counts.instruction_matches_non_library += bb.instruction_matches.len() as u64;
        }
    }
}

pub fn get_counts_and_histogram(
    cg1: &CallGraph,
    cg2: &CallGraph,
    flow_graphs1: &[FlowGraph],
    flow_graphs2: &[FlowGraph],
    fixed_points: &FixedPoints,
    histogram: &mut Histogram,
    counts: &mut Counts,
) {
    let mut counts1 = Counts::default();
    let mut counts2 = Counts::default();
    count_graphs(flow_graphs1, cg1, &mut counts1);
    count_graphs(flow_graphs2, cg2, &mut counts2);

    counts.functions_primary_library = counts1.functions_library;
    counts.functions_primary_non_library = counts1.functions_non_library;
    counts.functions_secondary_library = counts2.functions_library;
    counts.functions_secondary_non_library = counts2.functions_non_library;

    counts.basic_blocks_primary_library = counts1.basic_blocks_library;
    counts.basic_blocks_primary_non_library = counts1.basic_blocks_non_library;
    counts.basic_blocks_secondary_library = counts2.basic_blocks_library;
    counts.basic_blocks_secondary_non_library = counts2.basic_blocks_non_library;

    counts.instructions_primary_library = counts1.instructions_library;
    counts.instructions_primary_non_library = counts1.instructions_non_library;
    counts.instructions_secondary_library = counts2.instructions_library;
    counts.instructions_secondary_non_library = counts2.instructions_non_library;

    counts.flow_graph_edges_primary_library = counts1.edges_library;
    counts.flow_graph_edges_primary_non_library = counts1.edges_non_library;
    counts.flow_graph_edges_secondary_library = counts2.edges_library;
    counts.flow_graph_edges_secondary_non_library = counts2.edges_non_library;

    for fp in fixed_points {
        count_fixed_point(fp, cg1, cg2, flow_graphs1, flow_graphs2, counts, histogram);
    }
}
