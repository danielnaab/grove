---
status: current
sources:
  - grove design record, "Facts" (quoted in the G0 brief, 2026-11-21)
---

# Facts and sources

Everything grove reports is a **fact**: a value, where it came from (`source`), and when it was observed
(`observed_at`) (design record, "Facts").

| source | facts |
|---|---|
| `git` | branch, tree (modified, staged, untracked, conflicted), upstream and ahead/behind, worktrees |
| `git config` | remotes, as written in the config, before `insteadOf` |
| `FETCH_HEAD` | when the repository was last fetched, by anyone |
| `grove.toml:LINE` | tags and production, with the line that set them |
| `grove sync` | the last fetch outcome per remote, from grove's state |
| `tuteur` | pins (`tuteur status --json`, contract `status/1`). `graft` when grove asked graft, because tuteur is not on `PATH` |
| `pot` | runs (`pot runs --json`, contract `runs/1`) |

## Unknown is a result

When grove cannot read a source, the fact is `unknown`, with the reason and the time grove tried. It is shown
as `?`. It is not an error to act on (design record, "Facts"):

- If a tool is **absent** (not on `PATH`), its facts are unknown and the exit code stays 0. A machine without pot
  is not a broken machine.
- If a source **failed** (the tool errored, or its output did not match its contract), its facts are unknown and
  the exit code is 3 (partial). The output is still complete for everything else.

## Ages

A fact read during this invocation shows as `now`. A fact from grove's state, such as the last sync outcome,
shows its real age (`5m ago`). `FETCH_HEAD`'s age is a value in its own right: how long ago the repository was
last fetched.
