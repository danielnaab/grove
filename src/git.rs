//! Running git, read only.
//!
//! Every call goes through here with `--no-optional-locks`, `GIT_TERMINAL_PROMPT=0` and no pager. Reads:
//! `status --porcelain=v2 --branch` (branch, upstream, ahead/behind, tree), `config --get-regexp` (raw remote
//! URLs, before `insteadOf`), `worktree list --porcelain`, and the mtime of `FETCH_HEAD`.
//!
//! The only command that changes anything is `fetch` (for `grove sync`), with
//! `-c gc.auto=0 -c maintenance.auto=false`. It updates remote-tracking refs (and `FETCH_HEAD`) only.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::SystemTime;

use serde::Serialize;

use crate::error::{Error, Result};

fn git(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("--no-optional-locks")
        .arg("-C")
        .arg(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("LC_ALL", "C")
        .stdin(std::process::Stdio::null());
    cmd
}

fn run(mut cmd: Command) -> Result<Output> {
    cmd.output().map_err(|e| {
        Error::new(format!("cannot run git: {e}")).try_("git --version (grove needs git on PATH)")
    })
}

/// Run git in `dir` and return its stdout, or `None` when git fails.
fn read(dir: &Path, args: &[&str]) -> Result<Option<String>> {
    let mut cmd = git(dir);
    cmd.args(args);
    let out = run(cmd)?;
    Ok(out
        .status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned()))
}

/// What `git rev-parse` says about a directory that might be a checkout.
pub struct Probe {
    pub toplevel: PathBuf,
    pub common_dir: PathBuf,
}

/// Is `dir` the top of a checkout? `None` when it is not a checkout, or is inside one without being its top.
pub fn probe(dir: &Path) -> Result<Option<Probe>> {
    let Some(out) = read(
        dir,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--show-toplevel",
            "--git-common-dir",
        ],
    )?
    else {
        return Ok(None);
    };
    let mut lines = out.lines().map(|l| canonical(Path::new(l)));
    let (Some(toplevel), Some(common_dir)) = (lines.next(), lines.next()) else {
        return Ok(None);
    };
    Ok((toplevel == canonical(dir)).then_some(Probe { toplevel, common_dir }))
}

