use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use graph::CallGraph;

pub mod types;
pub mod instruction;
pub mod graph;
pub mod prime_signature;
pub mod reader;
pub mod fixed_points;
pub mod differ;
pub mod database_writer;
pub mod statistics;
pub mod log_writer;
pub mod basic_block_differ;

// Include generated proto code
pub mod bindiff {
    include!(concat!(env!("OUT_DIR"), "/security.bindiff.rs"));
    include!(concat!(env!("OUT_DIR"), "/security.bindiff.serde.rs"));
}

pub mod binexport {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

use bindiff::Config;

#[derive(Parser, Debug)]
#[command(author, version, about = "Find similarities and differences in disassembled code.", long_about = None)]
struct Args {
    #[arg(long, default_value_t = true, help = "display version/copyright information")]
    logo: bool,

    #[arg(long, help = "launch the BinDiff UI")]
    ui: bool,

    #[arg(long, help = "primary input file or path in batch mode")]
    primary: Option<String>,

    #[arg(long, help = "secondary input file (optional)")]
    secondary: Option<String>,

    #[arg(long, help = "output path, defaults to current directory")]
    output_dir: Option<PathBuf>,

    #[arg(
        long,
        value_delimiter = ',',
        default_value = "bin",
        help = "comma-separated list of output formats: log, bin"
    )]
    output_format: Vec<String>,

    #[arg(long, help = "dump MD indices (will not diff anything)")]
    md_index: bool,

    #[arg(long, help = "batch export .idb files from input directory to BinExport format")]
    export: bool,

    #[arg(long, help = "list hash/filenames for all .BinExport files in input directory")]
    ls: bool,

    #[arg(long, help = "specify config file name")]
    config: Option<PathBuf>,

    #[arg(long, help = "print parsed configuration to stdout and exit")]
    print_config: bool,

    // Positional arguments
    #[arg(help = "primary input file/directory (positional)")]
    pos_primary: Option<String>,

    #[arg(help = "secondary input file (positional)")]
    pos_secondary: Option<String>,
}

const DEFAULT_CONFIG_JSON: &str = include_str!("../../../bindiff.json");

fn load_default_config() -> Result<Config> {
    serde_json::from_str(DEFAULT_CONFIG_JSON).context("Failed to parse default config JSON")
}

fn load_config_file(path: &std::path::Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config JSON from {}", path.display()))
}

fn main() -> Result<()> {
    let mut args = Args::parse();

    // Handle positional arguments like in C++
    if args.primary.is_none() {
        args.primary = args.pos_primary.clone();
    } else if args.pos_primary.is_some() {
        eprintln!("Error: Primary input specified both as flag and positional argument.");
        std::process::exit(1);
    }

    if args.secondary.is_none() {
        args.secondary = args.pos_secondary.clone();
    } else if args.pos_secondary.is_some() {
        eprintln!("Error: Secondary input specified both as flag and positional argument.");
        std::process::exit(1);
    }

    if args.logo && !args.print_config {
        println!("BinDiff Rust (Port of BinDiff 8)");
        println!("Copyright 2011-2024 Google LLC, 2026 Jetski Port");
    }

    // Load configuration
    let mut config = load_default_config().context("Failed to load default configuration")?;

    if let Some(config_path) = &args.config {
        let loaded_config = load_config_file(config_path)?;
        // In C++ we merge, but for now we just replace or we can implement a simple merge if needed.
        // Protobuf has MergeFrom, but since we are using serde structs, we might need custom merge
        // or just use the loaded one.
        // For now, let's just use the loaded one to keep it simple, or merge manually if important.
        config = loaded_config;
    }

    if args.print_config {
        let config_json = serde_json::to_string_pretty(&config)
            .context("Failed to serialize configuration to JSON")?;
        println!("{}", config_json);
        return Ok(());
    }

    if let (Some(primary), Some(secondary)) = (&args.primary, &args.secondary) {
        let mut primary_call_graph = CallGraph::default();
        let mut primary_flow_graphs = Vec::new();
        println!("Reading primary: {}", primary);
        reader::read(std::path::Path::new(primary), &mut primary_call_graph, &mut primary_flow_graphs)
            .context("Failed to read primary")?;

        let mut secondary_call_graph = CallGraph::default();
        let mut secondary_flow_graphs = Vec::new();
        println!("Reading secondary: {}", secondary);
        reader::read(std::path::Path::new(secondary), &mut secondary_call_graph, &mut secondary_flow_graphs)
            .context("Failed to read secondary")?;

        println!("Diffing...");
        let mut context = differ::MatchingContext::new(
            &primary_call_graph,
            &secondary_call_graph,
            &primary_flow_graphs,
            &secondary_flow_graphs,
        );
        differ::diff(&mut context);

        println!("Diff completed!");
        println!("Matched functions: {}", context.fixed_points.len());
        
        let get_stem = |path_str: &str| {
            std::path::Path::new(path_str)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string()
        };
        let prim_name = get_stem(&primary_call_graph.filename);
        let sec_name = get_stem(&secondary_call_graph.filename);

        if args.output_format.iter().any(|f| f == "bin" || f == "binary") {
            let out_dir = args.output_dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            let out_filename = format!("{}_vs_{}.BinDiff", prim_name, sec_name);
            let out_path = out_dir.join(out_filename);
            println!("Writing SQLite database to: {}", out_path.display());
            
            let mut db_writer = database_writer::DatabaseWriter::create(&out_path)
                .context("Failed to create DatabaseWriter")?;
            db_writer.write(
                &primary_call_graph,
                &secondary_call_graph,
                &primary_flow_graphs,
                &secondary_flow_graphs,
                &context.fixed_points,
            ).context("Failed to write database matches")?;
            println!("SQLite database written successfully!");
        }

        if args.output_format.iter().any(|f| f == "log") {
            let out_dir = args.output_dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            let out_filename = format!("{}_vs_{}.results", prim_name, sec_name);
            let out_path = out_dir.join(out_filename);
            println!("Writing text log to: {}", out_path.display());
            
            let log_writer = log_writer::ResultsLogWriter::create(&out_path);
            log_writer.write(
                &primary_call_graph,
                &secondary_call_graph,
                &primary_flow_graphs,
                &secondary_flow_graphs,
                &context.fixed_points,
            ).context("Failed to write text log")?;
            println!("Text log written successfully!");
        }
    } else {
        println!("Arguments parsed successfully: {:?}", args);
        println!("Config loaded successfully (num_threads: {})", config.num_threads);
    }

    Ok(())
}
