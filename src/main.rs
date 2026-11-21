//! grove: keep your git checkouts in view. Read only.
//!
//! The layout follows the data flow of every command:
//!
//! ```text
//! cli ──▶ config ──▶ discover ──▶ address ──▶ observe (git + sources + state) ──▶ commands ──▶ output
//!          │                                      │
//!          └── grove.toml at HEAD                 └── facts: value | unknown, source, observed_at
//! ```
//!
//! Behavior is specified by the transcripts in `docs/reference/cli/`; read those before this code.
#![forbid(unsafe_code)]

mod address;
mod cli;
mod commands;
mod config;
mod context;
mod discover;
mod error;
mod exit;
mod fact;
mod forge;
mod git;
mod observe;
mod output;
mod parallel;
mod sources;
mod state;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command, ConfigCommand};
use context::Context;
use error::Error;
use exit::Exit;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => return usage_error(&err),
    };
    let name = cli.command.name();
    let result = Context::from_env(cli.json).and_then(|ctx| run(&ctx, cli.command));
    match result {
        Ok(exit) => exit.into(),
        Err(error) => {
            output::print_error(name, cli.json, &error);
            Exit::Error.into()
        }
    }
}

fn run(ctx: &Context, command: Command) -> error::Result<Exit> {
    match command {
        Command::Ls { address, path } => commands::ls::run(ctx, address.as_deref(), path),
        Command::Status { address } => commands::status::run(ctx, &address),
        Command::Sync { address } => commands::sync::run(ctx, address.as_deref()),
        Command::Open { address, print } => commands::open::run(ctx, &address, print),
        Command::Config { command: ConfigCommand::Check { file } } => {
            commands::config_check::run(ctx, file.as_deref())
        }
    }
}

/// `--help` and `--version` exit 0. A usage error exits 1, not clap's 2, and is JSON under `--json`.
fn usage_error(err: &clap::Error) -> ExitCode {
    if !err.use_stderr() {
        let _ = err.print();
        return Exit::Ok.into();
    }
    if std::env::args().any(|a| a == "--json") {
        let text = err.to_string();
        let first = text.lines().next().unwrap_or("usage error");
        let message = first.strip_prefix("error: ").unwrap_or(first);
        output::print_error("usage", true, &Error::new(message).try_("grove --help"));
    } else {
        let _ = err.print();
    }
    Exit::Error.into()
}
