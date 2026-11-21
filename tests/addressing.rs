//! Addressing that a transcript cannot show: `.` and paths from inside checkouts, worktrees outside the scan
//! globs, and ambiguous names.

mod common;

use common::{World, run};

#[test]
fn dot_is_the_repository_you_are_in_from_any_worktree_or_subdirectory() {
    let world = World::build();
    let feature = world.src("worktrees/gamma/feature");
    let mut cmd = world.grove(&["status", "--json"]);
    cmd.current_dir(&feature);
    let out = run(cmd);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.json()["repo"]["name"], "gamma");

    std::fs::create_dir(world.src("beta/sub")).unwrap();
    let mut cmd = world.grove(&["ls", "--path", "."]);
    cmd.current_dir(world.src("beta/sub"));
    assert_eq!(run(cmd).stdout.trim(), world.src("beta").display().to_string());
}

#[test]
fn paths_name_repositories() {
    let world = World::build();
    let out = run(world.grove(&["ls", "--path", "~/src/worktrees/gamma/feature"]));
    assert_eq!(out.stdout.trim(), world.src("gamma").display().to_string());

    let out = run(world.grove(&["status", "~/src/notes"]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "grove: ~/src/notes is not in a repository grove knows\n  try: grove config check (see discover.scan)\n"
    );
}

#[test]
fn worktrees_outside_the_scan_globs_are_listed_with_their_repository() {
    let world = World::build();
    let elsewhere = world.home.join("elsewhere/alpha-review");
    world.git(
        &world.src("alpha"),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "review",
            elsewhere.to_str().unwrap(),
        ],
    );

    let out = run(world.grove(&["ls", "alpha"]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("└ alpha-review  review"), "{}", out.stdout);

    let mut cmd = world.grove(&["status", "--json"]);
    cmd.current_dir(&elsewhere);
    let json = run(cmd).json();
    assert_eq!(json["repo"]["name"], "alpha");
    assert_eq!(json["repo"]["checkouts"][1]["worktree"], true);
}

#[test]
fn an_ambiguous_name_lists_the_choices() {
    let world = World::build();
    let other = world.src("worktrees/other/alpha");
    std::fs::create_dir_all(&other).unwrap();
    world.git(&other, &["init", "-q"]);

    let out = run(world.grove(&["status", "alpha"]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "grove: 2 repositories are named alpha: ~/src/alpha, ~/src/worktrees/other/alpha\n  try: grove status ~/src/alpha\n"
    );
    // A path still names one.
    let out = run(world.grove(&["status", "~/src/worktrees/other/alpha", "--json"]));
    assert_eq!(out.json()["repo"]["path"], other.display().to_string());
}

#[test]
fn usage_errors_exit_1_and_are_json_under_json() {
    let world = World::build();
    let out = run(world.grove(&["ls", "--bogus", "--json"]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.json()["error"]["message"],
        "unexpected argument '--bogus' found"
    );
}
