//! `grove ls [ADDRESS] [--path]`: one row per checkout (worktrees indented under their repository): branch, tree,
//! sync state, pins, runs, last fetch. `--path` prints main checkout paths only, for `gcd`.

use serde::Serialize;

use super::Setup;
use crate::address::Target;
use crate::context::Context;
use crate::error::Result;
use crate::exit::Exit;
use crate::observe::Observed;
use crate::output::{self, Table, plural};
use crate::sources::{Report, State};

#[derive(Serialize)]
struct Summary {
    repos: usize,
    dirty: usize,
    unpushed: usize,
    behind: usize,
}

impl Summary {
    fn of(observed: &[Observed]) -> Summary {
        let count = |f: fn(&Observed) -> bool| observed.iter().filter(|o| f(o)).count();
        Summary {
            repos: observed.len(),
            dirty: count(Observed::dirty),
            unpushed: count(Observed::unpushed),
            behind: count(Observed::behind),
        }
    }
}

pub fn run(ctx: &Context, address: Option<&str>, paths_only: bool) -> Result<Exit> {
    let setup = Setup::load(ctx)?;
    let target = setup.target(ctx, address)?;

    if paths_only && !matches!(target, Target::Derived(_)) {
        let paths: Vec<_> = setup.indices(&target).into_iter().map(|i| setup.repos[i].path.clone()).collect();
        print_paths(ctx, &paths);
        return Ok(Exit::Ok);
    }

    let observer = setup.observer(ctx);
    let (_, observed) = setup.observed(&observer, &target);
    if paths_only {
        print_paths(ctx, &observed.iter().map(|o| o.path.clone()).collect::<Vec<_>>());
        return Ok(Exit::Ok);
    }
    let report = observer.report(&observed);
    let exit = if report.partial() { Exit::Partial } else { Exit::Ok };

    if ctx.json {
        #[derive(Serialize)]
        struct Body<'a> {
            repos: &'a [Observed],
            summary: Summary,
            sources: &'a Report,
        }
        output::print_json("ls", Body { repos: &observed, summary: Summary::of(&observed), sources: &report });
        return Ok(exit);
    }

    let group = target.group();
    if observed.is_empty() {
        println!("no repos{}", group.map(|g| format!(" in {g}")).unwrap_or_default());
        return Ok(exit);
    }
    print!("{}", table(ctx, &observed).render(""));
    println!();
    println!("{}", summary_line(&Summary::of(&observed), group.as_deref()));
    for (column, state) in report.unknown() {
        if let State::Absent(detail) | State::Failed(detail) = state {
            println!("? {column}: {detail}");
        }
    }
    Ok(exit)
}

fn print_paths(ctx: &Context, paths: &[std::path::PathBuf]) {
    if ctx.json {
        #[derive(Serialize)]
        struct Body<'a> {
            paths: &'a [std::path::PathBuf],
        }
        output::print_json("ls", Body { paths });
    } else {
        for path in paths {
            println!("{}", path.display());
        }
    }
}

fn table(ctx: &Context, observed: &[Observed]) -> Table {
    let mut table = Table::new(["REPO", "BRANCH", "TREE", "SYNC", "PINS", "RUNS", "FETCHED"]);
    for repo in observed {
        for checkout in &repo.checkouts {
            let name = if checkout.worktree {
                let dir = checkout.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                format!("└ {dir}")
            } else {
                repo.name.clone()
            };
            let branch = match &checkout.branch.value {
                Ok(Some(branch)) => branch.clone(),
                Ok(None) => "(detached)".into(),
                Err(_) => "?".into(),
            };
            let tree = checkout.tree.get().map_or("?", |t| if t.dirty { "dirty" } else { "clean" });
            let fetched = if checkout.worktree { String::new() } else { repo.fetched_display(ctx.now) };
            table.row([
                name,
                branch,
                tree.to_string(),
                repo.sync_state(checkout),
                checkout.pins.get().map_or_else(|| "?".into(), crate::sources::Pins::display),
                checkout.runs.get().map_or_else(|| "?".into(), crate::sources::Runs::display),
                fetched,
            ]);
        }
    }
    table
}

/// `5 repos: 1 dirty, 3 unpushed, 1 behind`, or `2 repos in @tools: ...`.
fn summary_line(summary: &Summary, group: Option<&str>) -> String {
    let mut line = plural(summary.repos, "repo");
    if let Some(group) = group {
        line.push_str(&format!(" in {group}"));
    }
    let parts: Vec<String> = [("dirty", summary.dirty), ("unpushed", summary.unpushed), ("behind", summary.behind)]
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .map(|(what, n)| format!("{n} {what}"))
        .collect();
    if !parts.is_empty() {
        line.push_str(": ");
        line.push_str(&parts.join(", "));
    }
    line
}
