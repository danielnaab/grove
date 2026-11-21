//! grove: keep your git checkouts in view. Read only.
//!
//! The layout follows the data flow of every command:
//!
//! ```text
//! cli ──▶ config ──▶ discover ──▶ address ──▶ git + sources ──▶ commands ──▶ output
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
mod discover;
mod error;
mod exit;
mod fact;
mod forge;
mod git;
mod output;
mod sources;
mod state;

fn main() -> std::process::ExitCode {
    // Not built yet: each command lands test-first, against its transcript.
    eprintln!("grove: not built yet\n  try: read docs/reference/cli/");
    std::process::ExitCode::from(1)
}
