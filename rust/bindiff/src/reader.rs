use crate::graph::{CallGraph, FlowGraph, VERTEX_STUB, VERTEX_LIBRARY};
use crate::binexport::BinExport2;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read as IoRead;

pub fn read(
    filename: &std::path::Path,
    call_graph: &mut CallGraph,
    flow_graphs: &mut Vec<FlowGraph>,
) -> Result<()> {
    call_graph.reset();
    flow_graphs.clear();

    let mut file = File::open(filename)
        .with_context(|| format!("Failed to open file: {}", filename.display()))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .with_context(|| format!("Failed to read file: {}", filename.display()))?;

    use prost::Message;
    let proto = BinExport2::decode(&*buffer)
        .with_context(|| format!("Failed to parse BinExport2 proto from {}", filename.display()))?;

    setup_graphs_from_proto(
        &proto,
        filename.to_str().unwrap_or(""),
        call_graph,
        flow_graphs,
    )?;

    Ok(())
}

fn precalculate_instruction_addresses(proto: &BinExport2) -> Vec<u64> {
    let mut instruction_addresses = vec![0; proto.instruction.len()];
    let mut current_address = 0;
    for (i, instr) in proto.instruction.iter().enumerate() {
        if let Some(addr) = instr.address {
            current_address = addr;
        }
        instruction_addresses[i] = current_address;
        current_address += instr.raw_bytes.as_deref().unwrap_or(&[]).len() as u64;
    }
    instruction_addresses
}

fn setup_graphs_from_proto(
    proto: &BinExport2,
    filename: &str,
    call_graph: &mut CallGraph,
    flow_graphs: &mut Vec<FlowGraph>,
) -> Result<()> {
    call_graph.read(proto, filename).context("Failed to read call graph")?;

    let instruction_addresses = precalculate_instruction_addresses(proto);

    for proto_flow_graph in &proto.flow_graph {
        if proto_flow_graph.basic_block_index.is_empty() {
            continue;
        }
        let mut flow_graph = FlowGraph::default();
        flow_graph.read(proto, proto_flow_graph, call_graph, &instruction_addresses)
            .with_context(|| format!("Failed to read flow graph at index {:?}", proto_flow_graph.entry_basic_block_index))?;
        flow_graphs.push(flow_graph);
    }

    add_subs_to_call_graph(call_graph, flow_graphs)?;

    flow_graphs.sort_by_key(|fg| fg.entry_point_address);

    Ok(())
}

fn add_subs_to_call_graph(call_graph: &mut CallGraph, flow_graphs: &mut Vec<FlowGraph>) -> Result<()> {
    let node_indices: Vec<_> = call_graph.graph.node_indices().collect();
    for node_idx in node_indices {
        let address = call_graph.graph[node_idx].address;
        if flow_graphs.iter().any(|fg| fg.entry_point_address == address) {
            continue;
        }

        let mut dummy_fg = FlowGraph::default();
        dummy_fg.entry_point_address = address;
        dummy_fg.call_graph_vertex = Some(node_idx);

        call_graph.graph[node_idx].flags |= VERTEX_STUB | VERTEX_LIBRARY;

        flow_graphs.push(dummy_fg);
    }
    Ok(())
}
