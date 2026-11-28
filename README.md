---
status: draft
sources:
  - grove design record, as quoted in the G0 brief (2026-11-21)
  - stack decisions, Daniel (2026-11-20)
---

# grove

grove keeps your git checkouts in view. It lists every checkout under `~/src` with its branch, uncommitted
changes, ahead/behind, tuteur pins and pot runs, and it can fetch them all at once. It is **read only**. It never
changes a branch, index or working tree, and `grove sync` only fetches.

```console
$ grove ls
REPO       BRANCH   TREE   SYNC         PINS  RUNS  FETCHED
alpha      main     clean  even         ?     ?     4s ago
beta       main     dirty  ↑2           ?     ?     4s ago
delta      main     clean  local only   ?     ?     -
epsilon    main     clean  no upstream  ?     ?     never
gamma      main     clean  ↓1           ?     ?     4s ago
└ feature  feature  clean  no upstream  ?     ?

5 repos: 1 dirty, 3 unpushed, 1 behind
? pins: tuteur not found on PATH
? runs: pot not found on PATH
```

| command | what it does |
|---|---|
| `grove ls [ADDRESS] [--path]` | every checkout, or those in a group: branch, tree, sync state, pins, runs |
| `grove status [ADDRESS]` | one repository: everything grove knows, each fact with its source and age |
| `grove sync [ADDRESS]` | fetch every remote in parallel. Never merges. Skips this host's production repositories |
| `grove open ADDRESS` | open the forge page |
| `grove rollout [PROVIDER]` | the convergence view: each provider's latest signed release against every pin of it, in repositories and on hosts, with the next command for each pin behind |
| `grove here [--report]` | what this host owes (pull, bump, switch) with the commands; `--report` prints its report for another host |
| `grove config check` | validate `grove.toml` and show which line set each value |

An ADDRESS is a name (`beta`), `owner/name`, a path, `.`, or a group (`@tools`, `@dirty`, `@unpushed`,
`@behind`). Every command takes `--json` (schema `grove/1`).

## Status

**G1, pre-1.0 (0.1.0, 0.2.0 in preparation).** Every command in the table is built and passes its transcript in
[docs/reference/cli/](docs/reference/cli/). pins and runs come from tuteur's `status/1` and pot's `runs/1`
contracts, tested against their golden fixtures in [tests/contracts/](tests/contracts/). G1 added `rollout` and
`here`, with host reports; the design's other G1 asks (`tuteur release verify`, `pot doctor --json`) belong to
those tools.

## Docs

- Concepts (why): [read only](docs/concepts/read-only.md), [facts and sources](docs/concepts/facts.md),
  [repositories, worktrees and addresses](docs/concepts/repositories.md)
- Guides (how): [set up grove](docs/guides/setup.md), [develop and test grove](docs/guides/develop.md)
- Reference (exact): [CLI transcripts](docs/reference/cli/), [grove.toml](docs/reference/config.md),
  [JSON output and exit codes](docs/reference/json-and-exit-codes.md)
- For agents working on grove: [AGENTS.md](AGENTS.md)
