//! Discovery: from `[discover] scan` globs to repositories.
//!
//! 1. Expand each scan glob and drop matches of any `ignore` glob. A directory is a checkout when it is the top
//!    of a git work tree (`git rev-parse --show-toplevel`). Other directories are skipped silently.
//! 2. Group checkouts by canonical git common dir. One common dir is one repository, and its worktrees are merged
//!    under it.
//! 3. Add worktrees that the scan missed, from `git worktree list --porcelain`.
//! 4. Name each repository by its main checkout's directory name. `owner/name` comes from the forge remote.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::Config;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::forge::{self, Forge};
use crate::git::{self, Remote};
use crate::parallel;

#[derive(Debug, Clone)]
pub struct Repo {
    pub name: String,
    /// The main checkout.
    pub path: PathBuf,
    pub common_dir: PathBuf,
    /// The main checkout first, then worktrees by path.
    pub checkouts: Vec<Checkout>,
    pub remotes: Vec<Remote>,
    pub forge: Option<Forge>,
}

#[derive(Debug, Clone)]
pub struct Checkout {
    pub path: PathBuf,
    pub worktree: bool,
}

impl Repo {
    /// The `forge::primary` remote's name, when there is one.
    pub fn primary_remote(&self) -> Option<&Remote> {
        forge::primary(&self.remotes)
    }
}

pub fn discover(ctx: &Context, config: &Config) -> Result<Vec<Repo>> {
    let ignore: Vec<glob::Pattern> = config
        .ignore
        .iter()
        .map(|g| glob::Pattern::new(&ctx.expand(g).to_string_lossy()))
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| bad_glob(&e.to_string()))?;
    let ignored = |p: &std::path::Path| ignore.iter().any(|g| g.matches_path(p));

    let mut candidates = Vec::new();
    for scan in &config.scan {
        let pattern = ctx.expand(scan);
        let paths = glob::glob(&pattern.to_string_lossy()).map_err(|e| bad_glob(&e.to_string()))?;
        candidates.extend(paths.flatten().filter(|p| p.is_dir() && p.join(".git").exists()));
    }
    candidates.retain(|p| !ignored(p));
    candidates.sort();
    candidates.dedup();

    let probes = parallel::map(&candidates, |dir| git::probe(dir));
    let mut first_checkout: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    for probe in probes {
        if let Some(probe) = probe? {
            first_checkout.entry(probe.common_dir).or_insert(probe.toplevel);
        }
    }

    let found: Vec<(PathBuf, PathBuf)> = first_checkout.into_iter().collect();
    let repos = parallel::map(&found, |(common_dir, checkout)| -> Result<Repo> {
        let mut paths = git::worktrees(checkout)?;
        if paths.is_empty() {
            paths.push(checkout.clone());
        }
        let main = paths[0].clone();
        let mut worktrees: Vec<PathBuf> = paths[1..].iter().filter(|p| !ignored(p)).cloned().collect();
        worktrees.sort();
        let checkouts = std::iter::once(Checkout {
            path: main.clone(),
            worktree: false,
        })
        .chain(
            worktrees
                .into_iter()
                .map(|path| Checkout { path, worktree: true }),
        )
        .collect();
        let remotes = git::remotes(&main)?;
        let forge = forge::primary(&remotes).and_then(|r| forge::parse(&r.url));
        let name = main
            .file_name()
            .map_or_else(|| main.display().to_string(), |n| n.to_string_lossy().into());
        Ok(Repo {
            name,
            path: main,
            common_dir: common_dir.clone(),
            checkouts,
            remotes,
            forge,
        })
    });
    let mut repos = repos.into_iter().collect::<Result<Vec<_>>>()?;
    repos.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.path.cmp(&b.path)));
    Ok(repos)
}

fn bad_glob(message: &str) -> Error {
    Error::new(format!("grove.toml has a bad glob: {message}")).try_("grove config check")
}
