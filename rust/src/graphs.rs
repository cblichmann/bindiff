// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ops::RangeBounds;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;

include!(concat!(env!("OUT_DIR"), "/bindiff/mod.rs"));
use binexport2::bin_export2;
use binexport2::BinExport2;
use petgraph::dot::Dot;
use petgraph::visit::IntoEdgeReferences;

fn format_address(address: u64) -> String {
    if address <= 0xFFFFFFFF {
        format!("{:08X}", address)
    } else {
        format!("{:016X}", address)
    }
}

fn format_function_name(address: u64) -> String {
    format!("sub_{:X}", address)
}

fn lower_bound<T, U>(container: T, element: U) -> usize
where
    T: Deref<Target = [U]>,
    U: Ord,
{
    container
        .binary_search_by(|e| match e.cmp(&element) {
            // Since we try to find position of first element,
            // we treat all equal values as greater to move left.
            Ordering::Equal => Ordering::Greater,
            ord => ord,
        })
        // Since our comparator never returns `Ordering::Equal`,
        // it would always `Err(idx)`.
        .unwrap_err()
}

pub struct OperatorId(u64, u8);

pub struct Comment; // TODO

pub type CallGraphNodeFlags = u32;

/// Library function
pub const CALL_GRAPH_NODE_LIBRARY: CallGraphNodeFlags = 1 << 0;
/// Stub function, i.e. a single JMP, etc.
pub const CALL_GRAPH_NODE_STUB: CallGraphNodeFlags = 1 << 1;
/// Function has a name that is not auto-generated
pub const CALL_GRAPH_NODE_NAME: CallGraphNodeFlags = 1 << 2;
/// C++ demangled name
pub const CALL_GRAPH_NODE_DEMANGLED_NAME: CallGraphNodeFlags = 1 << 3;

#[derive(Default, Clone, Debug)]
pub struct CallGraphNode {
    address: u64,
    name: String,
    demangled_name: String,
    // bfs_top_down: u32,
    // bfs_bottom_up: u32,
    flags: CallGraphNodeFlags,
    //flow_graph: ptr to FlowGraph?
}

#[derive(Default, Clone, Debug)]
pub struct CallGraphEdge {
    flags: u32,
    md_index_proximity: f64,
    md_index_top_down: f64,
    md_index_bottom_up: f64,
}

#[derive(Default)]
pub struct CallGraph {
    graph: petgraph::csr::Csr<
        CallGraphNode,
        CallGraphEdge,
        petgraph::Directed,
        u32, // Index type
    >,
    md_index: f64,
    comments: BTreeMap<OperatorId, Comment>,
}

impl CallGraph {
    pub fn new() -> CallGraph {
        CallGraph::default()
    }