/// The worktree paths that `git worktree list` records, main checkout first. Missing ones are left out.
pub fn worktrees(dir: &Path) -> Result<Vec<PathBuf>> {
    let out = read(dir, &["worktree", "list", "--porcelain"])?.unwrap_or_default();
    Ok(out
        .lines()
        .filter_map(|l| l.strip_prefix("worktree "))
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .map(|p| canonical(&p))
        .collect())
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Remote {
    pub name: String,
    /// As written in git config, before `insteadOf`.
    pub url: String,
}

/// Remotes in config order, with their URLs as written.
pub fn remotes(dir: &Path) -> Result<Vec<Remote>> {
    let out = read(dir, &["config", "--get-regexp", r"^remote\..*\.url$"])?.unwrap_or_default();
    Ok(out
        .lines()
        .filter_map(|line| {
            let (key, url) = line.split_once(' ')?;
            let name = key.strip_prefix("remote.")?.strip_suffix(".url")?;
            Some(Remote {
                name: name.to_string(),
                url: url.to_string(),
            })
        })
        .collect())
}

/// When the repository was last fetched, by anyone: the mtime of `FETCH_HEAD`.
///
/// git empties `FETCH_HEAD` when a fetch starts and fills it when one succeeds, so an empty file records only a
/// failed attempt, and counts as no fetch.
pub fn fetched_at(common_dir: &Path) -> Option<SystemTime> {
    let meta = std::fs::metadata(common_dir.join("FETCH_HEAD")).ok()?;
    if meta.len() == 0 {
        return None;
    }
    meta.modified().ok()
}

/// A checkout's branch, upstream and tree, from one `git status`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    /// `None` when HEAD is detached.
    pub branch: Option<String>,
    pub upstream: Option<Upstream>,
    pub tree: Tree,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Upstream {
    #[serde(rename = "ref")]
    pub name: String,
    /// `None` when the upstream branch is gone.
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub gone: bool,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Tree {
    pub dirty: bool,
    pub staged: u32,
    pub modified: u32,
    pub untracked: u32,
    pub conflicted: u32,
}

pub fn status(dir: &Path) -> Result<std::result::Result<Status, String>> {
    let mut cmd = git(dir);
    cmd.args(["status", "--porcelain=v2", "--branch", "--untracked-files=normal"]);
    let out = run(cmd)?;
    if !out.status.success() {
        return Ok(Err(first_line(&out.stderr)));
    }
    Ok(Ok(parse_status(&String::from_utf8_lossy(&out.stdout))))
}

fn parse_status(text: &str) -> Status {
    let mut status = Status::default();
    let mut ab = None;
    for line in text.lines() {
        if let Some(head) = line.strip_prefix("# branch.head ") {
            status.branch = (head != "(detached)").then(|| head.to_string());
        } else if let Some(name) = line.strip_prefix("# branch.upstream ") {
            status.upstream = Some(Upstream {
                name: name.to_string(),
                ahead: None,
                behind: None,
                gone: true,
            });
        } else if let Some(counts) = line.strip_prefix("# branch.ab ") {
            let mut parts = counts
                .split(' ')
                .map(|n| n.trim_start_matches(['+', '-']).parse().ok());
            ab = Some((parts.next().flatten(), parts.next().flatten()));
        } else {
            let tree = &mut status.tree;
            match line.as_bytes() {
                [b'1' | b'2', b' ', x, y, ..] => {
                    tree.staged += u32::from(*x != b'.');
                    tree.modified += u32::from(*y != b'.');
                }
                [b'u', ..] => tree.conflicted += 1,
                [b'?', ..] => tree.untracked += 1,
                _ => {}
            }
        }
    }
    if let (Some(upstream), Some((ahead, behind))) = (&mut status.upstream, ab) {
        upstream.ahead = ahead;
        upstream.behind = behind;
        upstream.gone = false;
    }
    let t = &mut status.tree;
    t.dirty = t.staged + t.modified + t.untracked + t.conflicted > 0;
    status
}

/// What one `git fetch REMOTE` did.
pub struct Fetch {
    /// The refs it moved, shortened (`origin/main`).
    pub updated: Vec<String>,
}

/// `git fetch REMOTE`: the one command grove runs that changes anything, and only remote-tracking refs.
/// Prompts are off, so a remote that needs a password fails instead of waiting. `Err` is git's message.
pub fn fetch(dir: &Path, remote: &str) -> Result<std::result::Result<Fetch, String>> {
    let mut cmd = git(dir);
    cmd.args([
        "-c",
        "gc.auto=0",
        "-c",
        "maintenance.auto=false",
        "fetch",
        "--porcelain",
        "--",
        remote,
    ]);
    let out = run(cmd)?;
    if !out.status.success() {
        return Ok(Err(first_line(&out.stderr)));
    }
    let updated = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.starts_with('='))
        .filter_map(|l| l.rsplit(' ').next())
        .map(|r| {
            r.strip_prefix("refs/remotes/")
                .or_else(|| r.strip_prefix("refs/"))
                .unwrap_or(r)
                .to_string()
        })
        .collect();
    Ok(Ok(Fetch { updated }))
}

/// The commit `rev` names, in `dir`.
pub fn rev_parse(dir: &Path, rev: &str) -> Result<Option<String>> {
    Ok(read(dir, &["rev-parse", "--verify", "--quiet", rev])?.map(|s| s.trim().to_string()))
}

/// The contents of `path` at `rev` (`git show REV:PATH`), or `None` when it is not there.
pub fn show(dir: &Path, rev: &str, path: &str) -> Result<Option<String>> {
    read(dir, &["show", &format!("{rev}:{path}")])
}

/// Does the working tree's `path` differ from HEAD?
pub fn differs_from_head(dir: &Path, path: &str) -> Result<bool> {
    let mut cmd = git(dir);
    cmd.args(["diff", "--quiet", "HEAD", "--", path]);
    Ok(run(cmd)?.status.code() == Some(1))
}

/// git's first message line, without `fatal: ` or `error: `.
fn first_line(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let line = text
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("git failed");
    line.strip_prefix("fatal: ")
        .or_else(|| line.strip_prefix("error: "))
        .unwrap_or(line)
        .to_string()
}

/// `path` with symlinks resolved, or as given when it cannot be.
pub fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_v2() {
        let text = "# branch.oid abc\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -1\n\
                    1 .M N... 100644 100644 100644 a b README\n1 A. N... 0 100644 100644 0 b new\n\
                    u UU N... 1 2 3 4 a b c conflict\n? notes.txt\n";
        let status = parse_status(text);
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(
            status.upstream,
            Some(Upstream {
                name: "origin/main".into(),
                ahead: Some(2),
                behind: Some(1),
                gone: false
            })
        );
        assert_eq!(
            status.tree,
            Tree {
                dirty: true,
                staged: 1,
                modified: 1,
                untracked: 1,
                conflicted: 1
            }
        );
    }

    #[test]
    fn a_gone_upstream_has_no_counts() {
        let status = parse_status("# branch.head (detached)\n# branch.upstream origin/x\n");
        assert_eq!(status.branch, None);
        assert!(status.upstream.unwrap().gone);
    }
}
