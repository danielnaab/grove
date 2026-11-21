//! Addresses: how a person names repositories. There is no query language.
//!
//! - `name`: the main checkout's directory name. It is an error when ambiguous, and the error lists the paths.
//! - `owner/name`: from the forge remote.
//! - a path, or `.`: the repository that contains it, for any of its worktrees.
//! - `@group`: a `[tags]` group, or a derived one: `@dirty` (any checkout has changes), `@unpushed` (a checkout's
//!   branch is ahead of its upstream, or has no upstream while the repository has a remote), `@behind`.

use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::context::Context;
use crate::discover::Repo;
use crate::error::{Error, Result};

pub const DERIVED: &[&str] = &["behind", "dirty", "unpushed"];

/// What an address names, before any repository is observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    All,
    One(usize),
    /// A `[tags]` group and its repositories.
    Tag(String, Vec<usize>),
    /// A derived group: which repositories match is known only after observing them.
    Derived(String),
}

impl Target {
    /// `@tools`, for summaries and errors.
    pub fn group(&self) -> Option<String> {
        match self {
            Target::Tag(name, _) | Target::Derived(name) => Some(format!("@{name}")),
            Target::All | Target::One(_) => None,
        }
    }
}

/// Which repositories each tag and this host's production list name, with the line that set them.
pub struct Memberships {
    pub tags: Vec<(String, usize, Vec<usize>)>,
    pub production: Option<(usize, Vec<usize>)>,
}

impl Memberships {
    /// Addresses in grove.toml that match no repository are left out.
    pub fn new(ctx: &Context, config: &Config, repos: &[Repo]) -> Memberships {
        let members = |addresses: &[String]| -> Vec<usize> {
            let mut found: Vec<usize> = addresses
                .iter()
                .filter_map(|a| find(ctx, repos, a).ok())
                .collect();
            found.sort_unstable();
            found.dedup();
            found
        };
        Memberships {
            tags: config
                .tags
                .iter()
                .map(|t| (t.name.clone(), t.line, members(&t.addresses)))
                .collect(),
            production: config
                .production_on(&ctx.host)
                .map(|p| (p.line, members(&p.addresses))),
        }
    }

    pub fn tags_of(&self, repo: usize) -> impl Iterator<Item = &(String, usize, Vec<usize>)> {
        self.tags
            .iter()
            .filter(move |(_, _, members)| members.contains(&repo))
    }

    pub fn is_production(&self, repo: usize) -> bool {
        self.production
            .as_ref()
            .is_some_and(|(_, members)| members.contains(&repo))
    }
}

/// Resolve an address (`None` = every repository).
pub fn target(ctx: &Context, repos: &[Repo], groups: &Memberships, address: Option<&str>) -> Result<Target> {
    let Some(address) = address else {
        return Ok(Target::All);
    };
    let Some(group) = address.strip_prefix('@') else {
        return find(ctx, repos, address).map(Target::One);
    };
    if DERIVED.contains(&group) {
        return Ok(Target::Derived(group.to_string()));
    }
    if let Some((name, _, members)) = groups.tags.iter().find(|(name, _, _)| name == group) {
        return Ok(Target::Tag(name.clone(), members.clone()));
    }
    let mut names: Vec<String> = DERIVED
        .iter()
        .map(ToString::to_string)
        .chain(groups.tags.iter().map(|(name, _, _)| name.clone()))
        .map(|n| format!("@{n}"))
        .collect();
    names.sort();
    Err(Error::new(format!("no group @{group} (groups: {})", names.join(", "))).try_("grove config check"))
}

/// One repository by name, `owner/name` or path.
pub fn find(ctx: &Context, repos: &[Repo], address: &str) -> Result<usize> {
    if looks_like_path(address) {
        return by_path(ctx, repos, &ctx.expand(address), address);
    }
    let named: Vec<usize> = (0..repos.len()).filter(|&i| repos[i].name == address).collect();
    match named.as_slice() {
        [one] => return Ok(*one),
        [] => {}
        many => {
            let paths: Vec<String> = many.iter().map(|&i| ctx.tilde(&repos[i].path)).collect();
            return Err(Error::new(format!(
                "{} repositories are named {address}: {}",
                many.len(),
                paths.join(", ")
            ))
            .try_(format!("grove status {}", paths[0])));
        }
    }
    if address.contains('/') {
        if let Some(i) = repos
            .iter()
            .position(|r| r.forge.as_ref().is_some_and(|f| f.slug == address))
        {
            return Ok(i);
        }
        if Path::new(address).exists() {
            return by_path(ctx, repos, Path::new(address), address);
        }
    }
    Err(Error::new(format!("no repository named {address}")).try_("grove ls"))
}

fn looks_like_path(address: &str) -> bool {
    address == "." || address == ".." || ["/", "./", "../", "~"].iter().any(|p| address.starts_with(p))
}

/// The repository with a checkout that contains `path` (the deepest, when checkouts nest).
fn by_path(ctx: &Context, repos: &[Repo], path: &Path, address: &str) -> Result<usize> {
    let path: PathBuf = path
        .canonicalize()
        .map_err(|_| Error::new(format!("no such directory: {address}")).try_("grove ls --path"))?;
    repos
        .iter()
        .enumerate()
        .flat_map(|(i, r)| r.checkouts.iter().map(move |c| (i, &c.path)))
        .filter(|(_, checkout)| path.starts_with(checkout))
        .max_by_key(|(_, checkout)| checkout.components().count())
        .map(|(i, _)| i)
        .ok_or_else(|| {
            Error::new(format!("{} is not in a repository grove knows", ctx.tilde(&path)))
                .try_("grove config check (see discover.scan)")
        })
}
