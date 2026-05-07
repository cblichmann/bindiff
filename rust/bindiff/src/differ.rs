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

        let prim_fg = self.primary_flow_graphs.iter().find(|fg| fg.entry_point_address == primary_addr).unwrap();
        let sec_fg = self.secondary_flow_graphs.iter().find(|fg| fg.entry_point_address == secondary_addr).unwrap();

        let mut fp = FixedPoint {
            primary_address: primary_addr,
            secondary_address: secondary_addr,
            matching_step: step_name.to_string(),
            basic_block_fixed_points: Vec::new(),
            confidence: 1.0,
            similarity: 1.0,
            flags: 0,
            comments_ported: false,
        };

        crate::basic_block_differ::find_fixed_points_basic_block(&mut fp, prim_fg, sec_fg, self);

        let bbs1 = prim_fg.graph.node_count();
        let bbs2 = sec_fg.graph.node_count();
        let matched_bbs = fp.basic_block_fixed_points.len();
        if bbs1 > 0 || bbs2 > 0 {
            fp.similarity = matched_bbs as f64 / std::cmp::max(bbs1, bbs2) as f64;
        } else {
            fp.similarity = 1.0;
        }
        fp.confidence = fp.similarity;

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

pub fn match_by_instruction_count(context: &mut MatchingContext) {
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_count = HashMap::new();
    for fg in context.secondary_flow_graphs {
        if context.fixed_points_by_secondary.contains_key(&fg.entry_point_address) {
            continue;
        }
        let count = fg.instructions.len() as u64;
        if count == 0 {
            continue;
        }
        *secondary_counts.entry(count).or_insert(0) += 1;
        secondary_by_count.insert(count, fg.entry_point_address);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_count = HashMap::new();
    for fg in context.primary_flow_graphs {
        if context.fixed_points_by_primary.contains_key(&fg.entry_point_address) {
            continue;
        }
        let count = fg.instructions.len() as u64;
        if count == 0 {
            continue;
        }
        *primary_counts.entry(count).or_insert(0) += 1;
        primary_by_count.insert(count, fg.entry_point_address);
    }

    for (count, &prim_addr) in &primary_by_count {
        if primary_counts[count] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(count) {
                if sec_counts == 1 {
                    let sec_addr = secondary_by_count[count];
                    context.add_fixed_point(prim_addr, sec_addr, "function: instruction count");
                }
            }
        }
    }
}

pub fn match_by_loop_count(context: &mut MatchingContext) {
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_loops = HashMap::new();
    for fg in context.secondary_flow_graphs {
        if context.fixed_points_by_secondary.contains_key(&fg.entry_point_address) {
            continue;
        }
        let loops = fg.num_loops as u64;
        if loops == 0 {
            continue;
        }
        *secondary_counts.entry(loops).or_insert(0) += 1;
        secondary_by_loops.insert(loops, fg.entry_point_address);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_loops = HashMap::new();
    for fg in context.primary_flow_graphs {
        if context.fixed_points_by_primary.contains_key(&fg.entry_point_address) {
            continue;
        }
        let loops = fg.num_loops as u64;
        if loops == 0 {
            continue;
        }
        *primary_counts.entry(loops).or_insert(0) += 1;
        primary_by_loops.insert(loops, fg.entry_point_address);
    }

    for (loops, &prim_addr) in &primary_by_loops {
        if primary_counts[loops] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(loops) {
                if sec_counts == 1 {
                    let sec_addr = secondary_by_loops[loops];
                    context.add_fixed_point(prim_addr, sec_addr, "function: loop count matching");
                }
            }
        }
    }
}

