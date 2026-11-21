//! External sources behind one boundary: the contracts grove reads as data.
//!
//! Each tool is run and its JSON turned into facts keyed by checkout path. Nothing outside this module knows the
//! contracts' shapes. A tool's state:
//!
//! - `absent`: the tool is not on `PATH`. Its facts are `unknown` and this alone is not partial (exit 0).
//! - `failed`: the tool ran but failed, or printed JSON that does not match the contract. Its facts are
//!   `unknown` and the exit code is 3.
//! - `ok`.
//!
//! Contracts: `status/1` from `graft status --json` (pins) and `runs/1` from `pot runs --json` (runs). Their
//! schemas and golden fixtures are copied into `tests/contracts/`.

mod graft;
mod pot;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

pub use graft::Pins;
pub use pot::Runs;

use crate::context::Context;
use crate::fact::Fact;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", content = "detail", rename_all = "lowercase")]
pub enum State {
    Ok,
    Absent(String),
    Failed(String),
}

/// The sources' states, as every JSON document that shows facts reports them.
#[derive(Debug, Serialize)]
pub struct Report {
    pub git: State,
    pub graft: State,
    pub pot: State,
}

impl Report {
    /// `(column, state)` for each source whose facts are unknown, for the `? pins: ...` lines.
    pub fn unknown(&self) -> Vec<(&'static str, &State)> {
        [("pins", &self.graft), ("runs", &self.pot)]
            .into_iter()
            .filter(|(_, s)| **s != State::Ok)
            .collect()
    }

    pub fn partial(&self) -> bool {
        [&self.git, &self.graft, &self.pot]
            .into_iter()
            .any(|s| matches!(s, State::Failed(_)))
    }
}

pub struct Sources {
    graft: Option<PathBuf>,
    pot: std::result::Result<Vec<pot::Run>, State>,
}

impl Sources {
    /// Find graft and pot, and read pot's runs once.
    pub fn load() -> Sources {
        let pot = match on_path("pot") {
            None => Err(State::Absent("pot not found on PATH".into())),
            Some(pot) => run(&pot, &["runs", "--json"], "pot runs")
                .and_then(|out| pot::parse(&out).map_err(|e| format!("pot runs --json: {e}")))
                .map_err(State::Failed),
        };
        Sources {
            graft: on_path("graft"),
            pot,
        }
    }

    /// The `pins` fact for a checkout. graft runs only where `.gitmodules` exists: elsewhere there are none.
    pub fn pins(&self, ctx: &Context, checkout: &Path) -> Fact<Pins> {
        let Some(graft) = &self.graft else {
            return Fact::unknown("graft not found on PATH", "graft", ctx.now);
        };
        if !checkout.join(".gitmodules").exists() {
            return Fact::known(Pins::default(), ".gitmodules", ctx.now);
        }
        let dir = checkout.to_string_lossy();
        let result = run(
            graft,
            &["status", "-C", &dir, "--json"],
            &format!("graft status in {}", ctx.tilde(checkout)),
        )
        .and_then(|out| {
            graft::parse(&out).map_err(|e| format!("graft status --json in {}: {e}", ctx.tilde(checkout)))
        });
        match result {
            Ok(pins) => Fact::known(pins, "graft", ctx.now),
            Err(reason) => Fact::unknown(reason, "graft", ctx.now),
        }
    }

    /// The `runs` fact for a checkout: pot's runs whose workspace is this checkout.
    pub fn runs(&self, ctx: &Context, checkout: &Path) -> Fact<Runs> {
        match &self.pot {
            Ok(runs) => Fact::known(pot::runs_in(runs, checkout), "pot", ctx.now),
            Err(State::Absent(reason) | State::Failed(reason)) => {
                Fact::unknown(reason.clone(), "pot", ctx.now)
            }
            Err(State::Ok) => unreachable!("an error state is never Ok"),
        }
    }

    /// The report, given the pins facts this command read (graft's state is the worst of them).
    pub fn report<'a>(&self, pins: impl IntoIterator<Item = &'a Fact<Pins>>) -> Report {
        let graft = if self.graft.is_none() {
            State::Absent("graft not found on PATH".into())
        } else {
            pins.into_iter()
                .find_map(|p| p.value.as_ref().err())
                .map_or(State::Ok, |reason| State::Failed(reason.clone()))
        };
        let pot = match &self.pot {
            Ok(_) => State::Ok,
            Err(state) => state.clone(),
        };
        Report {
            git: State::Ok,
            graft,
            pot,
        }
    }
}

/// Run a tool and return its stdout, or why it failed.
fn run(program: &Path, args: &[&str], what: &str) -> std::result::Result<Vec<u8>, String> {
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{what} could not start: {e}"))?;
    if out.status.success() {
        return Ok(out.stdout);
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let first = stderr
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("no message");
    Err(format!("{what} failed ({}): {first}", out.status))
}

/// An executable named `name` on `PATH`.
pub fn on_path(name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|p| {
            p.metadata()
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
}
