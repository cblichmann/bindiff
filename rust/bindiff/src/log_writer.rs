use crate::graph::{CallGraph, FlowGraph};
use crate::fixed_points::FixedPoints;
use crate::statistics::{get_counts_and_histogram, Counts, Histogram};
use anyhow::{Result, Context};
use std::fs::File;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

pub struct ResultsLogWriter {
    path: PathBuf,
}

impl ResultsLogWriter {
    pub fn create<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn write(
        &self,
        call_graph1: &CallGraph,
        call_graph2: &CallGraph,
        flow_graphs1: &[FlowGraph],
        flow_graphs2: &[FlowGraph],
        fixed_points: &FixedPoints,
    ) -> Result<()> {
        let mut file = File::create(&self.path)
            .with_context(|| format!("Failed to create log file: {}", self.path.display()))?;

        let mut histogram = Histogram::new();
        let mut counts = Counts::default();
        get_counts_and_histogram(
            call_graph1,
            call_graph2,
            flow_graphs1,
            flow_graphs2,
            fixed_points,
            &mut histogram,
            &mut counts,
        );

        let get_stem = |path_str: &str| {
            std::path::Path::new(path_str)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string()
        };

        writeln!(file, "{}", get_stem(&call_graph1.filename))?;
        writeln!(file, "{}", get_stem(&call_graph2.filename))?;
        writeln!(file, "call graph1 MD index {:.16}", call_graph1.md_index)?;
        writeln!(file, "call graph2 MD index {:.16}", call_graph2.md_index)?;
        writeln!(file, "\n --------- statistics ---------")?;

        for (name, value) in counts.get_display_entries() {
            let padding = ".".repeat(60 - name.len());
            writeln!(file, "{}{}:{:>7}", name, padding, value)?;
        }
        writeln!(file)?;

        // Sort histogram by key to have deterministic output order
        let mut sorted_hist: Vec<_> = histogram.iter().collect();
        sorted_hist.sort_by_key(|&(k, _)| k);
        for (step, count) in sorted_hist {
            let padding = ".".repeat(60 - step.len());
            writeln!(file, "{}{}:{:>7}", step, padding, count)?;
        }
        writeln!(file)?;

        let similarity = if !flow_graphs1.is_empty() {
            fixed_points.len() as f64 / flow_graphs1.len() as f64
        } else {
            0.0
        };
        let confidence = similarity;

        writeln!(file, "similarity: {:.16}", similarity)?;
        writeln!(file, "confidence: {:.16}\n", confidence)?;

        writeln!(
            file,
            " --------- matched {} of {}/{} ({}/{}) ------------ ",
            fixed_points.len(),
            counts.functions_primary_non_library,
            counts.functions_secondary_non_library,
            counts.functions_primary_library,
            counts.functions_secondary_library,
        )?;

        for fp in fixed_points {
            let mut name1 = format!("sub_{:X}", fp.primary_address);
            let mut name2 = format!("sub_{:X}", fp.secondary_address);
            let mut md1 = 0.0;
            let mut md2 = 0.0;
            let mut lib1 = 0;
            let mut lib2 = 0;

            if let Some(node_idx) = call_graph1.get_vertex(fp.primary_address) {
                let name = &call_graph1.graph[node_idx].name;
                if !name.is_empty() {
                    name1 = name.clone();
                }
                let fg = flow_graphs1.iter().find(|fg| fg.entry_point_address == fp.primary_address).unwrap();
                lib1 = if crate::statistics::is_library(fg, call_graph1) { 1 } else { 0 };
                md1 = fg.md_index;
            }
            if let Some(node_idx) = call_graph2.get_vertex(fp.secondary_address) {
                let name = &call_graph2.graph[node_idx].name;
                if !name.is_empty() {
                    name2 = name.clone();
                }
                let fg = flow_graphs2.iter().find(|fg| fg.entry_point_address == fp.secondary_address).unwrap();
                lib2 = if crate::statistics::is_library(fg, call_graph2) { 1 } else { 0 };
                md2 = fg.md_index;
            }

            let bbs1 = flow_graphs1.iter().find(|fg| fg.entry_point_address == fp.primary_address).unwrap().graph.node_count();
            let bbs2 = flow_graphs2.iter().find(|fg| fg.entry_point_address == fp.secondary_address).unwrap().graph.node_count();

            writeln!(
                file,
                "{:X}\t{:X}\t{:.16}\t{:.16}\t{:.16}\t{:.16}\t{}\t{}\t{}\t\"{}\"\t\"{}\"",
                fp.primary_address,
                fp.secondary_address,
                fp.similarity,
                fp.confidence,
                md1,
                md2,
                lib1,
                lib2,
                fp.matching_step,
                name1,
                name2
            )?;
            writeln!(
                file,
                "\t{}\t{}\t{}",
                fp.basic_block_fixed_points.len(),
                bbs1,
                bbs2
            )?;
        }

        let matched_primary: HashSet<_> = fixed_points.iter().map(|fp| fp.primary_address).collect();
        let unmatched_primary: Vec<_> = flow_graphs1.iter()
            .filter(|fg| !matched_primary.contains(&fg.entry_point_address))
            .collect();

        writeln!(file, " --------- unmatched primary ({}) ------------ ", unmatched_primary.len())?;
        for fg in unmatched_primary {
            let name = if let Some(node_idx) = fg.call_graph_vertex {
                &call_graph1.graph[node_idx].name
            } else {
                ""
            };
            writeln!(
                file,
                "{:X}\t{}\t{:.16}\t{}",
                fg.entry_point_address,
                if crate::statistics::is_library(fg, call_graph1) { 1 } else { 0 },
                fg.md_index,
                name
            )?;
        }

        let matched_secondary: HashSet<_> = fixed_points.iter().map(|fp| fp.secondary_address).collect();
        let unmatched_secondary: Vec<_> = flow_graphs2.iter()
            .filter(|fg| !matched_secondary.contains(&fg.entry_point_address))
            .collect();

        writeln!(file, " --------- unmatched secondary ({}) ------------ ", unmatched_secondary.len())?;
        for fg in unmatched_secondary {
            let name = if let Some(node_idx) = fg.call_graph_vertex {
                &call_graph2.graph[node_idx].name
            } else {
                ""
            };
            writeln!(
                file,
                "{:X}\t{}\t{:.16}\t{}",
                fg.entry_point_address,
                if crate::statistics::is_library(fg, call_graph2) { 1 } else { 0 },
                fg.md_index,
                name
            )?;
        }

        Ok(())
    }
}
