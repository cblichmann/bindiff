use crate::graph::{CallGraph, FlowGraph, VERTEX_NAME};
use crate::fixed_points::{FixedPoints, FixedPoint};
use crate::types::Address;
use std::collections::HashMap;

pub struct MatchingContext<'a> {
    pub primary_call_graph: &'a CallGraph,
    pub secondary_call_graph: &'a CallGraph,
    pub primary_flow_graphs: &'a [FlowGraph],
    pub secondary_flow_graphs: &'a [FlowGraph],
    pub fixed_points: FixedPoints,
    pub fixed_points_by_primary: HashMap<Address, usize>,
    pub fixed_points_by_secondary: HashMap<Address, usize>,
}

impl<'a> MatchingContext<'a> {
    pub fn new(
        primary_call_graph: &'a CallGraph,
        secondary_call_graph: &'a CallGraph,
        primary_flow_graphs: &'a [FlowGraph],
        secondary_flow_graphs: &'a [FlowGraph],
    ) -> Self {
        Self {
            primary_call_graph,
            secondary_call_graph,
            primary_flow_graphs,
            secondary_flow_graphs,
            fixed_points: FixedPoints::new(),
            fixed_points_by_primary: HashMap::new(),
            fixed_points_by_secondary: HashMap::new(),
        }
    }

    pub fn add_fixed_point(&mut self, primary_addr: Address, secondary_addr: Address, step_name: &str) {
        if self.fixed_points_by_primary.contains_key(&primary_addr) || self.fixed_points_by_secondary.contains_key(&secondary_addr) {
            return;
        }

        let fp = FixedPoint {
            primary_address: primary_addr,
            secondary_address: secondary_addr,
            matching_step: step_name.to_string(),
            basic_block_fixed_points: Vec::new(),
            confidence: 1.0, // Default to 1.0 for simple steps
            similarity: 1.0,
            flags: 0,
            comments_ported: false,
        };

        self.fixed_points.push(fp);
        let idx = self.fixed_points.len() - 1;
        self.fixed_points_by_primary.insert(primary_addr, idx);
        self.fixed_points_by_secondary.insert(secondary_addr, idx);
    }
}

pub fn match_by_name(context: &mut MatchingContext) {
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_name = HashMap::new();
    for fg in context.secondary_flow_graphs {
        if context.fixed_points_by_secondary.contains_key(&fg.entry_point_address) {
            continue;
        }
        if let Some(node_idx) = fg.call_graph_vertex {
            let name = &context.secondary_call_graph.graph[node_idx].name;
            let flags = context.secondary_call_graph.graph[node_idx].flags;
            if flags & VERTEX_NAME != 0 {
                *secondary_counts.entry(name.clone()).or_insert(0) += 1;
                secondary_by_name.insert(name.clone(), fg.entry_point_address);
            }
        }
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_name = HashMap::new();
    for fg in context.primary_flow_graphs {
        if context.fixed_points_by_primary.contains_key(&fg.entry_point_address) {
            continue;
        }
        if let Some(node_idx) = fg.call_graph_vertex {
            let name = &context.primary_call_graph.graph[node_idx].name;
            let flags = context.primary_call_graph.graph[node_idx].flags;
            if flags & VERTEX_NAME != 0 {
                *primary_counts.entry(name.clone()).or_insert(0) += 1;
                primary_by_name.insert(name.clone(), fg.entry_point_address);
            }
        }
    }

    for (name, &prim_addr) in &primary_by_name {
        if primary_counts[name] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(name) {
                if sec_counts == 1 {
                    let sec_addr = secondary_by_name[name];
                    context.add_fixed_point(prim_addr, sec_addr, "function: name hash matching");
                }
            }
        }
    }
}

pub fn match_by_hash(context: &mut MatchingContext) {
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_hash = HashMap::new();
    for fg in context.secondary_flow_graphs {
        if context.fixed_points_by_secondary.contains_key(&fg.entry_point_address) {
            continue;
        }
        let hash = fg.byte_hash;
        *secondary_counts.entry(hash).or_insert(0) += 1;
        secondary_by_hash.insert(hash, fg.entry_point_address);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_hash = HashMap::new();
    for fg in context.primary_flow_graphs {
        if context.fixed_points_by_primary.contains_key(&fg.entry_point_address) {
            continue;
        }
        let hash = fg.byte_hash;
        *primary_counts.entry(hash).or_insert(0) += 1;
        primary_by_hash.insert(hash, fg.entry_point_address);
    }

    for (hash, &prim_addr) in &primary_by_hash {
        if primary_counts[hash] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(hash) {
                if sec_counts == 1 {
                    let sec_addr = secondary_by_hash[hash];
                    context.add_fixed_point(prim_addr, sec_addr, "function: hash matching");
                }
            }
        }
    }
}

