//! `grove rollout [PROVIDER]`: the convergence view. For each provider, its latest signed release against every
//! pin of it: repositories (status/1 and flake.lock at HEAD) and hosts (this host's activated commit, other
//! hosts' reports). Derived each time; nothing is stored or changed.

use std::fmt::Write;
use std::path::PathBuf;

use serde::Serialize;

use super::Setup;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::estate::{Estate, PinState, Provider, Release};
use crate::exit::Exit;
use crate::fact::Fact;
use crate::git;
use crate::hosts::{self, Host};
use crate::output::{self, Table, plural};
use crate::parallel;
use crate::pins::{self, Pin};
use crate::sources::{Sources, State};

#[derive(Serialize)]
struct Row {
    consumer: String,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
    via: String,
    pin: PinShown,
    state: PinState,
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    where_: Option<String>,
    source: String,
    observed_at: crate::observe::Time,
    #[serde(skip)]
    host: bool,
}

#[derive(Serialize)]
struct PinShown {
    tag: Option<String>,
    commit: Option<String>,
}

#[derive(Serialize)]
struct Shown<'a> {
    name: &'a str,
    path: &'a PathBuf,
    latest: &'a Release,
    consumers: Vec<Row>,
}

#[derive(Serialize)]
struct HostShown {
    host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    unknown: Option<String>,
    source: String,
    observed_at: crate::observe::Time,
}

#[derive(Serialize)]
struct Summary {
    providers: usize,
    converged: usize,
    behind: usize,
    off_release: usize,
}

#[derive(Serialize)]
struct SourcesShown {
    tuteur: State,
}

pub fn run(ctx: &Context, provider: Option<&str>) -> Result<Exit> {
    let setup = Setup::load(ctx)?;
    let estate = Estate::load(&setup.config, &setup.repos)?;
    let only = match provider {
        None => None,
        Some(address) => {
            let i = setup.one(ctx, address, "rollout")?;
            let Some(p) = estate.provider_for_repo(i) else {
                let path = ctx.tilde(&setup.repos[i].path);
                return Err(Error::new(format!(
                    "{} has no signed release tag (vMAJOR.MINOR.PATCH)",
                    setup.repos[i].name
                ))
                .try_(format!("git -C {path} tag -l 'v*'")));
            };
            Some(p.repo)
        }
    };

    let mut rows: Vec<(usize, Row)> = Vec::new();
    let (tuteur, repos_partial) = repo_rows(ctx, &setup, &estate, &mut rows);
    let (hosts_shown, hosts_partial) = host_rows(ctx, &setup, &estate, &mut rows);

    // Providers with consumers (or the one asked for), by name.
    let mut shown_providers: Vec<Shown> = estate
        .providers
        .iter()
        .filter(|p| only.is_none_or(|o| o == p.repo))
        .map(|p| shown_provider(p, &mut rows))
        .filter(|s| only.is_some() || !s.consumers.is_empty())
        .collect();
    shown_providers.sort_by(|a, b| a.name.cmp(b.name));

    let summary = Summary {
        providers: shown_providers.len(),
        converged: shown_providers
            .iter()
            .filter(|s| s.consumers.iter().all(|r| r.state == PinState::Current))
            .count(),
        behind: count(&shown_providers, PinState::Behind),
        off_release: count(&shown_providers, PinState::OffRelease),
    };
    let exit = if repos_partial || hosts_partial {
        Exit::Partial
    } else {
        Exit::Ok
    };

    if ctx.json {
        #[derive(Serialize)]
        struct Body<'a> {
            providers: &'a [Shown<'a>],
            hosts: &'a [HostShown],
            summary: &'a Summary,
            sources: SourcesShown,
        }
        output::print_json(
            "rollout",
            Body {
                providers: &shown_providers,
                hosts: &hosts_shown,
                summary: &summary,
                sources: SourcesShown { tuteur },
            },
        );
        return Ok(exit);
    }

    print_human(ctx, &shown_providers, &summary, &hosts_shown, &tuteur);
    Ok(exit)
}

