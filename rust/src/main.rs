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

use std::env;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use bindiff::graphs::binexport2::BinExport2;
use clap::Parser;
use clap::ValueEnum;

use bindiff::config;
use bindiff::version::BINDIFF_COPYRIGHT;
use bindiff::graphs::Binary;
use protobuf::Message;

fn bindiff_release() -> u32 {
    // We only use the major version
    clap::crate_version!()
        .splitn(2, '.')
        .next()
        .unwrap()
        .parse::<u32>()
        .unwrap()
}

fn bindiff_detailed_version() -> String {
    // Populated by build.rs
    format!(
        "{} (@{} {})",
        bindiff_release(),
        env!("GIT_REV"),
        env!("BUILD_DATE")
    )
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    /// Text file
    Log,
    /// BinDiff database loadable by the UI and disassembler plugins
    #[value(alias("bin"))]
    Binary,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Config file name
    #[arg(long)]
    config: Option<PathBuf>,

    /// Display version/copyright information
    #[arg(
        long,
        default_missing_value("true"),
        default_value("true"),
        num_args(0..=1),
        require_equals(true),
        action = clap::ArgAction::Set,
        value_name(""),
        hide_possible_values(true),
    )]
    logo: bool,

    #[arg(long, conflicts_with("logo"), hide(true))]
    nologo: bool,

    /// Print MD indices (will not diff anything)
    #[arg(long, visible_alias("md_index"))]
    md_index: bool,

    /// Output path, defaults to current directory
    #[arg(long, visible_alias("output_dir"))]
    output_dir: Option<PathBuf>,

    /// Comma-separated list of output formats
    #[arg(long, visible_alias("output_format"), default_value("binary"))]
    output_format: Vec<OutputFormat>,

    /// Primary input file
    #[arg(long)]
    primary: Option<PathBuf>,

    #[arg(conflicts_with("primary"), name("PRIMARY"))]
    primary_pos: Option<PathBuf>,

    /// Secondary input file
    #[arg(long)]
    secondary: Option<PathBuf>,

    #[arg(conflicts_with("secondary"), name("SECONDARY"))]
    secondary_pos: Option<PathBuf>,

    /// Print parsed configuration to stdout and exit
    #[arg(long, visible_alias("print_config"))]
    print_config: bool,

    /// Launch the BinDiff UI
    #[arg(long)]
    ui: bool,
    // Not implemented:
    //   Batch mode
    //
    // Unsupported flags from the C++ version:
    // --export (batch export .idb files from input directory to BinExport format);
    //   default: false;
    // --flagfile (comma-separated list of files to load flags from); default: ;
    // --fromenv (comma-separated list of flags to set from the environment [use
    //   'export FLAGS_flag1=value']); default: ;
    // --ls (list hash/filenames for all .BinExport files in input directory);
    //   default: false;
    // --tryfromenv (comma-separated list of flags to try to set from the
    //   environment if present); default: ;
    // --undefok (comma-separated list of flag names that it is okay to specify on
    //   the command line even if the program does not define a flag with that
    //   name); default: ;
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let output_dir = cli
        .output_dir
        .unwrap_or(std::env::current_dir().unwrap_or(".".into()));

    if cli.logo && !cli.nologo {
        println!(
            "BinDiff {}, {}",
            bindiff_detailed_version(),
            BINDIFF_COPYRIGHT
        );
    }

    let mut config = config::proto();
    if let Some(config_file) = cli.config {
        let loaded_config = config::load_from_file(config_file).unwrap();
        config::merge_into(&loaded_config, &mut config)?;
    }

    // Print configuration to stdout if requested
    if cli.print_config {
        let json_config = config::as_json_string(&config)?;
        println!("{}", json_config);
        return Ok(());
    }

    // Launch Java UI if requested
    if std::env::args().nth(0) == Some("bindiff_ui".to_string()) || cli.ui {
        return Err(anyhow!("not implemented"));
    }

    let primary = cli
        .primary
        .or(cli.primary_pos)
        .ok_or(anyhow!("Need primary input"))?;

    if !output_dir.is_dir() {
        return Err(anyhow!(
            "Output parameter (--output-dir) must be a writable directory: {}",
            output_dir.display()
        ));
    }

    if primary.is_file() {
        let mut reader = std::fs::OpenOptions::new().read(true).open(&primary)?;
        let proto = BinExport2::parse_from_reader(&mut reader)?;
        let binary = Binary::from_proto(&proto, &primary);
        eprintln!("OK");
    }

    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
