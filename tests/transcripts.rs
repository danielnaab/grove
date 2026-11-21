//! The CLI reference in `docs/reference/cli/` is the spec: run every transcript against a fixture world.
//!
//! `sync.md` fetches, which moves remote-tracking refs, so it gets a world of its own. The other transcripts only
//! read and share one. `TRYCMD=overwrite cargo test` rewrites expected output; review that diff like code.

mod common;

use common::World;

fn cases(world: &World) -> trycmd::TestCases {
    let cases = trycmd::TestCases::new();
    cases.register_bin("grove", std::path::Path::new(env!("CARGO_BIN_EXE_grove")));
    for (key, value) in world.env() {
        cases.env(key, value);
    }
    cases
        .insert_var("[HOME]", world.home.display().to_string())
        .expect("[HOME] substitution");
    cases
}

#[test]
fn read_only_transcripts() {
    let world = World::build();
    cases(&world)
        .case("docs/reference/cli/*.md")
        .skip("docs/reference/cli/sync.md")
        .run();
}

#[test]
fn sync_transcript() {
    let world = World::build();
    cases(&world).case("docs/reference/cli/sync.md").run();
}