pub fn match_by_address_sequence(context: &mut MatchingContext) {
    let mut unmatched_primary: Vec<_> = context.primary_flow_graphs.iter()
        .filter(|fg| !context.fixed_points_by_primary.contains_key(&fg.entry_point_address))
        .collect();
    let mut unmatched_secondary: Vec<_> = context.secondary_flow_graphs.iter()
        .filter(|fg| !context.fixed_points_by_secondary.contains_key(&fg.entry_point_address))
        .collect();

    if unmatched_primary.is_empty() || unmatched_primary.len() != unmatched_secondary.len() {
        return;
    }

    let sort_key = |fg: &FlowGraph, fgs: &[FlowGraph]| {
        let seq = fgs.iter().position(|x| x.entry_point_address == fg.entry_point_address).unwrap();
        (fg.instructions.len(), seq)
    };

    let fgs1 = context.primary_flow_graphs;
    unmatched_primary.sort_by(|a, b| {
        let key_a = sort_key(a, fgs1);
        let key_b = sort_key(b, fgs1);
        let cmp = key_b.0.cmp(&key_a.0);
        if cmp == std::cmp::Ordering::Equal {
            key_b.1.cmp(&key_a.1)
        } else {
            cmp
        }
    });

    let fgs2 = context.secondary_flow_graphs;
    unmatched_secondary.sort_by(|a, b| {
        let key_a = sort_key(a, fgs2);
        let key_b = sort_key(b, fgs2);
        let cmp = key_b.0.cmp(&key_a.0);
        if cmp == std::cmp::Ordering::Equal {
            key_b.1.cmp(&key_a.1)
        } else {
            cmp
        }
    });

    for i in 0..unmatched_primary.len() {
        context.add_fixed_point(
            unmatched_primary[i].entry_point_address,
            unmatched_secondary[i].entry_point_address,
            "function: address sequence",
        );
    }
}

pub fn match_by_call_sequence(context: &mut MatchingContext, accuracy: u8) {
    let step_name = match accuracy {
        0 => "function: call sequence matching(exact)",
        1 => "function: call sequence matching(topology)",
        _ => "function: call sequence matching(sequence)",
    };

    let current_matches: Vec<_> = context.fixed_points.iter()
        .map(|fp| (fp.primary_address, fp.secondary_address))
        .collect();

    for (prim_parent_addr, sec_parent_addr) in current_matches {
        let prim_parent = context.primary_flow_graphs.iter().find(|fg| fg.entry_point_address == prim_parent_addr).unwrap();
        let sec_parent = context.secondary_flow_graphs.iter().find(|fg| fg.entry_point_address == sec_parent_addr).unwrap();

        let unmatched_children1: Vec<_> = prim_parent.call_targets.iter()
            .filter(|&&addr| !context.fixed_points_by_primary.contains_key(&addr))
            .cloned()
            .collect();
        let unmatched_children2: Vec<_> = sec_parent.call_targets.iter()
            .filter(|&&addr| !context.fixed_points_by_secondary.contains_key(&addr))
            .cloned()
            .collect();

        if unmatched_children1.is_empty() || unmatched_children2.is_empty() {
            continue;
        }

        let mut map1 = HashMap::new();
        for addr in unmatched_children1 {
            let lvl = prim_parent.get_level_for_call_address(addr);
            let index = match accuracy {
                0 => ((lvl.0 as u32) << 16) + lvl.1 as u32,
                1 => lvl.0 as u32,
                _ => 0,
            };
            map1.entry(index).or_insert_with(Vec::new).push(addr);
        }

        let mut map2 = HashMap::new();
        for addr in unmatched_children2 {
            let lvl = sec_parent.get_level_for_call_address(addr);
            let index = match accuracy {
                0 => ((lvl.0 as u32) << 16) + lvl.1 as u32,
                1 => lvl.0 as u32,
                _ => 0,
            };
            map2.entry(index).or_insert_with(Vec::new).push(addr);
        }

        if accuracy == 2 {
            let mut sorted_children1: Vec<_> = prim_parent.call_targets.iter()
                .filter(|&&addr| !context.fixed_points_by_primary.contains_key(&addr))
                .cloned()
                .collect();
            sorted_children1.sort_by_key(|&addr| {
                let lvl = prim_parent.get_level_for_call_address(addr);
                ((lvl.0 as u32) << 16) + lvl.1 as u32
            });

            let mut sorted_children2: Vec<_> = sec_parent.call_targets.iter()
                .filter(|&&addr| !context.fixed_points_by_secondary.contains_key(&addr))
                .cloned()
                .collect();
            sorted_children2.sort_by_key(|&addr| {
                let lvl = sec_parent.get_level_for_call_address(addr);
                ((lvl.0 as u32) << 16) + lvl.1 as u32
            });

            if sorted_children1.len() == sorted_children2.len() {
                for i in 0..sorted_children1.len() {
                    context.add_fixed_point(sorted_children1[i], sorted_children2[i], step_name);
                }
            }
        } else {
            for (index, addrs1) in &map1 {
                if addrs1.len() == 1 {
                    if let Some(addrs2) = map2.get(index) {
                        if addrs2.len() == 1 {
                            context.add_fixed_point(addrs1[0], addrs2[0], step_name);
                        }
                    }
                }
            }
        }
    }
}

