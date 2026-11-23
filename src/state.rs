//! Machine state: `$XDG_STATE_HOME/grove` (default `~/.local/state/grove`). It is safe to delete.
//!
//! G0 keeps only the last `grove sync` outcome per repository (`sync.json`, keyed by git common dir), so
//! `grove status` can show a failed fetch and its age. Losing the file loses nothing that git does not also know.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::output;

const FILE: &str = "sync.json";

/// One repository's last sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastSync {
    /// `ok` when every remote fetched, else `failed`.
    pub result: String,
    pub at: String,
    pub remotes: Vec<RemoteOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteOutcome {
    pub remote: String,
    /// `updated`, `unchanged` or `failed`.
    pub result: String,
    pub error: Option<String>,
}

impl LastSync {
    pub fn new(at: SystemTime, remotes: Vec<RemoteOutcome>) -> Self {
        let failed = remotes.iter().any(|r| r.error.is_some());
        LastSync {
            result: if failed { "failed" } else { "ok" }.into(),
            at: output::timestamp(at),
            remotes,
        }
    }

    pub fn time(&self) -> Option<SystemTime> {
        output::parse_timestamp(&self.at)
    }

    /// `ok`, or `failed (origin)`.
    pub fn display(&self) -> String {
        let failed: Vec<&str> = self
            .remotes
            .iter()
            .filter(|r| r.error.is_some())
            .map(|r| r.remote.as_str())
            .collect();
        if failed.is_empty() {
            self.result.clone()
        } else {
            format!("{} ({})", self.result, failed.join(", "))
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub repos: BTreeMap<String, LastSync>,
}

impl State {
    /// The saved state, or an empty one when there is none or it cannot be read (it is only a cache).
    pub fn load(dir: &Path) -> State {
        std::fs::read(dir.join(FILE))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn last_sync(&self, common_dir: &Path) -> Option<&LastSync> {
        self.repos.get(&common_dir.display().to_string())
    }

    /// Record syncs and save, replacing the file atomically.
    pub fn record(dir: &Path, syncs: Vec<(&Path, LastSync)>) -> Result<()> {
        let mut state = State::load(dir);
        for (common_dir, sync) in syncs {
            state.repos.insert(common_dir.display().to_string(), sync);
        }
        let fail = |e: std::io::Error| {
            Error::new(format!("cannot save grove state in {}: {e}", dir.display()))
                .try_(format!("ls -ld {}", dir.display()))
        };
        std::fs::create_dir_all(dir).map_err(fail)?;
        let tmp = dir.join(format!("{FILE}.{}", std::process::id()));
        let json = serde_json::to_vec_pretty(&state).expect("state serializes");
        std::fs::write(&tmp, json).map_err(fail)?;
        std::fs::rename(&tmp, dir.join(FILE)).map_err(fail)
    }
}
