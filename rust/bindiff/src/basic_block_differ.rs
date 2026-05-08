use crate::graph::FlowGraph;
use crate::fixed_points::{FixedPoint, BasicBlockFixedPoint};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::collections::{HashSet, HashMap};

pub fn find_fixed_points_basic_block(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    context: &crate::differ::MatchingContext,
) {
    let mut unmatched_primary: HashSet<_> = primary.graph.node_indices().collect();
    let mut unmatched_secondary: HashSet<_> = secondary.graph.node_indices().collect();

    match_basic_blocks_by_hash(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
    );

    match_basic_blocks_by_md_index(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
        false,
    );

    match_basic_blocks_by_md_index(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
        true,
    );

    match_basic_blocks_by_prime(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
        4,
    );

    match_basic_blocks_by_edges_prime(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
    );

    match_basic_blocks_by_edges_md_index(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
        false,
    );

    match_basic_blocks_by_edges_md_index(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
        true,
    );

    match_basic_blocks_by_edges_loop(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
    );

    match_basic_blocks_by_call_refs(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
        context,
    );

    match_basic_blocks_by_string_refs(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
    );

    match_basic_blocks_by_md_index_relaxed(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
    );

    match_basic_blocks_by_loop_entry(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
    );

    match_basic_blocks_by_self_loops(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
    );

    match_basic_blocks_by_entry_nodes(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
        false,
    );

    match_basic_blocks_by_entry_nodes(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
        true,
    );

    match_basic_blocks_by_instruction_count(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
    );

    match_basic_blocks_by_jump_sequence(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
    );

    match_basic_blocks_by_prime(
        fixed_point,
        primary,
        secondary,
        &mut unmatched_primary,
        &mut unmatched_secondary,
        0,
    );

    let mut more_discovered = true;
    while more_discovered {
        more_discovered = propagate_basic_blocks(
            fixed_point,
            primary,
            secondary,
            &mut unmatched_primary,
            &mut unmatched_secondary,
        );
    }
}

