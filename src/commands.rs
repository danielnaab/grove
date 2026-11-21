//! One module per command. Each prints its own output and returns an exit code; an `Err` is printed by `main`.
//! Specified by `docs/reference/cli/<command>.md`.

pub mod config_check;
pub mod ls;
pub mod open;
pub mod status;
pub mod sync;

use crate::address::{self, Memberships, Target};
use crate::config::Config;
use crate::context::Context;
use crate::discover::{self, Repo};
use crate::error::{Error, Result};
use crate::observe::{Observed, Observer};
use crate::sources::Sources;
use crate::state::State;

/// What every repository command starts from: the config, the repositories it finds, and their groups.
pub struct Setup {
    pub config: Config,
    pub repos: Vec<Repo>,
    pub groups: Memberships,
}

impl Setup {
    pub fn load(ctx: &Context) -> Result<Setup> {
        let config = Config::load(&ctx.home)?;
        let repos = discover::discover(ctx, &config)?;
        let groups = Memberships::new(ctx, &config, &repos);
        Ok(Setup { config, repos, groups })
    }

    pub fn target(&self, ctx: &Context, address: Option<&str>) -> Result<Target> {
        address::target(ctx, &self.repos, &self.groups, address)
    }

    /// An observer, which reads pot's runs once.
    pub fn observer<'a>(&'a self, ctx: &'a Context) -> Observer<'a> {
        Observer {
            ctx,
            config: &self.config,
            repos: &self.repos,
            groups: &self.groups,
            sources: Sources::load(),
            state: State::load(&ctx.state_dir()),
        }
    }

    /// The repositories a target names, without observing them (a derived group needs `observed`).
    pub fn indices(&self, target: &Target) -> Vec<usize> {
        match target {
            Target::All | Target::Derived(_) => (0..self.repos.len()).collect(),
            Target::One(i) => vec![*i],
            Target::Tag(_, members) => members.clone(),
        }
    }

    /// Observe what a target names.
    pub fn observed(&self, observer: &Observer, target: &Target) -> (Vec<usize>, Vec<Observed>) {
        let which = self.indices(target);
        let observed = observer.observe(&which);
        let keep: Vec<bool> = observed
            .iter()
            .map(|o| match target {
                Target::Derived(name) => o.in_derived(name),
                _ => true,
            })
            .collect();
        let mut kept = keep.iter();
        let which = which.into_iter().filter(|_| *kept.next().unwrap_or(&false)).collect();
        let mut kept = keep.iter();
        let observed = observed.into_iter().filter(|_| *kept.next().unwrap_or(&false)).collect();
        (which, observed)
    }

    /// Exactly one repository, for `status` and `open`.
    pub fn one(&self, ctx: &Context, address: &str, command: &str) -> Result<usize> {
        let target = self.target(ctx, Some(address))?;
        let which = match &target {
            Target::One(i) => return Ok(*i),
            Target::Derived(_) => self.observed(&self.observer(ctx), &target).0,
            _ => self.indices(&target),
        };
        match which.as_slice() {
            [one] => Ok(*one),
            [] => Err(Error::new(format!("no repos in {address}")).try_("grove ls")),
            many => {
                let names: Vec<&str> = many.iter().map(|&i| self.repos[i].name.as_str()).collect();
                Err(Error::new(format!(
                    "{command} takes one repository, and {address} has {}: {}",
                    many.len(),
                    names.join(", ")
                ))
                .try_(format!("grove ls {address}")))
            }
        }
    }
}
