---
status: current
sources:
  - tests/fixtures/world.sh (the world itself; run it and look)
---

# The fixture world

Every transcript and integration test runs grove against a throwaway world built by
[`world.sh`](world.sh), never against a real home directory. It is built fresh for each test run with
fixed authors and dates, so commit IDs never change (the config commit is `448ebef`).

| checkout | state grove should report |
|---|---|
| `~/src/alpha` | `main`, clean, even with `origin/main`. Its remote has one more commit, which `grove sync` brings in (then `↓1`). Tag `@tools`. |
| `~/src/beta` | `main`, dirty (1 modified, 1 untracked), `↑2`. Tag `@tools`. |
| `~/src/delta` | `main`, clean, local only (no remotes). |
| `~/src/epsilon` | `main`, clean, has `origin` but no upstream and was never fetched. Its remote does not exist, so `grove sync` fails for it (exit 3). |
| `~/src/gamma` | `main`, clean, `↓1`. Tag `@infra`. **Production on host `testhost`**: observed only, never fetched. |
| `~/src/worktrees/gamma/feature` | gamma's worktree, branch `feature`, clean, no upstream. It is merged under gamma, which puts gamma in `@unpushed`. |
| `~/src/notes` | not a checkout: skipped silently. |
| `~/src/scratch` | a checkout matched by `ignore`: not shown. |
| `~/.workstation` | `grove/grove.toml` committed at `448ebef`, plus an uncommitted edit grove must not read. |

Remotes are bare repositories under `remotes/`. Checkouts name them by forge URL
(`https://git.example.org/daniel/NAME.git`), and a repo-local `url.<remotes>/.insteadOf` maps that URL to the
bare repository. So `grove open` sees a forge URL, and `grove sync` fetches offline.

## How the tests run it

`tests/transcripts.rs` does this:

1. Build one world in a temporary directory for the read-only transcripts, and a second one for
   `sync.md`, which fetches and changes remote-tracking refs.
2. Run grove with this environment: `HOME=<world>/home`, `GROVE_HOST=testhost`, `PATH` holding only a
   symlink to `git` (so `tuteur`, `graft`, `pot` and `xdg-open` are absent), `BROWSER=`, `NO_COLOR=1`, `TZ=UTC`,
   `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, and `XDG_STATE_HOME` inside the world. trycmd
   inherits the test's environment otherwise, so every variable that could reach outside the world is set.
3. Register the `[HOME]` substitution and run `trycmd` over `docs/reference/cli/*.md`. Each `NAME.in/`
   directory next to a transcript (for example `config-check.in/`) is copied into a sandbox and becomes the
   working directory.

The integration tests (`tests/*.rs`, helpers in `tests/common/`) build the same world and run grove with
`env_clear()` and the same environment. `tests/contracts.rs` adds stub `pot`, `tuteur` and `graft` scripts to `PATH` that
print the golden `runs/1` and `status/1` fixtures, so the `pins` and `runs` columns stop being `?`.
