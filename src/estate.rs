//! The estate's providers: every repository with a signed release tag, its latest release, and the URLs that
//! name it. A pin is matched with a provider by URL identity (`forge::identity`): any remote of the provider's
//! checkout, or an `aliases` entry. grove never guesses an identity from a name.
//!
//! A release is a tag `vMAJOR.MINOR.PATCH` with a signature, read from the checkout's local tags. The latest is
//! the highest. Whose signature it is, is the provider tool's to verify (`tuteur release verify`, later).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::config::Config;
use crate::discover::Repo;
use crate::error::Result;
use crate::forge;
use crate::git::{self, Tag};
use crate::parallel;
use crate::pins::{self, Pin, Via};

#[derive(Debug, Clone, Serialize)]
pub struct Release {
    pub tag: String,
    pub commit: String,
}

#[derive(Debug)]
pub struct Provider {
    /// Index into the discovered repositories.
    pub repo: usize,
    pub name: String,
    pub path: PathBuf,
    pub latest: Release,
    /// Signed releases, by version.
    releases: Vec<((u64, u64, u64), Tag)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PinState {
    Current,
    Behind,
    OffRelease,
}

impl PinState {
    pub fn display(self) -> &'static str {
        match self {
            PinState::Current => "current",
            PinState::Behind => "behind",
            PinState::OffRelease => "off release",
        }
    }
}

impl Provider {
    /// A pin's state against this provider's latest release.
    pub fn state(&self, pin: &Pin) -> PinState {
        if pin.commit.as_deref() == Some(self.latest.commit.as_str())
            || (pin.commit.is_none() && pin.tag.as_deref() == Some(self.latest.tag.as_str()))
        {
            return PinState::Current;
        }
        let release = self.releases.iter().find(|(_, t)| match (&pin.tag, &pin.commit) {
            (Some(tag), Some(commit)) => t.name == *tag && t.commit == *commit,
            (Some(tag), None) => t.name == *tag,
            (None, Some(commit)) => t.commit == *commit,
            (None, None) => false,
        });
        match release {
            Some(_) => PinState::Behind,
            None => PinState::OffRelease,
        }
    }

    /// The command that moves a repository's pin to the latest release (`path` is the consumer's checkout).
    pub fn next(&self, pin: &Pin, path: &std::path::Path) -> Option<String> {
        let path = path.display();
        let tag = &self.latest.tag;
        match &pin.how {
            Via::Pattern(name) => Some(format!("tuteur upgrade -C {path} {name}@{tag}")),
            Via::Tuteur => Some(format!("tuteur upgrade -C {path}")),
            Via::Flake(name, true) => Some(format!(
                "set inputs.{name}.url to ref=refs/tags/{tag} in {path}/flake.nix, then: \
                 nix flake update {name} --flake {path}"
            )),
            Via::Flake(name, false) => Some(format!("nix flake update {name} --flake {path}")),
            Via::Report => None,
        }
    }
}

pub struct Estate {
    pub providers: Vec<Provider>,
    by_identity: BTreeMap<String, usize>,
}

impl Estate {
    /// Read every repository's release tags, in parallel.
    pub fn load(config: &Config, repos: &[Repo]) -> Result<Estate> {
        let tags = parallel::map(repos, |repo| git::tags(&repo.path));
        let mut providers = Vec::new();
        let mut by_identity = BTreeMap::new();
        for (i, (repo, tags)) in repos.iter().zip(tags).enumerate() {
            let mut releases: Vec<_> = tags?
                .into_iter()
                .filter(|t| t.signed)
                .filter_map(|t| Some((pins::version(&t.name)?, t)))
                .collect();
            releases.sort_by_key(|(v, _)| *v);
            let Some((_, latest)) = releases.last() else {
                continue;
            };
            let latest = Release {
                tag: latest.name.clone(),
                commit: latest.commit.clone(),
            };
            let aliases = config
                .aliases
                .iter()
                .filter(|a| a.name == repo.name || repo.forge.as_ref().is_some_and(|f| f.slug == a.name))
                .flat_map(|a| a.addresses.iter());
            let index = providers.len();
            for url in repo.remotes.iter().map(|r| &r.url).chain(aliases) {
                if let Some(id) = forge::identity(url) {
                    by_identity.entry(id).or_insert(index);
                }
            }
            providers.push(Provider {
                repo: i,
                name: repo.name.clone(),
                path: repo.path.clone(),
                latest,
                releases,
            });
        }
        Ok(Estate {
            providers,
            by_identity,
        })
    }

    /// The provider a pin's URL names, if grove has a checkout of it.
    pub fn provider_of(&self, url: &str) -> Option<&Provider> {
        let id = forge::identity(url)?;
        self.by_identity.get(&id).map(|&i| &self.providers[i])
    }

    /// The provider whose repository is `repo` (an index into the discovered repositories).
    pub fn provider_for_repo(&self, repo: usize) -> Option<&Provider> {
        self.providers.iter().find(|p| p.repo == repo)
    }
}
