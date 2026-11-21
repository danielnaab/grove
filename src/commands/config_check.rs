//! `grove config check [--file PATH]`: validate grove.toml. List every value with the file and line that set it,
//! and note when the working tree differs from HEAD. A problem is an error (exit 1) that names its line.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::{Config, Origin, REPO_PATH};
use crate::context::Context;
use crate::error::Result;
use crate::exit::Exit;
use crate::output::{self, Table};

#[derive(Serialize)]
struct Value<'a> {
    key: &'a str,
    value: &'a serde_json::Value,
    line: usize,
}

#[derive(Serialize)]
struct Body<'a> {
    ok: bool,
    file: Option<PathBuf>,
    commit: Option<&'a str>,
    working_tree_differs: Option<bool>,
    host: &'a str,
    values: Vec<Value<'a>>,
    problems: Vec<String>,
}

pub fn run(ctx: &Context, file: Option<&Path>) -> Result<Exit> {
    let config = match file {
        Some(path) => Config::load_file(path)?,
        None => Config::load(&ctx.home)?,
    };
    let head_file = ctx.home.join(".workstation").join(REPO_PATH);

    if ctx.json {
        let (file, commit, differs) = match &config.origin {
            Origin::Head { commit, working_tree_differs, .. } => {
                (Some(head_file), Some(commit.as_str()), Some(*working_tree_differs))
            }
            Origin::File(path) => (Some(path.canonicalize().unwrap_or_else(|_| path.clone())), None, None),
            Origin::Defaults { .. } => (None, None, None),
        };
        let values =
            config.entries.iter().map(|e| Value { key: &e.key, value: &e.value, line: e.line }).collect();
        let body = Body {
            ok: true,
            file,
            commit,
            working_tree_differs: differs,
            host: &ctx.host,
            values,
            problems: Vec::new(),
        };
        output::print_json("config check", body);
        return Ok(Exit::Ok);
    }

    let head = ctx.tilde(&head_file);
    println!(
        "{}",
        match &config.origin {
            Origin::Head { commit, .. } => format!("ok: {head} at HEAD ({})", &commit[..7.min(commit.len())]),
            Origin::File(_) => format!("ok: {} (a draft: grove reads {head} at HEAD)", config.label),
            Origin::Defaults { .. } => format!("ok: no {head} at HEAD, so grove uses the defaults"),
        }
    );
    let mut table = Table::default();
    for entry in &config.entries {
        table.row([entry.key.clone(), show(&entry.value), config.source(entry.line)]);
    }
    if matches!(config.origin, Origin::Defaults { .. }) {
        table.row(["discover.scan".into(), config.scan.join(", "), "default".into()]);
    }
    print!("{}", table.render("  "));

    let mut notes = Vec::new();
    if let Some(production) = config.production_on(&ctx.host) {
        let verb = if production.addresses.len() == 1 { "is" } else { "are" };
        notes.push(format!(
            "this host is {}: {} {verb} production here (observed only)",
            ctx.host,
            production.addresses.join(", ")
        ));
    }
    if let Origin::Head { repo, working_tree_differs: true, .. } = &config.origin {
        notes.push("note: grove.toml in the working tree differs from HEAD, and grove reads HEAD".into());
        notes.push(format!("  fix: git -C {} commit {REPO_PATH}", ctx.tilde(repo)));
    }
    if !notes.is_empty() {
        println!("\n{}", notes.join("\n"));
    }
    Ok(Exit::Ok)
}

/// A value for people: lists joined with `, `, strings bare.
fn show(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Array(items) => items.iter().map(show).collect::<Vec<_>>().join(", "),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
