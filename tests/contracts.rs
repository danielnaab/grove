//! The contracts grove reads, driven by their golden fixtures (`tests/contracts/`) through stub `graft` and `pot`
//! scripts on `PATH`.

mod common;

use std::path::Path;

use common::{World, run};

fn fixture(path: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/contracts")
        .join(path)
        .display()
        .to_string()
}

/// delta gets a `.gitmodules`, so grove asks graft about it. graft prints `status/1`'s `clean.json`.
/// pot prints `runs/1`'s `list.json`, with the review run's workspace moved to alpha.
fn with_stubs(world: &World, graft: &str, pot: &str) {
    let delta = world.src("delta");
    std::fs::write(delta.join(".gitmodules"), "").unwrap();
    world.git(&delta, &["add", ".gitmodules"]);
    world.git(&delta, &["commit", "-q", "-m", "scions"]);
    world.stub("graft", graft);
    world.stub("pot", pot);
}

/// A stub that prints `text` (written next to the world, so the stub needs only `cat`).
fn printing(world: &World, name: &str, text: &str) -> String {
    let file = world.root.join(format!("{name}.out"));
    std::fs::write(&file, text).unwrap();
    format!(
        "#!/bin/sh\nexec {} {}\n",
        common::which("cat").display(),
        file.display()
    )
}

/// `pot runs --json` as the golden `list.json`, with the review run's workspace moved to alpha.
fn pot_list(world: &World) -> String {
    let list = std::fs::read_to_string(fixture("runs-1/list.json")).unwrap();
    let list = list.replace(
        "/home/daniel/src/flexion-assistant",
        &world.src("alpha").display().to_string(),
    );
    printing(world, "pot", &list)
}

#[test]
fn pins_and_runs_from_the_golden_fixtures() {
    let world = World::build();
    let graft = printing(
        &world,
        "graft",
        &std::fs::read_to_string(fixture("status-1/clean.json")).unwrap(),
    );
    with_stubs(&world, &graft, &pot_list(&world));

    let out = run(world.grove(&["ls", "--json"]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    let json = out.json();
    let repo = |name: &str| {
        json["repos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap()
            .clone()
    };
    assert_eq!(
        repo("delta")["checkouts"][0]["pins"]["value"],
        serde_json::json!({"scions": 2, "outdated": 1, "problems": 1})
    );
    assert_eq!(repo("delta")["checkouts"][0]["pins"]["source"], "graft");
    assert_eq!(repo("beta")["checkouts"][0]["pins"]["value"]["scions"], 0);
    assert_eq!(
        repo("alpha")["checkouts"][0]["runs"]["value"],
        serde_json::json!({"running": 0, "review": 1, "stopped": 0})
    );
    assert_eq!(json["sources"]["graft"], serde_json::json!({"state": "ok"}));
    assert_eq!(json["sources"]["pot"], serde_json::json!({"state": "ok"}));

    let out = run(world.grove(&["ls", "@tools"]));
    assert!(
        out.stdout.contains("alpha  main    clean  even  -     1 review"),
        "{}",
        out.stdout
    );
    assert!(!out.stdout.contains("? pins"), "{}", out.stdout);
    let out = run(world.grove(&["ls", "delta"]));
    assert!(out.stdout.contains("2, 1 outdated, 1 problem"), "{}", out.stdout);
}

#[test]
fn a_failing_tool_is_partial() {
    let world = World::build();
    let pot = printing(&world, "pot", r#"{"version": 2, "runs": []}"#);
    with_stubs(&world, "#!/bin/sh\necho 'graft: broken' >&2\nexit 1\n", &pot);

    let out = run(world.grove(&["ls"]));
    assert_eq!(out.code, 3);
    assert!(
        out.stdout
            .contains("? pins: graft status in ~/src/delta failed (exit status: 1): graft: broken"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("? runs: pot runs --json: runs/2 is newer than grove reads (runs/1)"),
        "{}",
        out.stdout
    );
    let json = run(world.grove(&["ls", "--json"])).json();
    assert_eq!(json["sources"]["graft"]["state"], "failed");
    assert_eq!(json["sources"]["pot"]["state"], "failed");
}
