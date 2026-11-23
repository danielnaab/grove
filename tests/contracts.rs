//! The contracts grove reads, driven by their golden fixtures (`tests/contracts/`) through stub `tuteur`, `graft`
//! and `pot` scripts on `PATH`.

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

/// delta gets a `.gitmodules`, so grove asks the pins tool (`tuteur`, or `graft`) about it. pot prints
/// `runs/1`'s `list.json`, with the review run's workspace moved to alpha.
fn with_stubs(world: &World, pins: &[(&str, &str)], pot: &str) {
    let delta = world.src("delta");
    std::fs::write(delta.join(".gitmodules"), "").unwrap();
    world.git(&delta, &["add", ".gitmodules"]);
    world.git(&delta, &["commit", "-q", "-m", "patterns"]);
    for (name, script) in pins {
        world.stub(name, script);
    }
    world.stub("pot", pot);
}

/// A stub that prints `status/1`'s `clean.json`.
fn clean(world: &World, name: &str) -> String {
    printing(
        world,
        name,
        &std::fs::read_to_string(fixture("status-1/clean.json")).unwrap(),
    )
}

fn delta_pins(json: &serde_json::Value) -> serde_json::Value {
    json["repos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "delta")
        .unwrap()["checkouts"][0]["pins"]
        .clone()
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
    let tuteur = clean(&world, "tuteur");
    with_stubs(&world, &[("tuteur", &tuteur)], &pot_list(&world));

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
    assert_eq!(repo("delta")["checkouts"][0]["pins"]["source"], "tuteur");
    assert_eq!(repo("beta")["checkouts"][0]["pins"]["value"]["scions"], 0);
    assert_eq!(
        repo("alpha")["checkouts"][0]["runs"]["value"],
        serde_json::json!({"running": 0, "review": 1, "stopped": 0})
    );
    assert_eq!(json["sources"]["tuteur"], serde_json::json!({"state": "ok"}));
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
    with_stubs(
        &world,
        &[("tuteur", "#!/bin/sh\necho 'tuteur: broken' >&2\nexit 1\n")],
        &pot,
    );

    let out = run(world.grove(&["ls"]));
    assert_eq!(out.code, 3);
    assert!(
        out.stdout
            .contains("? pins: tuteur status in ~/src/delta failed (exit status: 1): tuteur: broken"),
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
    assert_eq!(json["sources"]["tuteur"]["state"], "failed");
    assert_eq!(json["sources"]["pot"]["state"], "failed");
}

/// tuteur reads graft's layout too (`.scions/`, graft's README marker, `graft.nix`), so when both are installed
/// grove asks tuteur about every checkout, whichever tool the checkout pins.
#[test]
fn tuteur_first_when_both_are_installed() {
    let world = World::build();
    let tuteur = clean(&world, "tuteur");
    let graft = "#!/bin/sh\necho 'graft: should not run' >&2\nexit 1\n";
    let pot = printing(&world, "pot", r#"{"version": 1, "runs": []}"#);
    with_stubs(&world, &[("tuteur", &tuteur), ("graft", graft)], &pot);

    let out = run(world.grove(&["ls", "--json"]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    let pins = delta_pins(&out.json());
    assert_eq!(pins["source"], "tuteur");
    assert_eq!(pins["value"]["scions"], 2);
}

/// Without tuteur, grove falls back to graft, until the last checkouts move to tuteur.
#[test]
fn graft_when_tuteur_is_absent() {
    let world = World::build();
    let graft = clean(&world, "graft");
    let pot = printing(&world, "pot", r#"{"version": 1, "runs": []}"#);
    with_stubs(&world, &[("graft", &graft)], &pot);

    let out = run(world.grove(&["ls", "--json"]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    let json = out.json();
    let pins = delta_pins(&json);
    assert_eq!(pins["source"], "graft");
    assert_eq!(pins["value"]["outdated"], 1);
    assert_eq!(json["sources"]["tuteur"], serde_json::json!({"state": "ok"}));
    let out = run(world.grove(&["status", "delta"]));
    assert!(out.stdout.contains("graft, now"), "{}", out.stdout);
}
