use crate::graph::FlowGraph;
use crate::fixed_points::{FixedPoint, BasicBlockFixedPoint};
use petgraph::graph::NodeIndex;
use std::collections::{HashSet, HashMap};

pub fn find_fixed_points_basic_block(
    fixed_point: &mut FixedPoint,
    primary: &FlowGraph,
    secondary: &FlowGraph,
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

    match_basic_blocks_by_md_index_relaxed(
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