pub fn diff(context: &mut MatchingContext) {
    let skip_name = std::env::var("BINDIFF_DEV_SKIP_NAME_MATCHING")
        .map(|v| v == "1" || v.to_lowercase() == "true" || v.is_empty())
        .unwrap_or(false);

    if !skip_name {
        match_by_name(context);
    } else {
        println!("BINDIFF_DEV_SKIP_NAME_MATCHING is enabled: skipping function name matching!");
    }
    match_by_hash(context);
    match_by_prime(context);
    match_by_edges_flowgraph_md_index(context);
    match_by_flow_graph_md_index(context, false); // Top down
    match_by_flow_graph_md_index(context, true);  // Bottom up
    match_by_edges_callgraph_md_index(context);
    match_by_edges_proximity_md_index(context);
    match_by_call_graph_md_index(context, false); // Top down
    match_by_call_graph_md_index(context, true);  // Bottom up
    match_by_relaxed_md_index(context);
    match_by_instruction_count(context);
    match_by_loop_count(context);
    match_by_call_sequence(context, 0); // Exact call sequence
    match_by_call_sequence(context, 1); // Topology call sequence
    match_by_call_sequence(context, 2); // Sequence call sequence
    match_by_address_sequence(context);
}

pub fn match_by_relaxed_md_index(context: &mut MatchingContext) {
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_md = HashMap::new();
    for fg in context.secondary_flow_graphs {
        if context.fixed_points_by_secondary.contains_key(&fg.entry_point_address) {
            continue;
        }
        if let Some(node_idx) = fg.call_graph_vertex {
            let md = context.secondary_call_graph.get_vertex_md_index_relaxed(node_idx);
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
            let md = context.primary_call_graph.get_vertex_md_index_relaxed(node_idx);
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
                    context.add_fixed_point(prim_addr, sec_addr, "function: relaxed MD index matching");
                }
            }
        }
    }
}

pub fn match_by_edges_flowgraph_md_index(context: &mut MatchingContext) {
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    
    for edge in context.secondary_call_graph.graph.edge_indices() {
        let (sec_source_v, sec_target_v) = context.secondary_call_graph.graph.edge_endpoints(edge).unwrap();
        let sec_source_addr = context.secondary_call_graph.graph[sec_source_v].address;
        let sec_target_addr = context.secondary_call_graph.graph[sec_target_v].address;

        if context.fixed_points_by_secondary.contains_key(&sec_source_addr)
            || context.fixed_points_by_secondary.contains_key(&sec_target_addr)
        {
            continue;
        }

        let sec_source_fg = context.secondary_flow_graphs.iter().find(|fg| fg.entry_point_address == sec_source_addr).unwrap();
        let sec_target_fg = context.secondary_flow_graphs.iter().find(|fg| fg.entry_point_address == sec_target_addr).unwrap();
        
        let md_src = sec_source_fg.md_index;
        let md_tgt = sec_target_fg.md_index;
        if md_src == 0.0 || md_tgt == 0.0 {
            continue;
        }

        let sig = (md_src.to_bits(), md_tgt.to_bits());
        *secondary_counts.entry(sig).or_insert(0) += 1;
        secondary_by_sig.insert(sig, (sec_source_addr, sec_target_addr));
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    
    for edge in context.primary_call_graph.graph.edge_indices() {
        let (prim_source_v, prim_target_v) = context.primary_call_graph.graph.edge_endpoints(edge).unwrap();
        let prim_source_addr = context.primary_call_graph.graph[prim_source_v].address;
        let prim_target_addr = context.primary_call_graph.graph[prim_target_v].address;

        if context.fixed_points_by_primary.contains_key(&prim_source_addr)
            || context.fixed_points_by_primary.contains_key(&prim_target_addr)
        {
            continue;
        }

        let prim_source_fg = context.primary_flow_graphs.iter().find(|fg| fg.entry_point_address == prim_source_addr).unwrap();
        let prim_target_fg = context.primary_flow_graphs.iter().find(|fg| fg.entry_point_address == prim_target_addr).unwrap();
        
        let md_src = prim_source_fg.md_index;
        let md_tgt = prim_target_fg.md_index;
        if md_src == 0.0 || md_tgt == 0.0 {
            continue;
        }

        let sig = (md_src.to_bits(), md_tgt.to_bits());
        *primary_counts.entry(sig).or_insert(0) += 1;
        primary_by_sig.insert(sig, (prim_source_addr, prim_target_addr));
    }

    for (sig, &(prim_src, prim_tgt)) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let (sec_src, sec_tgt) = secondary_by_sig[sig];
                    
                    context.add_fixed_point(prim_src, sec_src, "function: edges flowgraph MD index");
                    context.add_fixed_point(prim_tgt, sec_tgt, "function: edges flowgraph MD index");
                }
            }
        }
    }
}

