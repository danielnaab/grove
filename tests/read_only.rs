//! The guarantee: no command changes HEAD, a branch, the index or the working tree of a repository it observes.
//! `grove sync` moves remote-tracking refs and `FETCH_HEAD` only, and never touches a production repository.

mod common;

use std::fmt::Write;
use std::path::Path;

use common::{World, run};

/// Everything grove must not change in a checkout: HEAD, local branches and tags, the index's bytes, and every
/// working-tree file.
fn fingerprint(world: &World, checkout: &Path) -> String {
    let mut print = String::new();
    print += &world.git(checkout, &["rev-parse", "HEAD"]);
    print += &world.git(checkout, &["symbolic-ref", "-q", "HEAD"]);
    print += &world.git(checkout, &["for-each-ref", "refs/heads", "refs/tags"]);
    let index = world.git(
        checkout,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
    );
    writeln!(print, "{:?}", std::fs::read(index.trim()).unwrap()).unwrap();
    let mut files = vec![checkout.to_path_buf()];
    while let Some(path) = files.pop() {
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            let mut entries: Vec<_> = std::fs::read_dir(&path)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            entries.sort();
            files.extend(entries);
        } else {
            writeln!(print, "{} {:?}", path.display(), std::fs::read(&path).unwrap()).unwrap();
        }
    }
    print
}

fn checkouts(world: &World) -> Vec<std::path::PathBuf> {
    [
        "alpha",
        "beta",
        "delta",
        "epsilon",
        "gamma",
        "worktrees/gamma/feature",
        "scratch",
    ]
    .iter()
    .map(|c| world.src(c))
    .chain([world.home.join(".workstation")])
    .collect()
}

#[test]
fn no_command_changes_a_repository() {
    let world = World::build();
    let before: Vec<String> = checkouts(&world).iter().map(|c| fingerprint(&world, c)).collect();
    let gamma_refs = world.git(&world.src("gamma"), &["for-each-ref"]);

    for args in [
        &["ls"][..],
        &["ls", "--json"],
        &["status", "beta"],
        &["status", "gamma", "--json"],
        &["sync"],
        &["sync", "--json"],
        &["open", "alpha", "--print"],
        &["config", "check"],
    ] {
        let out = run(world.grove(args));
        assert!([0, 3].contains(&out.code), "grove {args:?}: {}", out.stderr);
    }

    let after: Vec<String> = checkouts(&world).iter().map(|c| fingerprint(&world, c)).collect();
    for ((checkout, before), after) in checkouts(&world).iter().zip(before).zip(after) {
        assert!(before == after, "{} changed", checkout.display());
    }
    // gamma is production on testhost: not even its remote-tracking refs move.
    assert_eq!(world.git(&world.src("gamma"), &["for-each-ref"]), gamma_refs);
}

#[test]
fn sync_moves_only_remote_tracking_refs() {
    let world = World::build();
    let alpha = world.src("alpha");
    let before = world.git(&alpha, &["rev-parse", "main", "origin/main"]);
    let out = run(world.grove(&["sync", "alpha", "--json"]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.json()["results"][0]["updated"][0], "origin/main");
    let after = world.git(&alpha, &["rev-parse", "main", "origin/main"]);
    assert_eq!(before.lines().next(), after.lines().next(), "main must not move");
    assert_ne!(before.lines().nth(1), after.lines().nth(1), "origin/main moves");
}

#[test]
fn sync_runs_remotes_in_parallel_and_records_outcomes() {
    let world = World::build();
    let out = run(world.grove(&["sync", "--json"]));
    assert_eq!(out.code, 3);
    let json = out.json();
    assert_eq!(json["summary"]["failed"], 1);
    let state = world.home.join(".local/state/grove/sync.json");
    let state: serde_json::Value = serde_json::from_slice(&std::fs::read(state).unwrap()).unwrap();
    let epsilon = world.src("epsilon/.git").display().to_string();
    assert_eq!(state["repos"][&epsilon]["result"], "failed");

    let status = run(world.grove(&["status", "epsilon", "--json"])).json();
    assert_eq!(status["repo"]["last_sync"]["value"]["result"], "failed");
    assert_eq!(status["repo"]["last_sync"]["source"], "grove state");
}
