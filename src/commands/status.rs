//! `grove status [ADDRESS]`: everything grove knows about one repository, per checkout and for the repository,
//! each fact with its source and age. The default address is `.`.

use serde::Serialize;

use super::Setup;
use crate::context::Context;
use crate::error::Result;
use crate::exit::Exit;
use crate::fact::Fact;
use crate::observe::{Observed, ahead_behind, tree_display};
use crate::output::{self, Table};
use crate::sources::Report;

/// Labels are padded to the longest one, so every section's columns line up.
const LABEL_WIDTH: usize = "production".len();

pub fn run(ctx: &Context, address: &str) -> Result<Exit> {
    let setup = Setup::load(ctx)?;
    let i = setup.one(ctx, address, "status")?;
    let observer = setup.observer(ctx);
    let repo = observer.observe(&[i]).remove(0);
    let report = observer.report(std::slice::from_ref(&repo));
    let exit = if report.partial() { Exit::Partial } else { Exit::Ok };

    if ctx.json {
        #[derive(Serialize)]
        struct Body<'a> {
            repo: &'a Observed,
            sources: &'a Report,
        }
        output::print_json(
            "status",
            Body {
                repo: &repo,
                sources: &report,
            },
        );
    } else {
        print!("{}", render(ctx, &repo));
    }
    Ok(exit)
}

/// A section's rows: label, value, then source and age.
struct Section(Table);

impl Section {
    fn new() -> Self {
        Section(Table::default())
    }

    fn row<T>(&mut self, ctx: &Context, label: &str, fact: &Fact<T>, show: impl Fn(&T) -> String) {
        let value = fact.get().map_or_else(|| "?".into(), show);
        self.0
            .row([format!("{label:<LABEL_WIDTH$}"), value, fact.provenance(ctx.now)]);
    }

    fn render(&self) -> String {
        self.0.render("  ")
    }
}

fn render(ctx: &Context, repo: &Observed) -> String {
    let production = repo.production.get().copied().unwrap_or(false);
    let mut header = vec![repo.name.clone()];
    header.extend(repo.slug.clone());
    header.push(ctx.tilde(&repo.path));
    if production {
        header.push(format!("production on {}: observed only", ctx.host));
    }
    let mut out = format!("{}\n", header.join("  "));

    for checkout in &repo.checkouts {
        let worktree = if checkout.worktree { "  (worktree)" } else { "" };
        out = format!("{out}\n{}{worktree}\n", ctx.tilde(&checkout.path));
        let mut section = Section::new();
        section.row(ctx, "branch", &checkout.branch, |b| {
            b.clone().unwrap_or_else(|| "(detached)".into())
        });
        section.row(ctx, "tree", &checkout.tree, tree_display);
        section.row(ctx, "upstream", &checkout.upstream, |u| match u {
            None => "none".into(),
            Some(up) if up.gone => format!("{} gone", up.name),
            Some(up) => format!("{} {}", up.name, ahead_behind(up)),
        });
        section.row(ctx, "pins", &checkout.pins, crate::sources::Pins::display);
        section.row(ctx, "runs", &checkout.runs, crate::sources::Runs::display);
        out.push_str(&section.render());
    }

    out.push_str("\nrepository\n");
    let mut section = Section::new();
    let local_only = repo.local_only();
    section.row(ctx, "fetched", &repo.fetched, |f| match f {
        _ if local_only => "no remotes".into(),
        Some(t) => output::age(ctx.now, t.0),
        None => "never".into(),
    });
    match repo.remotes.get() {
        Some(remotes) if !remotes.is_empty() => {
            let meta = clone_meta(&repo.remotes);
            for remote in remotes {
                section.row(ctx, "remote", &meta, |()| {
                    format!("{} {}", remote.name, remote.url)
                });
            }
        }
        _ => section.row(ctx, "remote", &repo.remotes, |_| "none".into()),
    }
    section.row(ctx, "forge", &repo.forge, |f| {
        f.clone().unwrap_or_else(|| "none".into())
    });
    section.row(ctx, "tags", &repo.tags, |tags| {
        if tags.is_empty() {
            "none".into()
        } else {
            tags.iter()
                .map(|t| format!("@{t}"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    });
    section.row(ctx, "production", &repo.production, |p| {
        if *p {
            format!("yes, on {}: observed only", ctx.host)
        } else {
            "no".into()
        }
    });
    section.row(ctx, "last sync", &repo.last_sync, |s| {
        s.as_ref()
            .map_or_else(|| "none recorded".into(), crate::state::LastSync::display)
    });
    out.push_str(&section.render());
    out
}

/// A fact's source and time, for a row that shows one part of its value.
fn clone_meta<T>(fact: &Fact<T>) -> Fact<()> {
    Fact {
        value: Ok(()),
        source: fact.source.clone(),
        observed_at: fact.observed_at,
    }
}
