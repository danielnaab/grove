//! Observing repositories: every fact grove shows about a repository and its checkouts, read now.
//!
//! The `Observed` model is what `ls --json` lists and `status --json` shows, field for field.

use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Serialize, Serializer};

use crate::address::Memberships;
use crate::config::Config;
use crate::context::Context;
use crate::discover::Repo;
use crate::fact::Fact;
use crate::git::{self, Remote, Tree, Upstream};
use crate::output;
use crate::parallel;
use crate::sources::{Pins, Report, Runs, Sources};
use crate::state::{LastSync, State};

/// A time that serializes as RFC 3339.
#[derive(Debug, Clone, Copy)]
pub struct Time(pub SystemTime);

impl Serialize for Time {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&output::timestamp(self.0))
    }
}

#[derive(Debug, Serialize)]
pub struct Observed {
    pub name: String,
    pub slug: Option<String>,
    pub path: PathBuf,
    pub common_dir: PathBuf,
    pub tags: Fact<Vec<String>>,
    pub production: Fact<bool>,
    pub fetched: Fact<Option<Time>>,
    pub remotes: Fact<Vec<Remote>>,
    pub forge: Fact<Option<String>>,
    pub last_sync: Fact<Option<LastSync>>,
    pub checkouts: Vec<CheckoutFacts>,
}

#[derive(Debug, Serialize)]
pub struct CheckoutFacts {
    pub path: PathBuf,
    pub worktree: bool,
    pub branch: Fact<Option<String>>,
    pub tree: Fact<Tree>,
    pub upstream: Fact<Option<Upstream>>,
    pub pins: Fact<Pins>,
    pub runs: Fact<Runs>,
}

impl Observed {
    pub fn local_only(&self) -> bool {
        self.remotes.get().is_some_and(Vec::is_empty)
    }

    pub fn dirty(&self) -> bool {
        self.checkouts.iter().any(|c| c.tree.get().is_some_and(|t| t.dirty))
    }

    /// A checkout's branch is ahead of its upstream, or has none while the repository has a remote.
    pub fn unpushed(&self) -> bool {
        self.checkouts.iter().any(|c| match (c.branch.get(), c.upstream.get()) {
            (_, Some(Some(up))) => up.ahead.is_some_and(|n| n > 0),
            (Some(Some(_)), Some(None)) => !self.local_only(),
            _ => false,
        })
    }

    pub fn behind(&self) -> bool {
        self.checkouts.iter().any(|c| c.upstream.get().is_some_and(|u| u.as_ref().and_then(|u| u.behind) > Some(0)))
    }

    /// Is this repository in the derived group `@name`?
    pub fn in_derived(&self, name: &str) -> bool {
        match name {
            "dirty" => self.dirty(),
            "unpushed" => self.unpushed(),
            "behind" => self.behind(),
            _ => false,
        }
    }

    /// The SYNC column for a checkout: `even`, `↑2`, `↓1`, `↑2 ↓1`, `no upstream`, `gone`, `local only`.
    pub fn sync_state(&self, checkout: &CheckoutFacts) -> String {
        if self.local_only() {
            return "local only".into();
        }
        match &checkout.upstream.value {
            Err(_) => "?".into(),
            Ok(None) => "no upstream".into(),
            Ok(Some(up)) if up.gone => "gone".into(),
            Ok(Some(up)) => ahead_behind(up),
        }
    }

    /// The FETCHED column: `4s ago`, `never`, or `-` when there is no remote.
    pub fn fetched_display(&self, now: SystemTime) -> String {
        match self.fetched.get() {
            _ if self.local_only() => "-".into(),
            Some(Some(Time(t))) => output::age(now, *t),
            Some(None) => "never".into(),
            None => "?".into(),
        }
    }
}

/// `even`, `↑2`, `↓1`, `↑2 ↓1`.
pub fn ahead_behind(up: &Upstream) -> String {
    match (up.ahead.unwrap_or(0), up.behind.unwrap_or(0)) {
        (0, 0) => "even".into(),
        (a, 0) => format!("↑{a}"),
        (0, b) => format!("↓{b}"),
        (a, b) => format!("↑{a} ↓{b}"),
    }
}

