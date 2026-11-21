//! `pot runs --json`, contract `runs/1` (see `tests/contracts/`).
//!
//! Runs are matched to checkouts by workspace path, so a run in a worktree belongs to that worktree. grove shows
//! the runs that are still live or want a person: `running`, `review`, `stopped`. Other states (`exited`, and
//! any state grove does not know) are not counted.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A run, as much of it as grove reads.
#[derive(Debug, Clone)]
pub struct Run {
    pub state: String,
    pub workspaces: Vec<PathBuf>,
}

/// What grove shows of the runs in one checkout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Runs {
    pub running: usize,
    pub review: usize,
    pub stopped: usize,
}

impl Runs {
    /// `-`, `1 running`, `1 running, 2 review`.
    pub fn display(&self) -> String {
        let parts: Vec<String> = [("running", self.running), ("review", self.review), ("stopped", self.stopped)]
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .map(|(state, n)| format!("{n} {state}"))
            .collect();
        if parts.is_empty() { "-".into() } else { parts.join(", ") }
    }
}

/// `pot runs --json` prints a list, except inside a run, where it prints that run's record alone.
#[derive(Deserialize)]
struct Document {
    version: u64,
    #[serde(default)]
    runs: Option<Vec<Record>>,
    #[serde(flatten)]
    record: Record,
}

#[derive(Deserialize)]
struct Record {
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    workspace: Vec<Workspace>,
}

#[derive(Deserialize)]
struct Workspace {
    #[serde(default)]
    path: Option<String>,
}

pub fn parse(json: &[u8]) -> Result<Vec<Run>, String> {
    let doc: Document = serde_json::from_slice(json).map_err(|e| format!("not runs/1 JSON: {e}"))?;
    if doc.version != 1 {
        return Err(format!("runs/{} is newer than grove reads (runs/1)", doc.version));
    }
    let records = match doc.runs {
        Some(runs) => runs,
        None if doc.record.state.is_some() => vec![doc.record],
        None => Vec::new(),
    };
    Ok(records
        .into_iter()
        .map(|r| Run {
            state: r.state.unwrap_or_default(),
            workspaces: r.workspace.into_iter().filter_map(|w| w.path).map(PathBuf::from).collect(),
        })
        .collect())
}

/// Count the runs whose workspace is `checkout`.
pub fn runs_in(runs: &[Run], checkout: &Path) -> Runs {
    let same = |p: &PathBuf| p == checkout || p.canonicalize().is_ok_and(|c| c == checkout);
    let mut counts = Runs::default();
    for run in runs.iter().filter(|r| r.workspaces.iter().any(same)) {
        match run.state.as_str() {
            "running" => counts.running += 1,
            "review" => counts.review += 1,
            "stopped" => counts.stopped += 1,
            _ => {}
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!("{}/tests/contracts/runs-1/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn golden_list() {
        let runs = parse(&fixture("list.json")).unwrap();
        assert_eq!(runs.len(), 4);
        let at = |p: &str| runs_in(&runs, Path::new(p));
        assert_eq!(at("/home/daniel/src/flexion-assistant"), Runs { review: 1, ..Runs::default() });
        assert_eq!(at("/home/daniel/src/graft"), Runs { stopped: 1, ..Runs::default() });
        assert_eq!(at("/home/daniel/src/team-kb"), Runs::default()); // exited
        assert_eq!(at("/home/daniel/src/graft").display(), "1 stopped");
        assert_eq!(at("/elsewhere").display(), "-");
    }

    #[test]
    fn a_single_record_is_one_run() {
        for name in ["9b0e", "41c7", "7d21", "c0f3"] {
            assert_eq!(parse(&fixture(&format!("{name}.record.json"))).unwrap().len(), 1, "{name}");
        }
        // Inside a run, `pot runs --json` prints that run alone.
        let inside = parse(&fixture("inside-run.record.json")).unwrap();
        assert_eq!(runs_in(&inside, Path::new("/home/daniel/src/graft")).display(), "1 running");
    }

    #[test]
    fn lenient_but_versioned() {
        let newer = br#"{"version": 1, "runs": [{"state": "pending", "workspace": [{"path": "/x", "new": 1}]}]}"#;
        assert_eq!(runs_in(&parse(newer).unwrap(), Path::new("/x")), Runs::default());
        assert_eq!(parse(br#"{"version": 2, "runs": []}"#).unwrap_err(), "runs/2 is newer than grove reads (runs/1)");
    }
}
