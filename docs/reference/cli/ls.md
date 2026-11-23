---
status: draft
sources:
  - grove design record, "Addressing", "Facts" (quoted in the G0 brief, 2026-11-21)
---

# grove ls

`grove ls [ADDRESS] [--path] [--json]` lists repositories, one row per checkout, with each repository's
worktrees indented under it. It never fetches: ahead and behind are as of the last fetch, which `FETCHED` shows.

| column | meaning |
|---|---|
| `TREE` | `clean`, or `dirty` (staged, modified, untracked or conflicted files) |
| `SYNC` | `even`, `↑N` ahead, `↓N` behind, `↑N ↓M`, `no upstream`, `gone` (the upstream branch was deleted), or `local only` (no remotes) |
| `PINS` | tuteur patterns (`status/1`): how many, how many outdated, how many problems. `?` when unknown |
| `RUNS` | pot runs in this checkout (`runs/1`). `?` when unknown |
| `FETCHED` | how long ago the repository was last fetched, by anyone (`FETCH_HEAD`). `never` if no fetch is recorded, `-` if there are no remotes |

```console
$ grove ls
REPO       BRANCH   TREE   SYNC         PINS  RUNS  FETCHED
alpha      main     clean  even         ?     ?     [..]
beta       main     dirty  ↑2           ?     ?     [..]
delta      main     clean  local only   ?     ?     -
epsilon    main     clean  no upstream  ?     ?     never
gamma      main     clean  ↓1           ?     ?     [..]
└ feature  feature  clean  no upstream  ?     ?

5 repos: 1 dirty, 3 unpushed, 1 behind
? pins: tuteur not found on PATH
? runs: pot not found on PATH

```

`~/src/notes` is not a checkout and `~/src/scratch` is ignored by grove.toml, so neither is listed. gamma's
worktree has no upstream, so gamma counts as unpushed.

## Groups

Derived groups: `@dirty`, `@unpushed`, `@behind`. Tags from grove.toml: here, `@tools` and `@infra`.

```console
$ grove ls @dirty
REPO  BRANCH  TREE   SYNC  PINS  RUNS  FETCHED
beta  main    dirty  ↑2    ?     ?     [..]

1 repo in @dirty: 1 dirty, 1 unpushed
? pins: tuteur not found on PATH
? runs: pot not found on PATH

$ grove ls @unpushed
REPO       BRANCH   TREE   SYNC         PINS  RUNS  FETCHED
beta       main     dirty  ↑2           ?     ?     [..]
epsilon    main     clean  no upstream  ?     ?     never
gamma      main     clean  ↓1           ?     ?     [..]
└ feature  feature  clean  no upstream  ?     ?

3 repos in @unpushed: 1 dirty, 3 unpushed, 1 behind
? pins: tuteur not found on PATH
? runs: pot not found on PATH

$ grove ls @tools
REPO   BRANCH  TREE   SYNC  PINS  RUNS  FETCHED
alpha  main    clean  even  ?     ?     [..]
beta   main    dirty  ↑2    ?     ?     [..]

2 repos in @tools: 1 dirty, 1 unpushed
? pins: tuteur not found on PATH
? runs: pot not found on PATH

$ grove ls @behind
REPO       BRANCH   TREE   SYNC         PINS  RUNS  FETCHED
gamma      main     clean  ↓1           ?     ?     [..]
└ feature  feature  clean  no upstream  ?     ?

1 repo in @behind: 1 unpushed, 1 behind
? pins: tuteur not found on PATH
? runs: pot not found on PATH

```

A repository's worktrees are always listed with it, even when only one of its checkouts is in the group.
ls takes one address. A group that matches nothing prints `no repos in @NAME` and exits 0.

## Paths, for `gcd`

`--path` prints the main checkout path of each repository, one per line, and nothing else:

```console
$ grove ls --path beta
[HOME]/src/beta

$ grove ls --path @tools
[HOME]/src/alpha
[HOME]/src/beta

```

Use it with a shell function: `gcd() { cd "$(grove ls --path "$1")" || return; }`.

## Errors

```console
$ grove ls zeta
? 1
grove: no repository named zeta
  try: grove ls

$ grove ls @nope
? 1
grove: no group @nope (groups: @behind, @dirty, @infra, @tools, @unpushed)
  try: grove config check

```

## JSON

Every fact has `source` and `observed_at`. An unknown fact has `unknown` (the reason) instead of `value`.

```console
$ grove ls beta --json
{
  "schema": "grove/1",
  "command": "ls",
  "repos": [
    {
      "name": "beta",
      "slug": "daniel/beta",
      "path": "[HOME]/src/beta",
      "common_dir": "[HOME]/src/beta/.git",
      "tags": {
        "value": [
          "tools"
        ],
        "source": "grove.toml:8",
        "observed_at": "[..]"
      },
      "production": {
        "value": false,
        "source": "grove.toml",
        "observed_at": "[..]"
      },
      "fetched": {
        "value": "[..]",
        "source": "FETCH_HEAD",
        "observed_at": "[..]"
      },
      "remotes": {
        "value": [
          {
            "name": "origin",
            "url": "https://git.example.org/daniel/beta.git"
          }
        ],
        "source": "git config",
        "observed_at": "[..]"
      },
      "forge": {
        "value": "https://git.example.org/daniel/beta",
        "source": "remote origin",
        "observed_at": "[..]"
      },
      "last_sync": {
        "value": null,
        "source": "grove state",
        "observed_at": "[..]"
      },
      "checkouts": [
        {
          "path": "[HOME]/src/beta",
          "worktree": false,
          "branch": {
            "value": "main",
            "source": "git",
            "observed_at": "[..]"
          },
          "tree": {
            "value": {
              "dirty": true,
              "staged": 0,
              "modified": 1,
              "untracked": 1,
              "conflicted": 0
            },
            "source": "git",
            "observed_at": "[..]"
          },
          "upstream": {
            "value": {
              "ref": "origin/main",
              "ahead": 2,
              "behind": 0
            },
            "source": "git",
            "observed_at": "[..]"
          },
          "pins": {
            "unknown": "tuteur not found on PATH",
            "source": "tuteur",
            "observed_at": "[..]"
          },
          "runs": {
            "unknown": "pot not found on PATH",
            "source": "pot",
            "observed_at": "[..]"
          }
        }
      ]
    }
  ],
  "summary": {
    "repos": 1,
    "dirty": 1,
    "unpushed": 1,
    "behind": 0
  },
  "sources": {
    "git": {
      "state": "ok"
    },
    "tuteur": {
      "state": "absent",
      "detail": "tuteur not found on PATH"
    },
    "pot": {
      "state": "absent",
      "detail": "pot not found on PATH"
    }
  }
}

```