/// `clean`, or `1 modified, 1 untracked`.
pub fn tree_display(tree: &Tree) -> String {
    let parts: Vec<String> = [
        ("staged", tree.staged),
        ("modified", tree.modified),
        ("untracked", tree.untracked),
        ("conflicted", tree.conflicted),
    ]
    .into_iter()
    .filter(|(_, n)| *n > 0)
    .map(|(what, n)| format!("{n} {what}"))
    .collect();
    if parts.is_empty() { "clean".into() } else { parts.join(", ") }
}

pub struct Observer<'a> {
    pub ctx: &'a Context,
    pub config: &'a Config,
    pub repos: &'a [Repo],
    pub groups: &'a Memberships,
    pub sources: Sources,
    pub state: State,
}

impl Observer<'_> {
    /// Observe these repositories, in parallel, in the order given.
    pub fn observe(&self, which: &[usize]) -> Vec<Observed> {
        parallel::map(which, |&i| self.observe_one(i))
    }

    fn observe_one(&self, i: usize) -> Observed {
        let (ctx, repo, now) = (self.ctx, &self.repos[i], self.ctx.now);
        let label = &self.config.label;

        let tags: Vec<_> = self.groups.tags_of(i).collect();
        let tags_source = tags.first().map_or_else(|| label.clone(), |(_, line, _)| self.config.source(*line));
        let tags = Fact::known(tags.iter().map(|(name, _, _)| name.clone()).collect(), tags_source, now);

        let production = self.groups.is_production(i);
        let production_source = match &self.groups.production {
            Some((line, _)) if production => self.config.source(*line),
            _ => label.clone(),
        };

        let forge = match repo.primary_remote() {
            Some(remote) => {
                Fact::known(repo.forge.as_ref().map(|f| f.url.clone()), format!("remote {}", remote.name), now)
            }
            None => Fact::known(None, "git config", now),
        };

        let last_sync = match self.state.last_sync(&repo.common_dir) {
            Some(sync) => Fact::known(Some(sync.clone()), "grove state", sync.time().unwrap_or(now)),
            None => Fact::known(None, "grove state", now),
        };

        Observed {
            name: repo.name.clone(),
            slug: repo.forge.as_ref().map(|f| f.slug.clone()),
            path: repo.path.clone(),
            common_dir: repo.common_dir.clone(),
            tags,
            production: Fact::known(production, production_source, now),
            fetched: Fact::known(git::fetched_at(&repo.common_dir).map(Time), "FETCH_HEAD", now),
            remotes: Fact::known(repo.remotes.clone(), "git config", now),
            forge,
            last_sync,
            checkouts: repo.checkouts.iter().map(|c| self.checkout(&c.path, c.worktree)).collect(),
        }
    }

    fn checkout(&self, path: &std::path::Path, worktree: bool) -> CheckoutFacts {
        let now = self.ctx.now;
        let (branch, tree, upstream) = match git::status(path) {
            Ok(Ok(status)) => (
                Fact::known(status.branch, "git", now),
                Fact::known(status.tree, "git", now),
                Fact::known(status.upstream, "git", now),
            ),
            Ok(Err(reason)) | Err(crate::error::Error { message: reason, .. }) => (
                Fact::unknown(reason.clone(), "git", now),
                Fact::unknown(reason.clone(), "git", now),
                Fact::unknown(reason, "git", now),
            ),
        };
        CheckoutFacts {
            path: path.to_path_buf(),
            worktree,
            branch,
            tree,
            upstream,
            pins: self.sources.pins(self.ctx, path),
            runs: self.sources.runs(self.ctx, path),
        }
    }

    /// The sources' states over these observations. git is `failed` when any checkout could not be read.
    pub fn report(&self, observed: &[Observed]) -> Report {
        let checkouts = || observed.iter().flat_map(|o| &o.checkouts);
        let mut report = self.sources.report(checkouts().map(|c| &c.pins));
        if let Some(reason) = checkouts().find_map(|c| c.tree.value.as_ref().err()) {
            report.git = crate::sources::State::Failed(reason.clone());
        }
        report
    }
}
