//! `grove rollout` and `grove here` beyond the transcripts: a new release, reports that fail their checks, a host
//! grove.toml does not describe, and the read-only guarantee over the estate.

mod common;

use std::path::Path;

use common::{World, run};

/// A new signed release of tool, which the estate's `~/.workstation` does not pin yet.
fn release_tool_v0_3(world: &World) {
    let tool = world.src("tool");
    let key = world.root.join("keys/release");
    world.git(&tool, &["commit", "-q", "--allow-empty", "-m", "tool v0.3.0"]);
    world.git(
        &tool,
        &[
            "-c",
            "gpg.format=ssh",
            "-c",
            &format!("user.signingkey={}", key.display()),
            "tag",
            "-s",
            "-m",
            "tool v0.3.0",
            "v0.3.0",
        ],
    );
}

fn consumers<'a>(json: &'a serde_json::Value, provider: &str) -> &'a Vec<serde_json::Value> {
    json["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == provider)
        .unwrap()["consumers"]
        .as_array()
        .unwrap()
}

#[test]
fn a_new_release_puts_every_pin_behind() {
    let world = World::estate();
    release_tool_v0_3(&world);

    let out = run(world.grove(&["rollout", "tool", "--json"]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    let json = out.json();
    let rows = consumers(&json, "tool");
    let states: Vec<_> = rows
        .iter()
        .map(|r| (r["consumer"].as_str().unwrap(), r["state"].as_str().unwrap()))
        .collect();
    assert_eq!(
        states,
        [
            (".workstation", "behind"),
            ("host mac", "behind"),
            ("host testhost", "behind")
        ]
    );
    let ws = world.home.join(".workstation").display().to_string();
    assert_eq!(
        rows[0]["next"],
        format!(
            "set inputs.tool.url to ref=refs/tags/v0.3.0 in {ws}/flake.nix, then: nix flake update tool --flake {ws}"
        )
    );
    assert_eq!(
        rows[1]["next"],
        "sudo darwin-rebuild switch --flake ~/.workstation"
    );
    assert_eq!(rows[1]["where"], "mac");

    let out = run(world.grove(&["here"]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.stdout.starts_with("testhost owes 2 steps\n"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("  bump tool in ~/.workstation: v0.2.0 to v0.3.0\n"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains(".workstation  tool      v0.1.0     v0.2.0  v0.3.0"),
        "{}",
        out.stdout
    );
    // bump comes before switch: switching first would activate the old pin.
    let bump = out.stdout.find("  bump").unwrap();
    let switch = out.stdout.find("  switch").unwrap();
    assert!(bump < switch, "{}", out.stdout);
}

/// Rewrite a report's JSON (its signature stays as it was).
fn tamper(world: &World, host: &str, from: &str, to: &str) {
    let file = world.home.join(format!(".local/state/grove/hosts/{host}.json"));
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains(from));
    std::fs::write(&file, text.replace(from, to)).unwrap();
}

fn host_unknown(world: &World, host: &str) -> (i32, String) {
    let out = run(world.grove(&["rollout", "--json"]));
    let json = out.json();
    let reason = json["hosts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["host"] == host)
        .unwrap()["unknown"]
        .as_str()
        .unwrap_or("")
        .to_string();
    (out.code, reason)
}

#[test]
fn a_report_must_be_signed_by_its_host() {
    let world = World::estate();
    tamper(&world, "mac", "\"tag\": \"v0.2.0\"", "\"tag\": \"v0.1.0\"");
    let (code, reason) = host_unknown(&world, "mac");
    assert_eq!(code, 0);
    assert_eq!(
        reason,
        "report not verified: the signature does not match grove/allowed_signers for mac"
    );

    // Signed by mac's key, but naming another host: not mac's report.
    tamper(&world, "mac", "\"host\": \"mac\"", "\"host\": \"other\"");
    let hosts = world.home.join(".local/state/grove/hosts");
    std::fs::remove_file(hosts.join("mac.json.sig")).unwrap();
    let status = std::process::Command::new(common::which("ssh-keygen"))
        .args(["-q", "-Y", "sign", "-n", "grove-report", "-f"])
        .arg(world.root.join("keys/mac"))
        .arg(hosts.join("mac.json"))
        .status()
        .unwrap();
    assert!(status.success());
    let (_, reason) = host_unknown(&world, "mac");
    assert_eq!(reason, "report not verified: it names host other, not mac");

    // A report signed by another world's mac key does not match this world's signers.
    let world2 = World::estate();
    let hosts2 = world2.home.join(".local/state/grove/hosts");
    for ext in ["json", "json.sig"] {
        std::fs::copy(
            hosts2.join(format!("mac.{ext}")),
            hosts.join(format!("mac.{ext}")),
        )
        .unwrap();
    }
    let (_, reason) = host_unknown(&world, "mac");
    assert_eq!(
        reason, "report not verified: the signature does not match grove/allowed_signers for mac",
        "world2's key is not world's"
    );

    std::fs::remove_file(hosts.join("mac.json.sig")).unwrap();
    let (_, reason) = host_unknown(&world, "mac");
    assert!(
        reason.starts_with("report not verified: no signature"),
        "{reason}"
    );
}

#[test]
fn an_unreadable_report_is_partial() {
    let world = World::estate();
    std::fs::write(
        world.home.join(".local/state/grove/hosts/broken.json"),
        "not json",
    )
    .unwrap();
    let (code, reason) = host_unknown(&world, "broken");
    assert_eq!(code, 3);
    assert!(reason.starts_with("report unreadable: "), "{reason}");
}

#[test]
fn here_needs_the_hosts_after_table() {
    let world = World::estate();
    let mut cmd = world.grove(&["here"]);
    cmd.env("GROVE_HOST", "elsewhere");
    let out = run(cmd);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "grove: elsewhere: grove.toml has no hosts.elsewhere.after, so grove does not know which repositories \
         configure this host\n  try: grove config check\n"
    );

    // Without the activated file, the host's pins are unknown, and nothing is owed on a guess.
    std::fs::remove_file(world.home.join(".config/grove/activated/.workstation")).unwrap();
    let out = run(world.grove(&["here"]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.stdout.starts_with("testhost owes nothing\n"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.contains(
            "? .workstation: activated commit unknown: ~/.config/grove/activated/.workstation is missing"
        ),
        "{}",
        out.stdout
    );
    let (_, reason) = host_unknown(&world, "testhost");
    assert!(reason.contains("is missing"), "{reason}");
    let out = run(world.grove(&["rollout"]));
    assert!(
        out.stdout.contains(
            "? host testhost: ~/.config/grove/activated/.workstation is missing: the host's config writes it when \
             activated, now\n"
        ),
        "{}",
        out.stdout
    );
}

/// HEAD, refs, index and files of every checkout in the estate, before and after.
fn fingerprints(world: &World) -> Vec<String> {
    let checkouts: Vec<_> = ["kit", "tool", "app", "site"]
        .iter()
        .map(|c| world.src(c))
        .chain([world.home.join(".workstation")])
        .collect();
    checkouts
        .iter()
        .map(|c: &std::path::PathBuf| {
            let p: &Path = c;
            let mut print = world.git(p, &["rev-parse", "HEAD"]);
            print += &world.git(p, &["for-each-ref"]);
            print += &world.git(
                p,
                &["status", "--porcelain=v2", "--branch", "--untracked-files=all"],
            );
            print
        })
        .collect()
}

#[test]
fn rollout_and_here_change_nothing() {
    let world = World::estate();
    let before = fingerprints(&world);
    for args in [
        &["rollout"][..],
        &["rollout", "--json"],
        &["rollout", "kit"],
        &["here"],
        &["here", "--json"],
        &["here", "--report"],
    ] {
        let out = run(world.grove(args));
        assert!([0, 3].contains(&out.code), "grove {args:?}: {}", out.stderr);
    }
    assert_eq!(before, fingerprints(&world));
    let leftovers: Vec<_> = std::fs::read_dir(world.home.join(".local/state/grove"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|n| n.to_string_lossy().starts_with(".allowed_signers"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}
