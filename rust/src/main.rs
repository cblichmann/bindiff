use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use clap::ValueEnum;

mod config;

const BINDIFF_COPYRIGHT: &str = "(c)2004-2011 zynamics GmbH, (c)2011-2024 Google LLC.";

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

fn main() -> ExitCode {
    let mut cli = Cli::parse();

    if cli.output_dir.is_none() {
        cli.output_dir = Some(std::env::current_dir().unwrap_or(".".into()));
    }
    println!("{}", cli.output_dir.unwrap().display());
    println!("{}", config::CONFIG_NAME);

    if cli.logo && !cli.nologo {
        println!(
            "BinDiff {}, {}",
            bindiff_detailed_version(),
            BINDIFF_COPYRIGHT
        );
    }

    let mut config = config::proto();
    if let Some(config_file) = cli.config {
        let loaded = config::load_from_file(config_file);
        // config::merge_into(loaded, config);
    }

    // Print configuration to stdout if requested
    if cli.print_config {
        eprintln!("not implemented!");
        return ExitCode::FAILURE;
    }

    // Launch Java UI if requested
    if cli.ui {
        eprintln!("not implemented!");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
