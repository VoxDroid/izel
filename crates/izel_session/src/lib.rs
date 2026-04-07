//! Compilation session state and configuration.

use clap::Parser;
use std::path::PathBuf;

/// The primary configuration for an Izel compilation session.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct SessionOptions {
    /// The input file to compile.
    pub input: Option<PathBuf>,

    /// The output path for the compiled binary.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Cross-compilation target triple.
    #[arg(long)]
    pub target: Option<String>,

    /// Emit specific IR or meta-information.
    #[arg(long)]
    pub emit: Option<String>,

    /// Optimize the output binary.
    #[arg(short = 'O', long, default_value = "0")]
    pub opt: String,

    /// Emit debug information.
    #[arg(long)]
    pub debug: bool,

    /// Exclude standard library.
    #[arg(long)]
    pub no_std: bool,

    /// Enforce strict effect annotation checks.
    #[arg(long)]
    pub check_effects: bool,

    /// Run the code after compilation using JIT.
    #[arg(long)]
    pub run: bool,

    /// Enable runtime contract checking (inject @requires/@ensures assertions).
    #[arg(long)]
    pub check_contracts: bool,

    /// Retain witness checks in release-oriented builds.
    #[arg(long)]
    pub keep_witnesses: bool,

    /// Enable link-time optimization.
    #[arg(long)]
    pub lto: bool,

    /// Strip debug symbols from output.
    #[arg(long)]
    pub strip: bool,

    /// CPU model for target-specific code generation.
    #[arg(long)]
    pub target_cpu: Option<String>,

    /// Diagnostic output format.
    #[arg(long)]
    pub error_format: Option<String>,

    /// Language edition.
    #[arg(long, default_value = "2025")]
    pub edition: String,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Command {
    /// Format a source file
    Fmt {
        /// The file to format
        input: PathBuf,
    },
    /// Start the language server
    Lsp,
    /// Resolve project dependencies
    Deps {
        /// Path to Izel.toml
        manifest_path: PathBuf,
    },
}

pub struct Session {
    pub options: SessionOptions,
}

impl Session {
    pub fn new(options: SessionOptions) -> Self {
        Self { options }
    }
}