fn shown(pin: &Pin) -> PinShown {
    PinShown {
        tag: pin.tag.clone(),
        commit: pin.commit.clone(),
    }
}

/// Take this provider's rows out of `rows`: repositories by name, then hosts by name.
fn shown_provider<'a>(p: &'a Provider, rows: &mut Vec<(usize, Row)>) -> Shown<'a> {
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(rows).into_iter().partition(|(r, _)| *r == p.repo);
    *rows = rest;
    let mut consumers: Vec<Row> = mine.into_iter().map(|(_, row)| row).collect();
    consumers.sort_by(|a, b| (a.host, &a.consumer, &a.via).cmp(&(b.host, &b.consumer, &b.via)));
    Shown {
        name: &p.name,
        path: &p.path,
        latest: &p.latest,
        consumers,
    }
}

fn count(providers: &[Shown], state: PinState) -> usize {
    providers
        .iter()
        .flat_map(|s| &s.consumers)
        .filter(|r| r.state == state)
        .count()
}

fn print_provider(ctx: &Context, shown: &Shown) {
    println!("{} {}  {}", shown.name, shown.latest.tag, ctx.tilde(shown.path));
    if shown.consumers.is_empty() {
        println!("  no consumers");
        return;
    }
    let mut table = Table::new(["CONSUMER", "VIA", "PIN", "STATE"]);
    for row in &shown.consumers {
        let pin = row
            .pin
            .tag
            .clone()
            .or_else(|| row.pin.commit.as_ref().map(|c| c[..c.len().min(7)].to_string()))
            .unwrap_or_else(|| "?".into());
        let mut state = row.state.display().to_string();
        if row.host && row.observed_at.0 < ctx.now {
            state = format!("{state}, as of {}", output::age(ctx.now, row.observed_at.0));
        }
        table.row([row.consumer.clone(), row.via.clone(), pin, state]);
    }
    let rendered = table.render("  ");
    let mut lines = rendered.lines();
    println!("{}", lines.next().unwrap_or_default());
    for (line, row) in lines.zip(&shown.consumers) {
        println!("{line}");
        if let Some(next) = &row.next {
            let next = tilde_all(ctx, next);
            if row.host {
                println!("    next on {}: {next}", row.where_.as_deref().unwrap_or("?"));
            } else {
                println!("    next: {next}");
            }
        }
    }
}

/// A command line for people: the home directory as `~`.
fn tilde_all(ctx: &Context, text: &str) -> String {
    let home = ctx.home.display().to_string();
    text.replace(&format!("{home}/"), "~/")
}

/// `2 providers: 0 converged, 2 pins behind`, `1 provider: 1 converged`.
fn summary_line(s: &Summary) -> String {
    let mut line = format!("{}: {} converged", plural(s.providers, "provider"), s.converged);
    if s.behind > 0 {
        let _ = write!(line, ", {} behind", plural(s.behind, "pin"));
    }
    if s.off_release > 0 {
        let _ = write!(line, ", {} off release", plural(s.off_release, "pin"));
    }
    line
}