pub fn match_by_edges_callgraph_md_index(context: &mut MatchingContext) {
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    
    for edge in context.secondary_call_graph.graph.edge_indices() {
        let (sec_source_v, sec_target_v) = context.secondary_call_graph.graph.edge_endpoints(edge).unwrap();
        let sec_source_addr = context.secondary_call_graph.graph[sec_source_v].address;
        let sec_target_addr = context.secondary_call_graph.graph[sec_target_v].address;

        if context.fixed_points_by_secondary.contains_key(&sec_source_addr)
            || context.fixed_points_by_secondary.contains_key(&sec_target_addr)
        {
            continue;
        }

        let md = context.secondary_call_graph.graph[edge].md_index_top_down;
        if md == 0.0 {
            continue;
        }

        let sig = md.to_bits();
        *secondary_counts.entry(sig).or_insert(0) += 1;
        secondary_by_sig.insert(sig, (sec_source_addr, sec_target_addr));
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    
    for edge in context.primary_call_graph.graph.edge_indices() {
        let (prim_source_v, prim_target_v) = context.primary_call_graph.graph.edge_endpoints(edge).unwrap();
        let prim_source_addr = context.primary_call_graph.graph[prim_source_v].address;
        let prim_target_addr = context.primary_call_graph.graph[prim_target_v].address;

        if context.fixed_points_by_primary.contains_key(&prim_source_addr)
            || context.fixed_points_by_primary.contains_key(&prim_target_addr)
        {
            continue;
        }

        let md = context.primary_call_graph.graph[edge].md_index_top_down;
        if md == 0.0 {
            continue;
        }

        let sig = md.to_bits();
        *primary_counts.entry(sig).or_insert(0) += 1;
        primary_by_sig.insert(sig, (prim_source_addr, prim_target_addr));
    }

    for (sig, &(prim_src, prim_tgt)) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let (sec_src, sec_tgt) = secondary_by_sig[sig];
                    
                    context.add_fixed_point(prim_src, sec_src, "function: edges callgraph MD index");
                    context.add_fixed_point(prim_tgt, sec_tgt, "function: edges callgraph MD index");
                }
            }
        }
    }
}

pub fn match_by_edges_proximity_md_index(context: &mut MatchingContext) {
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    
    for edge in context.secondary_call_graph.graph.edge_indices() {
        let (sec_source_v, sec_target_v) = context.secondary_call_graph.graph.edge_endpoints(edge).unwrap();
        let sec_source_addr = context.secondary_call_graph.graph[sec_source_v].address;
        let sec_target_addr = context.secondary_call_graph.graph[sec_target_v].address;

        if context.fixed_points_by_secondary.contains_key(&sec_source_addr)
            || context.fixed_points_by_secondary.contains_key(&sec_target_addr)
        {
            continue;
        }

        let md = context.secondary_call_graph.calculate_proximity_md_index(edge);
        if md == 0.0 {
            continue;
        }

        let sig = md.to_bits();
        *secondary_counts.entry(sig).or_insert(0) += 1;
        secondary_by_sig.insert(sig, (sec_source_addr, sec_target_addr));
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    
    for edge in context.primary_call_graph.graph.edge_indices() {
        let (prim_source_v, prim_target_v) = context.primary_call_graph.graph.edge_endpoints(edge).unwrap();
        let prim_source_addr = context.primary_call_graph.graph[prim_source_v].address;
        let prim_target_addr = context.primary_call_graph.graph[prim_target_v].address;

        if context.fixed_points_by_primary.contains_key(&prim_source_addr)
            || context.fixed_points_by_primary.contains_key(&prim_target_addr)
        {
            continue;
        }

        let md = context.primary_call_graph.calculate_proximity_md_index(edge);
        if md == 0.0 {
            continue;
        }

        let sig = md.to_bits();
        *primary_counts.entry(sig).or_insert(0) += 1;
        primary_by_sig.insert(sig, (prim_source_addr, prim_target_addr));
    }

    for (sig, &(prim_src, prim_tgt)) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let (sec_src, sec_tgt) = secondary_by_sig[sig];
                    
                    context.add_fixed_point(prim_src, sec_src, "function: edges proximity MD index");
                    context.add_fixed_point(prim_tgt, sec_tgt, "function: edges proximity MD index");
                }
            }
        }
    }
}