    pub fn from_proto(proto: &bin_export2::CallGraph) -> Result<Self> {
        let mut call_graph = Self::new();

        let num_nodes = proto.vertex.len();
        let mut last_address = 0;
        let mut temp_addresses = vec![0u64; num_nodes];
        for i in 0..num_nodes {
            let proto_node = &proto.vertex[i];
            let mut node = CallGraphNode {
                address: proto_node.address(),
                ..Default::default()
            };

            if node.address < last_address {
                return Err(anyhow!(
                    "Call graph nodes not sorded: {} >= {}",
                    format_address(node.address),
                    format_address(last_address)
                ));
            }
            last_address = node.address;

            temp_addresses[i] = node.address;
            if let Some(mangled_name) = proto_node.mangled_name.as_deref() {
                node.flags |= CALL_GRAPH_NODE_NAME;
                node.name = mangled_name.to_owned();
            }
            if let Some(demangled_name) = proto_node.demangled_name.as_deref() {
                assert!(proto_node.has_mangled_name());
                node.flags |= CALL_GRAPH_NODE_DEMANGLED_NAME;
                node.demangled_name = demangled_name.to_owned();
            }
            if node.flags & CALL_GRAPH_NODE_NAME == 0 {
                // Synthesize a name for the function
                node.name = format_function_name(node.address);
            }
            node.flags |=
                match proto_node.type_.unwrap_or_default().enum_value() {
                    Ok(bin_export2::call_graph::vertex::Type::LIBRARY) => {
                        CALL_GRAPH_NODE_LIBRARY
                    }
                    Ok(bin_export2::call_graph::vertex::Type::THUNK) => {
                        CALL_GRAPH_NODE_STUB
                    }
                    Ok(_) | Err(_) => 0,
                };

            call_graph.graph.add_node(node);
        }

        let num_edges = proto.edge.len();
        for i in 0..num_edges {
            let proto_edge = &proto.edge[i];
            let source_address = proto.vertex
                [proto_edge.source_vertex_index() as usize]
                .address();
            let target_address = proto.vertex
                [proto_edge.target_vertex_index() as usize]
                .address();

            if let Ok(source) = temp_addresses.binary_search(&source_address) {
                if let Ok(target) =
                    temp_addresses.binary_search(&target_address)
                {
                    if temp_addresses[source] == source_address
                        && temp_addresses[target] == target_address
                    {
                        call_graph.graph.add_edge(
                            source as u32,
                            target as u32,
                            CallGraphEdge::default(),
                        );
                    }
                }
            }
        }

        println!("---- 8< -------- 8< ------ SNIP ------ 8< -------- 8< ----");
        println!("{:?}", Dot::with_config(&call_graph.graph, &[petgraph::dot::Config::EdgeNoLabel]));
        println!("---- 8< -------- 8< ------ SNIP ------ 8< -------- 8< ----");

        Ok(call_graph)
    }
}

// Basic block level and inner basic block level
struct Level(u16, u16);

#[derive(Default)]
pub struct FlowGraph {
    entry_point_address: u64,

    //graph: petgraph,
    level_for_call: Vec<(u64, Level)>,
    //call_graph: ptr? to call graph
    //call_graph_vertex: ??
    md_index: f64,
    md_index_inverted: f64,
    //fixed_point: ptr? to fixed point
    prime: u64,
    byte_hash: u32,
    string_references: u32,
    //instructions: Vec<>
    call_targets: Vec<u64>,
    num_loops: u16,

    // Flow graph infos
    address: u64,
    name: Option<String>,
    demangled_name: Option<String>,
    basic_block_count: i32,
    edge_count: i32,
    instruction_count: i32,
}

struct FlowGraphByEntryPointAddress(FlowGraph);

impl Deref for FlowGraphByEntryPointAddress {
    type Target = FlowGraph;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FlowGraphByEntryPointAddress {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Ord for FlowGraphByEntryPointAddress {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.entry_point_address.cmp(&other.0.entry_point_address)
    }
}

impl PartialOrd for FlowGraphByEntryPointAddress {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FlowGraphByEntryPointAddress {
    fn eq(&self, other: &Self) -> bool {
        self.0.entry_point_address == other.0.entry_point_address
    }
}

impl Eq for FlowGraphByEntryPointAddress {}

#[derive(Default)]
pub struct Binary {
    exe_filename: String,
    exe_hash: String, // SHA256 always?
    filename: PathBuf,
    call_graph: CallGraph,
    flow_graphs: BTreeSet<FlowGraphByEntryPointAddress>,
}

impl Binary {
    pub fn from_proto<P: AsRef<Path>>(
        proto: &BinExport2,
        path: P,
    ) -> Result<Self> {
        Ok(Binary {
            exe_filename: proto.meta_information.executable_name().to_string(),
            exe_hash: proto.meta_information.executable_id().to_string(),
            filename: path.as_ref().to_owned(),
            call_graph: CallGraph::from_proto(&proto.call_graph)?,
            flow_graphs: BTreeSet::<FlowGraphByEntryPointAddress>::new(),
        })
    }
}
