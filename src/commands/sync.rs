//! `grove sync [ADDRESS]`: `git fetch` every remote of every addressed repository in parallel. It never merges.
//! Production repositories on this host are skipped. A failed remote makes the exit code 3. Outcomes are recorded
//! in `state`, for `grove status`.

use std::time::SystemTime;

use serde::Serialize;

use super::Setup;
use crate::context::Context;
use crate::discover::Repo;
use crate::error::{Error, Result};
use crate::exit::Exit;
use crate::git;
use crate::observe::{Time, sync_state};
use crate::output::{self, Table, plural};
use crate::parallel;
use crate::state::{LastSync, RemoteOutcome, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Updated,
    Unchanged,
    Failed,
    Skipped,
    NoRemotes,
}

/// One row: a remote fetched, or a repository not fetched.
#[derive(Serialize)]
struct Row {
    repo: String,
    remote: Option<String>,
    result: Outcome,
    updated: Vec<String>,
    error: Option<String>,
    observed_at: Time,
    #[serde(skip)]
    repo_index: usize,
}

#[derive(Serialize, Default)]
struct Summary {
    repos: usize,
    fetched: usize,
    failed: usize,
    skipped: usize,
    local_only: usize,
}

pub fn run(ctx: &Context, address: Option<&str>) -> Result<Exit> {
    let setup = Setup::load(ctx)?;
    let target = setup.target(ctx, address)?;
    let which = match target {
        crate::address::Target::Derived(_) => setup.observed(&setup.observer(ctx), &target).0,
        _ => setup.indices(&target),
    };

    let mut rows = Vec::new();
    let mut jobs = Vec::new();
    for &i in &which {
        let repo = &setup.repos[i];
        let skip = |result| Row {
            repo: repo.name.clone(),
            remote: None,
            result,
            updated: Vec::new(),
            error: None,
            observed_at: Time(ctx.now),
            repo_index: i,
        };
        if setup.groups.is_production(i) {
            rows.push(skip(Outcome::Skipped));
        } else if repo.remotes.is_empty() {
            rows.push(skip(Outcome::NoRemotes));
        } else {
            jobs.extend(repo.remotes.iter().map(|r| (i, r.name.clone())));
        }
    }
    let fetched = parallel::map(&jobs, |(i, remote)| fetch(&setup.repos[*i], *i, remote));
    rows.extend(fetched.into_iter().collect::<Result<Vec<_>>>()?);
    // Repositories in name order, each one's remotes in config order.
    let position = |i: usize| which.iter().position(|&w| w == i).unwrap_or(usize::MAX);
    rows.sort_by_key(|r| position(r.repo_index));

    record(ctx, &setup.repos, &rows)?;
    let summary = summarize(&which, &rows);
    let exit = if summary.failed > 0 { Exit::Partial } else { Exit::Ok };

    if ctx.json {
        #[derive(Serialize)]
        struct Body<'a> {
            results: &'a [Row],
            summary: &'a Summary,
        }
        output::print_json("sync", Body { results: &rows, summary: &summary });
        return Ok(exit);
    }

    let mut table = Table::new(["REPO", "REMOTE", "RESULT", "SYNC"]);
    let mut previous = None;
    for row in &rows {
        let first = previous != Some(row.repo_index);
        previous = Some(row.repo_index);
        let result = match row.result {
            Outcome::Updated => format!("updated {}", row.updated.join(", ")),
            Outcome::Unchanged => "no changes".into(),
            Outcome::Failed => "failed".into(),
            Outcome::Skipped => "skipped: production".into(),
            Outcome::NoRemotes => "no remotes".into(),
        };
        let repo = &setup.repos[row.repo_index];
        let (name, sync) = if first { (row.repo.clone(), sync_after(repo)) } else { (String::new(), String::new()) };
        table.row([name, row.remote.clone().unwrap_or_default(), result, sync]);
    }
    print!("{}", table.render(""));
    println!();
    println!("{}", summary_line(ctx, &summary));
    for row in rows.iter().filter(|r| r.result == Outcome::Failed) {
        let repo = &setup.repos[row.repo_index];
        let remote = row.remote.as_deref().unwrap_or_default();
        let error = Error::new(format!(
            "fetching {} ({remote}) failed: {}",
            row.repo,
            row.error.as_deref().unwrap_or("no message")
        ))
        .try_(format!("git -C {} fetch {remote}", ctx.tilde(&repo.path)));
        eprintln!("{error}");
    }
    Ok(exit)
}

