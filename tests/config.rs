//! Config cases a transcript cannot show: no committed grove.toml, and a committed one with problems.

mod common;

use common::{World, run};

#[test]
fn without_a_committed_grove_toml_grove_uses_the_defaults() {
    let world = World::build();
    std::fs::remove_dir_all(world.home.join(".workstation")).unwrap();

    let out = run(world.grove(&["config", "check"]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.stdout,
        "ok: no ~/.workstation/grove/grove.toml at HEAD, so grove uses the defaults\n  \
         discover.scan  ~/src/*, ~/src/worktrees/*/*  default\n"
    );
    // Nothing is ignored now, and nothing is production.
    let out = run(world.grove(&["ls", "--path"]));
    assert_eq!(out.stdout.lines().count(), 6, "{}", out.stdout);
}

#[test]
fn uncommitted_edits_are_not_read() {
    let world = World::build();
    let file = world.home.join(".workstation/grove/grove.toml");
    std::fs::write(&file, "version = 1\n[tags]\ntools = [\"delta\"]\n").unwrap();
    let out = run(world.grove(&["ls", "--path", "@tools"]));
    assert!(out.stdout.contains("/src/alpha"), "{}", out.stdout);
}

#[test]
fn a_committed_problem_stops_every_command() {
    let world = World::build();
    let ws = world.home.join(".workstation");
    std::fs::write(
        ws.join("grove/grove.toml"),
        "version = 1\n[tags]\nbehind = [\"alpha\"]\n",
    )
    .unwrap();
    world.git(&ws, &["commit", "-q", "-am", "bad"]);
    let out = run(world.grove(&["ls"]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        format!(
            "grove: grove.toml:3: tags.behind: @behind is a derived group\n  fix: $EDITOR {}/grove/grove.toml\n",
            ws.display()
        )
    );
    let json = run(world.grove(&["ls", "--json"])).json();
    assert_eq!(
        json["error"]["message"],
        "grove.toml:3: tags.behind: @behind is a derived group"
    );
}
