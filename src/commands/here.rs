//! `grove here [--report]`: what this host owes, with the commands, for each config repository in
//! `hosts.THIS.after`. `--report` prints this host's report (always JSON) for another host's `grove rollout`.

use serde::Serialize;

use super::Setup;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::estate::{Estate, PinState};
use crate::exit::Exit;
use crate::fact::Fact;
use crate::git;
use crate::hosts::{self, Local};
use crate::output::{self, Table, plural};

#[derive(Serialize)]
struct Step {
    /// `pull`, `bump` or `switch`.
    #[serde(rename = "step")]
    kind: &'static str,
    repo: String,
    what: String,
    next: String,
    #[serde(rename = "where")]
    where_: String,
}

#[derive(Serialize)]
struct PinRow {
    repo: String,
    provider: String,
    via: String,
    activated: Option<String>,
    head: Option<String>,
    latest: String,
    state: Option<PinState>,
}

#[derive(Serialize)]
struct ConfigShown<'a> {
    repo: &'a str,
    path: Option<&'a std::path::Path>,
    activated: &'a Fact<String>,
    head: Option<&'a str>,
}

pub fn run(ctx: &Context, report: bool) -> Result<Exit> {
    let setup = Setup::load(ctx)?;
    let locals = hosts::local(ctx, &setup.config, &setup.repos);
    if report {
        output::print_json("here", hosts::report(ctx, &locals));
        return Ok(Exit::Ok);
    }
    if locals.is_empty() {
        return Err(Error::new(format!(
            "{host}: grove.toml has no hosts.{host}.after, so grove does not know which repositories configure \
             this host",
            host = ctx.host
        ))
        .try_("grove config check"));
    }
    let estate = Estate::load(&setup.config, &setup.repos)?;

    let mut steps = Vec::new();
    let mut rows = Vec::new();
    let mut unknown = Vec::new();
    for local in &locals {
        let name = &local.after.repo;
        let Some(repo) = local.repo else {
            unknown.push(format!("{name}: grove found no repository {name}"));
            continue;
        };
        let path = ctx.tilde(&repo.path);
        if let Ok(Ok(status)) = git::status(&repo.path)
            && let Some(behind) = status.upstream.and_then(|u| u.behind).filter(|b| *b > 0)
        {
            steps.push(Step {
                kind: "pull",
                repo: name.clone(),
                what: format!("pull {path}: {behind} behind its upstream"),
                next: format!("git -C {path} pull --ff-only"),
                where_: ctx.host.clone(),
            });
        }
        pins(&estate, local, &mut rows, &mut steps, ctx);
        match (local.activated.get(), &local.head) {
            (Some(activated), Some(head)) if activated != head => steps.push(Step {
                kind: "switch",
                repo: name.clone(),
                what: format!(
                    "switch {path}: activated {}, HEAD {}",
                    short(activated),
                    short(head)
                ),
                next: local.after.command.clone(),
                where_: ctx.host.clone(),
            }),
            (None, _) => unknown.push(format!(
                "{name}: activated commit unknown: {}",
                local.activated.value.as_ref().err().map_or("", String::as_str)
            )),
            _ => {}
        }
    }

    if ctx.json {
        print_json(ctx, &locals, &steps, &rows, &unknown);
    } else {
        print_human(ctx, &steps, &rows, &unknown);
    }
    Ok(Exit::Ok)
}

/// One row per provider this config pins (at the activated commit or HEAD), and a `bump` step for each pin
/// at HEAD that is behind or off release.
fn pins(estate: &Estate, local: &Local, rows: &mut Vec<PinRow>, steps: &mut Vec<Step>, ctx: &Context) {
    let name = &local.after.repo;
    let at_activated = local.at_activated.as_ref().ok();
    let at_head = local.at_head.as_ref().ok();
    let mut providers: Vec<(&str, &crate::estate::Provider)> = at_activated
        .into_iter()
        .chain(at_head)
        .flatten()
        .filter_map(|pin| Some((pin.via.as_str(), estate.provider_of(&pin.url)?)))
        .collect();
    providers.sort_by(|a, b| a.1.name.cmp(&b.1.name).then(a.0.cmp(b.0)));
    providers.dedup_by(|a, b| a.0 == b.0 && a.1.repo == b.1.repo);
    for (via, provider) in providers {
        let find = |pins: Option<&Vec<crate::pins::Pin>>| {
            pins.into_iter()
                .flatten()
                .find(|p| {
                    p.via == via
                        && estate
                            .provider_of(&p.url)
                            .is_some_and(|x| x.repo == provider.repo)
                })
                .cloned()
        };
        let (activated, head) = (find(at_activated), find(at_head));
        let label = |p: &crate::pins::Pin| {
            p.tag
                .clone()
                .or_else(|| p.commit.as_deref().map(short))
                .unwrap_or_else(|| "?".into())
        };
        let state = head.as_ref().map(|p| provider.state(p));
        if let (Some(pin), Some(PinState::Behind | PinState::OffRelease), Some(repo)) =
            (&head, state, local.repo)
            && let Some(next) = provider.next(pin, &repo.path)
        {
            let home = ctx.home.display().to_string();
            steps.push(Step {
                kind: "bump",
                repo: name.clone(),
                what: format!(
                    "bump {} in {}: {} to {}",
                    provider.name,
                    ctx.tilde(&repo.path),
                    label(pin),
                    provider.latest.tag
                ),
                next: next.replace(&format!("{home}/"), "~/"),
                where_: ctx.host.clone(),
            });
        }
        rows.push(PinRow {
            repo: name.clone(),
            provider: provider.name.clone(),
            via: via.to_string(),
            activated: activated.as_ref().map(label),
            head: head.as_ref().map(label),
            latest: provider.latest.tag.clone(),
            state,
        });
    }
}

fn short(commit: &str) -> String {
    commit[..commit.len().min(7)].to_string()
}

fn print_json(ctx: &Context, locals: &[Local], steps: &[Step], rows: &[PinRow], unknown: &[String]) {
    #[derive(Serialize)]
    struct Body<'a> {
        host: &'a str,
        configs: Vec<ConfigShown<'a>>,
        steps: &'a [Step],
        pins: &'a [PinRow],
        unknown: &'a [String],
    }
    let configs = locals
        .iter()
        .map(|l| ConfigShown {
            repo: &l.after.repo,
            path: l.repo.map(|r| r.path.as_path()),
            activated: &l.activated,
            head: l.head.as_deref(),
        })
        .collect();
    output::print_json(
        "here",
        Body {
            host: &ctx.host,
            configs,
            steps,
            pins: rows,
            unknown,
        },
    );
}

fn print_human(ctx: &Context, steps: &[Step], rows: &[PinRow], unknown: &[String]) {
    if steps.is_empty() {
        println!("{} owes nothing", ctx.host);
    } else {
        println!("{} owes {}", ctx.host, plural(steps.len(), "step"));
        for step in steps {
            println!("  {}", step.what);
            println!("    {}", step.next);
        }
    }
    for line in unknown {
        println!("? {line}");
    }
    if !rows.is_empty() {
        println!();
        let mut table = Table::new(["REPO", "PROVIDER", "ACTIVATED", "HEAD", "LATEST"]);
        for row in rows {
            let dash = |v: &Option<String>| v.clone().unwrap_or_else(|| "-".into());
            table.row([
                row.repo.clone(),
                row.provider.clone(),
                dash(&row.activated),
                dash(&row.head),
                row.latest.clone(),
            ]);
        }
        print!("{}", table.render(""));
    }
}