fn fetch(repo: &Repo, i: usize, remote: &str) -> Result<Row> {
    let fetched = git::fetch(&repo.path, remote)?;
    let (result, updated, error) = match fetched {
        Ok(f) if f.updated.is_empty() => (Outcome::Unchanged, f.updated, None),
        Ok(f) => (Outcome::Updated, f.updated, None),
        Err(message) => (Outcome::Failed, Vec::new(), Some(message)),
    };
    Ok(Row {
        repo: repo.name.clone(),
        remote: Some(remote.to_string()),
        result,
        updated,
        error,
        observed_at: Time(SystemTime::now()),
        repo_index: i,
    })
}

/// The main checkout's SYNC state, read again after the fetch.
fn sync_after(repo: &Repo) -> String {
    let upstream = git::status(&repo.path).ok().and_then(Result::ok).map(|s| s.upstream);
    sync_state(repo.remotes.is_empty(), upstream.as_ref())
}

/// Save each fetched repository's outcome for `grove status`.
fn record(ctx: &Context, repos: &[Repo], rows: &[Row]) -> Result<()> {
    let mut syncs: Vec<(usize, Vec<RemoteOutcome>)> = Vec::new();
    for row in rows {
        let Some(remote) = &row.remote else { continue };
        let outcome = RemoteOutcome {
            remote: remote.clone(),
            result: if row.result == Outcome::Failed { "failed" } else { "ok" }.into(),
            error: row.error.clone(),
        };
        match syncs.last_mut() {
            Some((i, outcomes)) if *i == row.repo_index => outcomes.push(outcome),
            _ => syncs.push((row.repo_index, vec![outcome])),
        }
    }
    if syncs.is_empty() {
        return Ok(());
    }
    let syncs = syncs.into_iter().map(|(i, o)| (repos[i].common_dir.as_path(), LastSync::new(ctx.now, o))).collect();
    State::record(&ctx.state_dir(), syncs)
}

fn summarize(which: &[usize], rows: &[Row]) -> Summary {
    let mut summary = Summary { repos: which.len(), ..Summary::default() };
    for &i in which {
        let mine: Vec<&Row> = rows.iter().filter(|r| r.repo_index == i).collect();
        match mine.first().map(|r| r.result) {
            Some(Outcome::Skipped) => summary.skipped += 1,
            Some(Outcome::NoRemotes) => summary.local_only += 1,
            _ if mine.iter().any(|r| r.result == Outcome::Failed) => summary.failed += 1,
            _ => summary.fetched += 1,
        }
    }
    summary
}

/// `fetched 2 of 5 repos: 1 failed, 1 skipped (production on testhost), 1 local only. Nothing was merged ...`
fn summary_line(ctx: &Context, s: &Summary) -> String {
    let mut parts = Vec::new();
    if s.failed > 0 {
        parts.push(format!("{} failed", s.failed));
    }
    if s.skipped > 0 {
        parts.push(format!("{} skipped (production on {})", s.skipped, ctx.host));
    }
    if s.local_only > 0 {
        parts.push(format!("{} local only", s.local_only));
    }
    let detail = if parts.is_empty() { String::new() } else { format!(": {}", parts.join(", ")) };
    format!("fetched {} of {}{detail}. Nothing was merged or checked out.", s.fetched, plural(s.repos, "repo"))
}
