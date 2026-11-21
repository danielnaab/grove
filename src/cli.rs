//! The command line, declared with clap (derive).
//!
//! Parse with `try_parse`. `--help` and `--version` exit 0, and every usage error exits 1, not clap's 2
//! (see `exit`).

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "grove", version, about)]
pub struct Cli {
    /// Print JSON (schema grove/1)
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List repositories: branch, tree, ahead/behind, pins, runs
    Ls {
        /// A name, owner/name, path, `.` or @group (default: every repository)
        address: Option<String>,
        /// Print each repository's main checkout path, one per line, and nothing else
        #[arg(long)]
        path: bool,
    },
    /// Everything grove knows about one repository, with sources and ages
    Status {
        /// A name, owner/name, path or `.`
        #[arg(default_value = ".")]
        address: String,
    },
    /// Fetch every remote, in parallel. Never merges
    Sync {
        /// A name, owner/name, path, `.` or @group (default: every repository)
        address: Option<String>,
    },
    /// Open a repository's forge page
    Open {
        /// A name, owner/name, path or `.`
        address: String,
        /// Print the URL instead of opening it
        #[arg(long)]
        print: bool,
    },
    /// Check grove.toml
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Validate grove.toml and show which line set each value
    Check {
        /// Check this file as written (a draft) instead of grove.toml at HEAD
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

impl Command {
    /// The command's name in JSON output (`"command"`).
    pub fn name(&self) -> &'static str {
        match self {
            Command::Ls { .. } => "ls",
            Command::Status { .. } => "status",
            Command::Sync { .. } => "sync",
            Command::Open { .. } => "open",
            Command::Config { .. } => "config check",
        }
    }
}