/// Rows for every repository's pins: status/1 and flake.lock at HEAD, for each main checkout. Returns the pins
/// tool's state and whether any source failed.
fn repo_rows(ctx: &Context, setup: &Setup, estate: &Estate, rows: &mut Vec<(usize, Row)>) -> (State, bool) {
    let sources = Sources::pins_only();
    let read = parallel::map(&setup.repos, |repo| {
        let status = sources.pinned(ctx, &repo.path);
        let flake = match git::show(&repo.path, "HEAD", "flake.lock") {
            Ok(Some(text)) => pins::from_flake_lock(&text),
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(e.message),
        };
        (status, flake)
    });
    let tuteur = sources.tuteur_state(read.iter().map(|(status, _)| status));
    let mut partial = matches!(tuteur, State::Failed(_));

    for (i, (repo, (status, flake))) in setup.repos.iter().zip(&read).enumerate() {
        let flake_fact = match flake {
            Ok(pins) => Fact::known(pins.clone(), "flake.lock", ctx.now),
            Err(reason) => {
                partial = true;
                Fact::unknown(reason.clone(), "flake.lock", ctx.now)
            }
        };
        let mut seen: Vec<(usize, Option<String>)> = Vec::new();
        for fact in [status, &flake_fact] {
            for pin in fact.get().into_iter().flatten() {
                let Some(p) = estate.provider_of(&pin.url) else {
                    continue;
                };
                if p.repo == i {
                    continue; // a provider's pin of itself
                }
                let key = (p.repo, pin.commit.clone());
                if seen.contains(&key) {
                    continue; // the same pin twice (tuteur's own pin is also a flake input)
                }
                seen.push(key);
                let state = p.state(pin);
                let next = (state != PinState::Current)
                    .then(|| p.next(pin, &repo.path))
                    .flatten();
                rows.push((
                    p.repo,
                    Row {
                        consumer: repo.name.clone(),
                        kind: "repo",
                        path: Some(repo.path.clone()),
                        via: pin.via.clone(),
                        pin: shown(pin),
                        state,
                        where_: next.as_ref().map(|_| ctx.host.clone()),
                        next,
                        source: fact.source.clone(),
                        observed_at: crate::observe::Time(fact.observed_at),
                        host: false,
                    },
                ));
            }
        }
    }

    (tuteur, partial)
}

/// Rows for every host's pins: this host's activated commits, then other hosts' reports. Returns the hosts and
/// whether any report was unreadable.
fn host_rows(
    ctx: &Context,
    setup: &Setup,
    estate: &Estate,
    rows: &mut Vec<(usize, Row)>,
) -> (Vec<HostShown>, bool) {
    let mut partial = false;
    let mut hosts: Vec<Host> = hosts::this_host(ctx, &setup.config, &setup.repos)
        .into_iter()
        .collect();
    hosts.extend(hosts::reports(ctx));
    hosts.sort_by(|a, b| a.name.cmp(&b.name));
    for host in &hosts {
        partial |= host.unreadable;
        for config in host.configs.get().into_iter().flatten() {
            let short = config.activated.as_deref().map_or("?", |c| &c[..c.len().min(7)]);
            let after = setup
                .config
                .after_on(&host.name)
                .into_iter()
                .find(|a| a.repo == config.repo)
                .map(|a| a.command.clone());
            for pin in &config.pins {
                let Some(p) = estate.provider_of(&pin.url) else {
                    continue;
                };
                let state = p.state(pin);
                let next = if state == PinState::Current {
                    None
                } else {
                    after.clone()
                };
                rows.push((
                    p.repo,
                    Row {
                        consumer: format!("host {}", host.name),
                        kind: "host",
                        path: None,
                        via: format!("activated {} {short}", config.repo),
                        pin: shown(pin),
                        state,
                        where_: next.as_ref().map(|_| host.name.clone()),
                        next,
                        source: host.configs.source.clone(),
                        observed_at: crate::observe::Time(host.configs.observed_at),
                        host: true,
                    },
                ));
            }
        }
    }

    let shown = hosts
        .iter()
        .map(|h| HostShown {
            host: h.name.clone(),
            unknown: h.configs.value.as_ref().err().cloned(),
            source: h.configs.source.clone(),
            observed_at: crate::observe::Time(h.configs.observed_at),
        })
        .collect();
    (shown, partial)
}

fn print_human(
    ctx: &Context,
    shown_providers: &[Shown],
    summary: &Summary,
    hosts_shown: &[HostShown],
    tuteur: &State,
) {
    for shown in shown_providers {
        print_provider(ctx, shown);
        println!();
    }
    println!("{}", summary_line(summary));
    for host in hosts_shown {
        if let Some(reason) = &host.unknown {
            let when = if host.observed_at.0 >= ctx.now {
                "now".to_string()
            } else {
                format!("as of {}", output::age(ctx.now, host.observed_at.0))
            };
            println!("? host {}: {reason}, {when}", host.host);
        }
    }
    if let State::Absent(detail) | State::Failed(detail) = tuteur {
        println!("? pins: {detail}");
    }
}
