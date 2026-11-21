# grove: agent instructions

grove keeps a person's git checkouts in view. It is **read only**, and it is the first Rust code of a tool
family (grove, pot, tuteur). The conventions below are the family's shared dev kit. Follow them here, and copy
them when you start the next tool.

Orientation: [README.md](README.md). Design: [docs/concepts/](docs/concepts/). CLI behavior is specified by the
transcripts in [docs/reference/cli/](docs/reference/cli/), which are also the tests.

## Hard rules

- **Never change a repository.** No command may change HEAD, a branch, the index or the working tree of any
  repository grove observes. `grove sync` runs `git fetch` and nothing else. Never merge, pull, checkout, reset,
  stash, commit or push on a user's repository, not even "to help".
- **Reads must not write either.** Run `git status` and friends with `git --no-optional-locks`, because plain
  `git status` refreshes the index and takes `index.lock`. Run fetch with `GIT_TERMINAL_PROMPT=0` and
  `-c gc.auto=0 -c maintenance.auto=false`.
- **Production repositories are observed only.** A repository listed in `hosts.<this host>.production` is never
  fetched. grove shows it and nothing more.
- **No secrets.** grove holds no keys, tokens or push rights. `grove config check` rejects values that look like
  secrets and never prints the value it rejected. Do not add a feature that needs a credential.
- **Config comes from the committed tree**: `git -C ~/.workstation show HEAD:grove/grove.toml`, never the working
  tree. `grove config check --file PATH` is the only way to read a file that is not committed, and it says so.
- **Tests never touch the real `~/src` or `~/.workstation`.** Every test builds a throwaway world with
  [`tests/fixtures/world.sh`](tests/fixtures/world.sh) and points `HOME` at it.
- **No `unsafe`** (`unsafe_code = "forbid"` in `Cargo.toml`). **No async runtime.** Use `std::thread::scope` to
  run git calls in parallel.
- Exit code **125 belongs to pot**. Never return it.

## Family CLI conventions

1. **Human output is the default and is designed as UX.** Aligned columns, `~` for the home directory, a summary
   line, and nothing a person has to decode. No color unless stdout is a terminal and `NO_COLOR` is unset.
2. **`--json` on every command**, including errors. Every JSON document starts with `"schema": "grove/1"` (the
   tool name and a major version) and `"command"`. Adding a field is compatible. Renaming or removing one needs
   `grove/2`.
3. **Every fact carries `source` and `observed_at`.** A fact grove could not read is `unknown`, with a reason and
   an age. It is still a result, not an error.
4. **Errors say what failed and the exact command that helps**:

   ```text
   grove: <what failed, naming the thing>
     fix: <a command that fixes it>      (when one exists)
     try: <a command that shows more>    (otherwise)
   ```

5. **Stable exit codes**:

   | code | meaning |
   |------|---------|
   | 0    | ok |
   | 1    | error: nothing useful was produced (including bad usage; clap's 2 is mapped to 1) |
   | 3    | partial: the output is valid, but some sources were unreadable or some remotes failed |
   | 10   | needs you: a person has to decide (reserved; no G0 command returns it yet) |
   | 125  | pot's. Never used. |

6. Addresses are a short name, `owner/name`, a path, `.`, or `@group`. There is no query language.

## Dev kit

- `nix develop` gives cargo, rustc, clippy, rustfmt and cargo-deny. `nix flake check` is the gate: it runs the
  build, `cargo clippy -- --deny warnings` (pedantic), `cargo fmt --check`, `cargo deny check bans licenses
  sources`, and `cargo test`.
- Advisories need the network: run `cargo deny check advisories` in the dev shell.
- Keep dependencies few. Before adding a crate, check whether std or an existing dependency covers it, and say why
  in the commit message. Current runtime dependencies: clap, serde, serde_json, toml, glob, humantime.
- Version: `0.1.0`. Stay pre-1.0 until the `grove/1` output and the contracts grove reads are stable.

## Tests: behavior first

- **Transcripts** are Markdown files under `docs/reference/cli/`. Each `console` block is run by
  [trycmd](https://docs.rs/trycmd): `$ command`, an optional `? <exit code>`, then the expected stdout and stderr.
  `[..]` matches the rest of a line and `...` matches any number of lines. `[HOME]` is the fixture home. They are
  the reference docs, so write them for a reader first.
- Update expected output on purpose with `TRYCMD=overwrite cargo test`, then review the diff like code.
- **Integration tests** (`tests/*.rs`) cover what a transcript cannot show: `.` and path addressing from inside a
  checkout, worktrees outside the scan globs, ambiguous names, concurrency, and the guarantee that no repository's
  HEAD, index or working tree changes (hash them before and after).
- Test behavior through the binary. Do not test private functions unless the logic is hard to reach from the CLI
  (for example, secret patterns and forge URL parsing).
- Write the test first and watch it fail.

## Contracts grove reads

- `runs/1` from `pot runs --json` and `status/1` from `graft status --json`. Their JSON Schemas and golden
  fixtures live in graft's repository. grove treats them as data: it parses them leniently, ignores unknown
  fields, and turns a missing or unreadable source into `unknown` facts.
- The readers live in `src/sources/` behind one boundary (`Sources`). Nothing else in grove knows the contracts'
  shapes. Tests drive them with the golden fixtures through stub `pot` and `graft` scripts on `PATH`.
- Status (2026-11-21): the schemas and golden fixtures are copied into [tests/contracts/](tests/contracts/)
  from graft's `contracts` branch (commit noted there). Copy them again when a contract changes.

## Docs

The docs follow knowledge-base-kit conventions. `README.md` is for orientation. `docs/concepts/` explains why,
`docs/guides/` explains how, and `docs/reference/` is exact. Every page starts with frontmatter that has a
`status` (`draft`, `current` or `superseded`) and a `sources` list. Claims about decisions cite where they were
made.

## Working here

- Commit in small steps that each build and pass. Do not push or publish.
- If pot denies something (a package, a path, a host), stop and report `pot why <item>`. Do not work around it.