pub fn match_by_prime(context: &mut MatchingContext) {
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_prime = HashMap::new();
    for fg in context.secondary_flow_graphs {
        if context.fixed_points_by_secondary.contains_key(&fg.entry_point_address) {
            continue;
        }
        let prime = fg.prime;
        if prime == 0 {
            continue;
        }
        *secondary_counts.entry(prime).or_insert(0) += 1;
        secondary_by_prime.insert(prime, fg.entry_point_address);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_prime = HashMap::new();
    for fg in context.primary_flow_graphs {
        if context.fixed_points_by_primary.contains_key(&fg.entry_point_address) {
            continue;
        }
        let prime = fg.prime;
        if prime == 0 {
            continue;
        }
        *primary_counts.entry(prime).or_insert(0) += 1;
        primary_by_prime.insert(prime, fg.entry_point_address);
    }

    for (prime, &prim_addr) in &primary_by_prime {
        if primary_counts[prime] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(prime) {
                if sec_counts == 1 {
                    let sec_addr = secondary_by_prime[prime];
                    context.add_fixed_point(prim_addr, sec_addr, "function: prime signature matching");
                }
            }
        }
    }
}

pub fn match_by_flow_graph_md_index(context: &mut MatchingContext, inverted: bool) {
    let step_name = if inverted {
        "function: MD index matching (flowgraph MD index, bottom up)"
    } else {
        "function: MD index matching (flowgraph MD index, top down)"
    };

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_md = HashMap::new();
    for fg in context.secondary_flow_graphs {
        if context.fixed_points_by_secondary.contains_key(&fg.entry_point_address) {
            continue;
        }
        let md = if inverted { fg.md_index_inverted } else { fg.md_index };
        if md == 0.0 {
            continue;
        }
        let md_bits = md.to_bits();
        *secondary_counts.entry(md_bits).or_insert(0) += 1;
        secondary_by_md.insert(md_bits, fg.entry_point_address);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_md = HashMap::new();
    for fg in context.primary_flow_graphs {
        if context.fixed_points_by_primary.contains_key(&fg.entry_point_address) {
            continue;
        }
        let md = if inverted { fg.md_index_inverted } else { fg.md_index };
        if md == 0.0 {
            continue;
        }
        let md_bits = md.to_bits();
        *primary_counts.entry(md_bits).or_insert(0) += 1;
        primary_by_md.insert(md_bits, fg.entry_point_address);
    }

    for (md_bits, &prim_addr) in &primary_by_md {
        if primary_counts[md_bits] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(md_bits) {
                if sec_counts == 1 {
                    let sec_addr = secondary_by_md[md_bits];
                    context.add_fixed_point(prim_addr, sec_addr, step_name);
                }
            }
        }
    }
}

pub fn match_by_call_graph_md_index(context: &mut MatchingContext, inverted: bool) {
    let step_name = if inverted {
        "function: MD index matching (callGraph MD index, bottom up)"
    } else {
        "function: MD index matching (callGraph MD index, top down)"
    };

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_md = HashMap::new();
    for fg in context.secondary_flow_graphs {
        if context.fixed_points_by_secondary.contains_key(&fg.entry_point_address) {
            continue;
        }
        if let Some(node_idx) = fg.call_graph_vertex {
            let md = context.secondary_call_graph.get_vertex_md_index(node_idx, inverted);
            if md == 0.0 {
                continue;
            }
            let md_bits = md.to_bits();
            *secondary_counts.entry(md_bits).or_insert(0) += 1;
            secondary_by_md.insert(md_bits, fg.entry_point_address);
        }
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_md = HashMap::new();
    for fg in context.primary_flow_graphs {
        if context.fixed_points_by_primary.contains_key(&fg.entry_point_address) {
            continue;
        }
        if let Some(node_idx) = fg.call_graph_vertex {
            let md = context.primary_call_graph.get_vertex_md_index(node_idx, inverted);
            if md == 0.0 {
                continue;
            }
            let md_bits = md.to_bits();
            *primary_counts.entry(md_bits).or_insert(0) += 1;
            primary_by_md.insert(md_bits, fg.entry_point_address);
        }
    }

    for (md_bits, &prim_addr) in &primary_by_md {
        if primary_counts[md_bits] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(md_bits) {
                if sec_counts == 1 {
                    let sec_addr = secondary_by_md[md_bits];
                    context.add_fixed_point(prim_addr, sec_addr, step_name);
                }
            }
        }
    }
}

pub fn diff(context: &mut MatchingContext) {
    match_by_name(context);
    match_by_hash(context);
    match_by_prime(context);
    match_by_flow_graph_md_index(context, false); // Top down
    match_by_flow_graph_md_index(context, true);  // Bottom up
    match_by_call_graph_md_index(context, false); // Top down
    match_by_call_graph_md_index(context, true);  // Bottom up
}