fn match_basic_blocks_by_hash(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
) {
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_hash = HashMap::new();
    for &v in unmatched_secondary.iter() {
        let hash = secondary.graph[v].basic_block_hash;
        if hash == 0 {
            continue;
        }
        *secondary_counts.entry(hash).or_insert(0) += 1;
        secondary_by_hash.insert(hash, v);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_hash = HashMap::new();
    for &v in unmatched_primary.iter() {
        let hash = primary.graph[v].basic_block_hash;
        if hash == 0 {
            continue;
        }
        *primary_counts.entry(hash).or_insert(0) += 1;
        primary_by_hash.insert(hash, v);
    }

    for (hash, &prim_v) in &primary_by_hash {
        if primary_counts[hash] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(hash) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_hash[hash];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: "basicBlock: hash matching".to_string(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

fn match_basic_blocks_by_prime(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
    min_instructions: usize,
) {
    let step_name = format!("basicBlock: prime matching ({} instructions minimum)", min_instructions);

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_prime = HashMap::new();
    for &v in unmatched_secondary.iter() {
        if secondary.get_instructions(v).len() < min_instructions {
            continue;
        }
        let prime = secondary.graph[v].prime;
        if prime == 0 {
            continue;
        }
        *secondary_counts.entry(prime).or_insert(0) += 1;
        secondary_by_prime.insert(prime, v);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_prime = HashMap::new();
    for &v in unmatched_primary.iter() {
        if primary.get_instructions(v).len() < min_instructions {
            continue;
        }
        let prime = primary.graph[v].prime;
        if prime == 0 {
            continue;
        }
        *primary_counts.entry(prime).or_insert(0) += 1;
        primary_by_prime.insert(prime, v);
    }

    for (prime, &prim_v) in &primary_by_prime {
        if primary_counts[prime] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(prime) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_prime[prime];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: step_name.clone(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

fn match_basic_blocks_by_md_index(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
    inverted: bool,
) {
    let step_name = if inverted {
        "basicBlock: MD index matching (bottom up)"
    } else {
        "basicBlock: MD index matching (top down)"
    };

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_md = HashMap::new();
    for &v in unmatched_secondary.iter() {
        let md = secondary.get_vertex_md_index(v, inverted);
        if md == 0.0 {
            continue;
        }
        let md_bits = md.to_bits();
        *secondary_counts.entry(md_bits).or_insert(0) += 1;
        secondary_by_md.insert(md_bits, v);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_md = HashMap::new();
    for &v in unmatched_primary.iter() {
        let md = primary.get_vertex_md_index(v, inverted);
        if md == 0.0 {
            continue;
        }
        let md_bits = md.to_bits();
        *primary_counts.entry(md_bits).or_insert(0) += 1;
        primary_by_md.insert(md_bits, v);
    }

    for (md_bits, &prim_v) in &primary_by_md {
        if primary_counts[md_bits] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(md_bits) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_md[md_bits];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: step_name.to_string(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

fn match_basic_blocks_by_md_index_relaxed(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
) {
    let step_name = "basicBlock: relaxed MD index matching";

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_md = HashMap::new();
    for &v in unmatched_secondary.iter() {
        let md = secondary.get_vertex_md_index_relaxed(v);
        if md == 0.0 {
            continue;
        }
        let md_bits = md.to_bits();
        *secondary_counts.entry(md_bits).or_insert(0) += 1;
        secondary_by_md.insert(md_bits, v);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_md = HashMap::new();
    for &v in unmatched_primary.iter() {
        let md = primary.get_vertex_md_index_relaxed(v);
        if md == 0.0 {
            continue;
        }
        let md_bits = md.to_bits();
        *primary_counts.entry(md_bits).or_insert(0) += 1;
        primary_by_md.insert(md_bits, v);
    }

    for (md_bits, &prim_v) in &primary_by_md {
        if primary_counts[md_bits] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(md_bits) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_md[md_bits];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: step_name.to_string(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

fn match_basic_blocks_by_call_refs(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
    context: &crate::differ::MatchingContext,
) {
    let step_name = "basicBlock: call reference matching";

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_ref = HashMap::new();
    for &v in unmatched_secondary.iter() {
        let calls = secondary.get_call_targets(v);
        if calls.is_empty() {
            continue;
        }
        
        let mut ref_feature = 0u64;
        let mut valid = true;
        for (i, &addr) in calls.iter().enumerate() {
            if let Some(&fp_idx) = context.fixed_points_by_secondary.get(&addr) {
                let fp = &context.fixed_points[fp_idx];
                ref_feature = ref_feature.wrapping_add(
                    (i + 1) as u64 * (fp.primary_address.wrapping_add(fp.secondary_address))
                );
            } else {
                valid = false;
                break;
            }
        }
        if valid && ref_feature > 0 {
            *secondary_counts.entry(ref_feature).or_insert(0) += 1;
            secondary_by_ref.insert(ref_feature, v);
        }
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_ref = HashMap::new();
    for &v in unmatched_primary.iter() {
        let calls = primary.get_call_targets(v);
        if calls.is_empty() {
            continue;
        }
        
        let mut ref_feature = 0u64;
        let mut valid = true;
        for (i, &addr) in calls.iter().enumerate() {
            if let Some(&fp_idx) = context.fixed_points_by_primary.get(&addr) {
                let fp = &context.fixed_points[fp_idx];
                ref_feature = ref_feature.wrapping_add(
                    (i + 1) as u64 * (fp.primary_address.wrapping_add(fp.secondary_address))
                );
            } else {
                valid = false;
                break;
            }
        }
        if valid && ref_feature > 0 {
            *primary_counts.entry(ref_feature).or_insert(0) += 1;
            primary_by_ref.insert(ref_feature, v);
        }
    }

    for (ref_feature, &prim_v) in &primary_by_ref {
        if primary_counts[ref_feature] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(ref_feature) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_ref[ref_feature];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: step_name.to_string(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

fn match_basic_blocks_by_self_loops(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
) {
    let step_name = "basicBlock: self loop matching";

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_loops = HashMap::new();
    for &v in unmatched_secondary.iter() {
        let self_loops = secondary.graph.edges_directed(v, petgraph::Direction::Outgoing)
            .filter(|edge| edge.target() == v)
            .count();
        if self_loops == 0 {
            continue;
        }
        *secondary_counts.entry(self_loops).or_insert(0) += 1;
        secondary_by_loops.insert(self_loops, v);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_loops = HashMap::new();
    for &v in unmatched_primary.iter() {
        let self_loops = primary.graph.edges_directed(v, petgraph::Direction::Outgoing)
            .filter(|edge| edge.target() == v)
            .count();
        if self_loops == 0 {
            continue;
        }
        *primary_counts.entry(self_loops).or_insert(0) += 1;
        primary_by_loops.insert(self_loops, v);
    }

    for (self_loops, &prim_v) in &primary_by_loops {
        if primary_counts[self_loops] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(self_loops) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_loops[self_loops];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: step_name.to_string(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

fn match_basic_blocks_by_instruction_count(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
) {
    let step_name = "basicBlock: instruction count matching";

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    for &v in unmatched_secondary.iter() {
        let insts = secondary.get_instructions(v).len() as u64;
        let md = secondary.get_vertex_md_index(v, false);
        let sig = (md.to_bits(), insts);
        *secondary_counts.entry(sig).or_insert(0) += 1;
        secondary_by_sig.insert(sig, v);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    for &v in unmatched_primary.iter() {
        let insts = primary.get_instructions(v).len() as u64;
        let md = primary.get_vertex_md_index(v, false);
        let sig = (md.to_bits(), insts);
        *primary_counts.entry(sig).or_insert(0) += 1;
        primary_by_sig.insert(sig, v);
    }

    for (sig, &prim_v) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_sig[sig];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: step_name.to_string(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

fn match_basic_blocks_by_string_refs(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
) {
    let step_name = "basicBlock: string references matching";

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    for &v in unmatched_secondary.iter() {
        let sig = secondary.graph[v].string_hash;
        if sig <= 1 {
            continue;
        }
        *secondary_counts.entry(sig).or_insert(0) += 1;
        secondary_by_sig.insert(sig, v);
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    for &v in unmatched_primary.iter() {
        let sig = primary.graph[v].string_hash;
        if sig <= 1 {
            continue;
        }
        *primary_counts.entry(sig).or_insert(0) += 1;
        primary_by_sig.insert(sig, v);
    }

    for (sig, &prim_v) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_sig[sig];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: step_name.to_string(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

fn match_basic_blocks_by_loop_entry(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
) {
    let step_name = "basicBlock: loop entry matching";

    let mut sorted_sec: Vec<_> = unmatched_secondary.iter().cloned().collect();
    sorted_sec.sort_by_key(|v| v.index());
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    let mut loop_idx = 0u64;
    for v in sorted_sec {
        if secondary.graph[v].flags & crate::graph::VERTEX_LOOPENTRY != 0 {
            let sig = loop_idx;
            loop_idx += 1;
            *secondary_counts.entry(sig).or_insert(0) += 1;
            secondary_by_sig.insert(sig, v);
        }
    }

    let mut sorted_prim: Vec<_> = unmatched_primary.iter().cloned().collect();
    sorted_prim.sort_by_key(|v| v.index());
    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    let mut loop_idx = 0u64;
    for v in sorted_prim {
        if primary.graph[v].flags & crate::graph::VERTEX_LOOPENTRY != 0 {
            let sig = loop_idx;
            loop_idx += 1;
            *primary_counts.entry(sig).or_insert(0) += 1;
            primary_by_sig.insert(sig, v);
        }
    }

    for (sig, &prim_v) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_sig[sig];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: step_name.to_string(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

fn match_basic_blocks_by_entry_nodes(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
    inverted: bool,
) {
    let step_name = if inverted {
        "basicBlock: exit point matching"
    } else {
        "basicBlock: entry point matching"
    };

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    for &v in unmatched_secondary.iter() {
        let match_node = if inverted {
            secondary.graph.edges_directed(v, petgraph::Direction::Outgoing).count() == 0
        } else {
            secondary.graph.edges_directed(v, petgraph::Direction::Incoming).count() == 0
        };
        
        if match_node {
            let sig = 1u64;
            *secondary_counts.entry(sig).or_insert(0) += 1;
            secondary_by_sig.insert(sig, v);
        }
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    for &v in unmatched_primary.iter() {
        let match_node = if inverted {
            primary.graph.edges_directed(v, petgraph::Direction::Outgoing).count() == 0
        } else {
            primary.graph.edges_directed(v, petgraph::Direction::Incoming).count() == 0
        };
        
        if match_node {
            let sig = 1u64;
            *primary_counts.entry(sig).or_insert(0) += 1;
            primary_by_sig.insert(sig, v);
        }
    }

    for (sig, &prim_v) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_sig[sig];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: step_name.to_string(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

fn match_basic_blocks_by_jump_sequence(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
) {
    let step_name = "basicBlock: jump sequence matching";

    let mut sorted_sec: Vec<_> = unmatched_secondary.iter().cloned().collect();
    sorted_sec.sort_by_key(|v| v.index());
    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    let mut sec_md_counts = HashMap::new();
    for v in sorted_sec {
        let md = secondary.get_vertex_md_index(v, false);
        let md_bits = md.to_bits();
        let count = sec_md_counts.entry(md_bits).or_insert(0u64);
        let sig = (md_bits, *count);
        *count += 1;
        *secondary_counts.entry(sig).or_insert(0) += 1;
        secondary_by_sig.insert(sig, v);
    }

    let mut sorted_prim: Vec<_> = unmatched_primary.iter().cloned().collect();
    sorted_prim.sort_by_key(|v| v.index());
    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    let mut prim_md_counts = HashMap::new();
    for v in sorted_prim {
        let md = primary.get_vertex_md_index(v, false);
        let md_bits = md.to_bits();
        let count = prim_md_counts.entry(md_bits).or_insert(0u64);
        let sig = (md_bits, *count);
        *count += 1;
        *primary_counts.entry(sig).or_insert(0) += 1;
        primary_by_sig.insert(sig, v);
    }

    for (sig, &prim_v) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let sec_v = secondary_by_sig[sig];
                    
                    fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                        primary_vertex: prim_v,
                        secondary_vertex: sec_v,
                        matching_step: step_name.to_string(),
                        instruction_matches: Vec::new(),
                    });
                    unmatched_primary.remove(&prim_v);
                    unmatched_secondary.remove(&sec_v);
                }
            }
        }
    }
}

pub fn match_basic_blocks_by_edges_md_index(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
    inverted: bool,
) {
    let step_name = if inverted {
        "basicBlock: edges MD index (bottom up)"
    } else {
        "basicBlock: edges MD index (top down)"
    };

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    for edge in secondary.graph.edge_indices() {
        let (sec_source_v, sec_target_v) = secondary.graph.edge_endpoints(edge).unwrap();
        if sec_source_v == sec_target_v {
            continue;
        }
        if unmatched_secondary.contains(&sec_source_v) || unmatched_secondary.contains(&sec_target_v) {
            let md = if inverted {
                secondary.graph[edge].md_index_bottom_up
            } else {
                secondary.graph[edge].md_index_top_down
            };
            if md == 0.0 {
                continue;
            }
            let sig = md.to_bits();
            *secondary_counts.entry(sig).or_insert(0) += 1;
            secondary_by_sig.insert(sig, (sec_source_v, sec_target_v));
        }
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    for edge in primary.graph.edge_indices() {
        let (prim_source_v, prim_target_v) = primary.graph.edge_endpoints(edge).unwrap();
        if prim_source_v == prim_target_v {
            continue;
        }
        if unmatched_primary.contains(&prim_source_v) || unmatched_primary.contains(&prim_target_v) {
            let md = if inverted {
                primary.graph[edge].md_index_bottom_up
            } else {
                primary.graph[edge].md_index_top_down
            };
            if md == 0.0 {
                continue;
            }
            let sig = md.to_bits();
            *primary_counts.entry(sig).or_insert(0) += 1;
            primary_by_sig.insert(sig, (prim_source_v, prim_target_v));
        }
    }

    for (sig, &(prim_src, prim_tgt)) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let (sec_src, sec_tgt) = secondary_by_sig[sig];
                    
                    if unmatched_primary.contains(&prim_src) && unmatched_secondary.contains(&sec_src) {
                        fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                            primary_vertex: prim_src,
                            secondary_vertex: sec_src,
                            matching_step: step_name.to_string(),
                            instruction_matches: Vec::new(),
                        });
                        unmatched_primary.remove(&prim_src);
                        unmatched_secondary.remove(&sec_src);
                    }
                    if unmatched_primary.contains(&prim_tgt) && unmatched_secondary.contains(&sec_tgt) {
                        fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                            primary_vertex: prim_tgt,
                            secondary_vertex: sec_tgt,
                            matching_step: step_name.to_string(),
                            instruction_matches: Vec::new(),
                        });
                        unmatched_primary.remove(&prim_tgt);
                        unmatched_secondary.remove(&sec_tgt);
                    }
                }
            }
        }
    }
}

pub fn match_basic_blocks_by_edges_prime(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
) {
    let step_name = "basicBlock: edges prime product";

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    for edge in secondary.graph.edge_indices() {
        let (sec_source_v, sec_target_v) = secondary.graph.edge_endpoints(edge).unwrap();
        if sec_source_v == sec_target_v {
            continue;
        }
        if unmatched_secondary.contains(&sec_source_v) || unmatched_secondary.contains(&sec_target_v) {
            let sig = secondary.graph[sec_source_v].prime
                .wrapping_add(secondary.graph[sec_target_v].prime)
                .wrapping_add(1);
            *secondary_counts.entry(sig).or_insert(0) += 1;
            secondary_by_sig.insert(sig, (sec_source_v, sec_target_v));
        }
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    for edge in primary.graph.edge_indices() {
        let (prim_source_v, prim_target_v) = primary.graph.edge_endpoints(edge).unwrap();
        if prim_source_v == prim_target_v {
            continue;
        }
        if unmatched_primary.contains(&prim_source_v) || unmatched_primary.contains(&prim_target_v) {
            let sig = primary.graph[prim_source_v].prime
                .wrapping_add(primary.graph[prim_target_v].prime)
                .wrapping_add(1);
            *primary_counts.entry(sig).or_insert(0) += 1;
            primary_by_sig.insert(sig, (prim_source_v, prim_target_v));
        }
    }

    for (sig, &(prim_src, prim_tgt)) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let (sec_src, sec_tgt) = secondary_by_sig[sig];
                    
                    if unmatched_primary.contains(&prim_src) && unmatched_secondary.contains(&sec_src) {
                        fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                            primary_vertex: prim_src,
                            secondary_vertex: sec_src,
                            matching_step: step_name.to_string(),
                            instruction_matches: Vec::new(),
                        });
                        unmatched_primary.remove(&prim_src);
                        unmatched_secondary.remove(&sec_src);
                    }
                    if unmatched_primary.contains(&prim_tgt) && unmatched_secondary.contains(&sec_tgt) {
                        fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                            primary_vertex: prim_tgt,
                            secondary_vertex: sec_tgt,
                            matching_step: step_name.to_string(),
                            instruction_matches: Vec::new(),
                        });
                        unmatched_primary.remove(&prim_tgt);
                        unmatched_secondary.remove(&sec_tgt);
                    }
                }
            }
        }
    }
}

pub fn match_basic_blocks_by_edges_loop(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
) {
    let step_name = "basicBlock: edges Lengauer Tarjan dominated";

    let mut secondary_counts = HashMap::new();
    let mut secondary_by_sig = HashMap::new();
    for edge in secondary.graph.edge_indices() {
        let (sec_source_v, sec_target_v) = secondary.graph.edge_endpoints(edge).unwrap();
        if sec_source_v == sec_target_v {
            continue;
        }
        if unmatched_secondary.contains(&sec_source_v) || unmatched_secondary.contains(&sec_target_v) {
            if secondary.graph[edge].flags & crate::graph::EDGE_DOMINATED != 0 {
                let sig = 1u64;
                *secondary_counts.entry(sig).or_insert(0) += 1;
                secondary_by_sig.insert(sig, (sec_source_v, sec_target_v));
            }
        }
    }

    let mut primary_counts = HashMap::new();
    let mut primary_by_sig = HashMap::new();
    for edge in primary.graph.edge_indices() {
        let (prim_source_v, prim_target_v) = primary.graph.edge_endpoints(edge).unwrap();
        if prim_source_v == prim_target_v {
            continue;
        }
        if unmatched_primary.contains(&prim_source_v) || unmatched_primary.contains(&prim_target_v) {
            if primary.graph[edge].flags & crate::graph::EDGE_DOMINATED != 0 {
                let sig = 1u64;
                *primary_counts.entry(sig).or_insert(0) += 1;
                primary_by_sig.insert(sig, (prim_source_v, prim_target_v));
            }
        }
    }

    for (sig, &(prim_src, prim_tgt)) in &primary_by_sig {
        if primary_counts[sig] == 1 {
            if let Some(&sec_counts) = secondary_counts.get(sig) {
                if sec_counts == 1 {
                    let (sec_src, sec_tgt) = secondary_by_sig[sig];
                    
                    if unmatched_primary.contains(&prim_src) && unmatched_secondary.contains(&sec_src) {
                        fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                            primary_vertex: prim_src,
                            secondary_vertex: sec_src,
                            matching_step: step_name.to_string(),
                            instruction_matches: Vec::new(),
                        });
                        unmatched_primary.remove(&prim_src);
                        unmatched_secondary.remove(&sec_src);
                    }
                    if unmatched_primary.contains(&prim_tgt) && unmatched_secondary.contains(&sec_tgt) {
                        fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                            primary_vertex: prim_tgt,
                            secondary_vertex: sec_tgt,
                            matching_step: step_name.to_string(),
                            instruction_matches: Vec::new(),
                        });
                        unmatched_primary.remove(&prim_tgt);
                        unmatched_secondary.remove(&sec_tgt);
                    }
                }
            }
        }
    }
}

fn propagate_basic_blocks(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
    unmatched_primary: &mut HashSet<NodeIndex<u32>>,
    unmatched_secondary: &mut HashSet<NodeIndex<u32>>,
) -> bool {
    let mut discovered = false;
    
    let current_matches: Vec<_> = fixed_point.basic_block_fixed_points.iter()
        .map(|fp| (fp.primary_vertex, fp.secondary_vertex))
        .collect();

    for (prim_v, sec_v) in current_matches {
        // Propagation down (children)
        let c1: Vec<_> = primary.get_children(prim_v).into_iter()
            .filter(|v| unmatched_primary.contains(v))
            .collect();
        let c2: Vec<_> = secondary.get_children(sec_v).into_iter()
            .filter(|v| unmatched_secondary.contains(v))
            .collect();

        if c1.len() == 1 && c2.len() == 1 {
            let child1 = c1[0];
            let child2 = c2[0];
            fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                primary_vertex: child1,
                secondary_vertex: child2,
                matching_step: "basicBlock: propagation (size==1)".to_string(),
                instruction_matches: Vec::new(),
            });
            unmatched_primary.remove(&child1);
            unmatched_secondary.remove(&child2);
            discovered = true;
        }

        // Propagation up (parents)
        let p1: Vec<_> = primary.get_parents(prim_v).into_iter()
            .filter(|v| unmatched_primary.contains(v))
            .collect();
        let p2: Vec<_> = secondary.get_parents(sec_v).into_iter()
            .filter(|v| unmatched_secondary.contains(v))
            .collect();

        if p1.len() == 1 && p2.len() == 1 {
            let parent1 = p1[0];
            let parent2 = p2[0];
            fixed_point.basic_block_fixed_points.push(BasicBlockFixedPoint {
                primary_vertex: parent1,
                secondary_vertex: parent2,
                matching_step: "basicBlock: propagation (size==1)".to_string(),
                instruction_matches: Vec::new(),
            });
            unmatched_primary.remove(&parent1);
            unmatched_secondary.remove(&parent2);
            discovered = true;
        }
    }

    discovered
}
