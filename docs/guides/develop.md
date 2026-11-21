---
status: current
sources:
  - stack decisions, Daniel (2026-11-20)
  - trycmd documentation, https://docs.rs/trycmd
---

# Develop and test grove

```sh
nix develop          # cargo, rustc, clippy, rustfmt, cargo-deny
cargo test           # transcripts and integration tests
nix flake check      # the gate: build, clippy (pedantic, deny warnings), fmt, cargo-deny, tests
```

## Behavior first

The CLI is specified by **Markdown transcripts** in [`docs/reference/cli/`](../reference/cli/): a command, its
exit code, and its exact output. They are the reference docs and the tests at once (stack decisions,
2026-11-20). To change behavior:

1. Edit the transcript to show the behavior you want, and run `cargo test` to watch it fail.
2. Write the code.
3. If the output changed as intended, `TRYCMD=overwrite cargo test` rewrites the transcripts. Review that diff
   as carefully as code.

Transcript syntax (trycmd): each ` ```console ` block holds `$ command` lines, an optional `? CODE` on the
line after the command, then the expected stdout and stderr. `[..]` matches the rest of a line, a line of `...`
matches any number of lines, and `[HOME]` is the fixture home. Other code blocks are prose and are not run.

Setups too complex for a transcript are **integration tests** in `tests/*.rs`, against the same
[fixture world](../../tests/fixtures/README.md) or a variant of it. Examples: running from inside a checkout,
ambiguous names, worktrees outside the scan, and the guarantee that no repository changes.

## Why trycmd, not scrut

Both run Markdown transcripts. trycmd fits better:

- **It runs inside `cargo test`.** `nix flake check` covers the transcripts through crane's `cargoTest`, with no
  second test runner and no extra binary to package. Cargo.lock pins it, and cargo-deny checks it like any other
  dependency.
- **Transcripts stay readable as user docs.** trycmd runs the command itself, without a shell, so a transcript
  holds only what a user would type. scrut runs each block in a shell, which invites inline setup (`git init`,
  `cd`, pipes) that would make the reference pages read like test scripts.
- **The cost is accepted.** With no shell there are no pipes, no `cd`, and no per-command environment. Setup
  comes from the harness (the fixture world), and anything that needs more is an integration test, which is the
  split the stack decisions ask for anyway.
- Its matching (`[..]`, `...`, named substitutions) and its `TRYCMD=overwrite` workflow cover what grove's
  output needs: ages, timestamps and temporary paths.

Before you switch runners, reconsider whether scrut's regex and glob line matching would be worth a second runner.
