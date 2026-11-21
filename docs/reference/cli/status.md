---
status: draft
sources:
  - grove design record, "Facts", "Read only" (quoted in the G0 brief, 2026-11-21)
---

# grove status

`grove status [ADDRESS] [--json]` shows everything grove knows about one repository. There is one section per
checkout (the main checkout first, then worktrees) and one for the repository. Each fact shows its source and
age. `now` means it was read during this command. ADDRESS defaults to `.`, the repository you are in.

```console
$ grove status beta
beta  daniel/beta  ~/src/beta

~/src/beta
  branch      main                     git, now
  tree        1 modified, 1 untracked  git, now
  upstream    origin/main ↑2           git, now
  pins        ?                        graft: not found on PATH, now
  runs        ?                        pot: not found on PATH, now

repository
  fetched     [..]
  remote      origin https://git.example.org/daniel/beta.git  git config, now
  forge       https://git.example.org/daniel/beta             remote origin, now
  tags        @tools                                          grove.toml:8, now
  production  no                                              grove.toml, now
  last sync   none recorded                                   grove state, now

```

A repository with a worktree, listed as production for this host:

```console
$ grove status gamma
gamma  daniel/gamma  ~/src/gamma  production on testhost: observed only

~/src/gamma
  branch      main            git, now
  tree        clean           git, now
  upstream    origin/main ↓1  git, now
  pins        ?               graft: not found on PATH, now
  runs        ?               pot: not found on PATH, now

~/src/worktrees/gamma/feature  (worktree)
  branch      feature  git, now
  tree        clean    git, now
  upstream    none     git, now
  pins        ?        graft: not found on PATH, now
  runs        ?        pot: not found on PATH, now

repository
  fetched     [..]
  remote      origin https://git.example.org/daniel/gamma.git  git config, now
  forge       https://git.example.org/daniel/gamma             remote origin, now
  tags        @infra                                           grove.toml:9, now
  production  yes, on testhost: observed only                  grove.toml:12, now
  last sync   none recorded                                    grove state, now

```

## Errors

status takes exactly one repository:

```console
$ grove status zeta
? 1
grove: no repository named zeta
  try: grove ls

$ grove status @tools
? 1
grove: status takes one repository, and @tools has 2: alpha, beta
  try: grove ls @tools

```

## JSON

The repository object is the same one `grove ls --json` lists. Errors are JSON too.

```console
$ grove status delta --json
{
  "schema": "grove/1",
  "command": "status",
  "repo": {
    "name": "delta",
    "slug": null,
    "path": "[HOME]/src/delta",
    "common_dir": "[HOME]/src/delta/.git",
    "tags": {
      "value": [],
      "source": "grove.toml",
      "observed_at": "[..]"
    },
    "production": {
      "value": false,
      "source": "grove.toml",
      "observed_at": "[..]"
    },
    "fetched": {
      "value": null,
      "source": "FETCH_HEAD",
      "observed_at": "[..]"
    },
    "remotes": {
      "value": [],
      "source": "git config",
      "observed_at": "[..]"
    },
    "forge": {
      "value": null,
      "source": "git config",
      "observed_at": "[..]"
    },
    "last_sync": {
      "value": null,
      "source": "grove state",
      "observed_at": "[..]"
    },
    "checkouts": [
      {
        "path": "[HOME]/src/delta",
        "worktree": false,
        "branch": {
          "value": "main",
          "source": "git",
          "observed_at": "[..]"
        },
        "tree": {
          "value": {
            "dirty": false,
            "staged": 0,
            "modified": 0,
            "untracked": 0,
            "conflicted": 0
          },
          "source": "git",
          "observed_at": "[..]"
        },
        "upstream": {
          "value": null,
          "source": "git",
          "observed_at": "[..]"
        },
        "pins": {
          "unknown": "graft not found on PATH",
          "source": "graft",
          "observed_at": "[..]"
        },
        "runs": {
          "unknown": "pot not found on PATH",
          "source": "pot",
          "observed_at": "[..]"
        }
      }
    ]
  },
  "sources": {
    "git": {
      "state": "ok"
    },
    "graft": {
      "state": "absent",
      "detail": "graft not found on PATH"
    },
    "pot": {
      "state": "absent",
      "detail": "pot not found on PATH"
    }
  }
}

$ grove status zeta --json
? 1
{
  "schema": "grove/1",
  "command": "status",
  "error": {
    "message": "no repository named zeta",
    "try": "grove ls"
  }
}

```

`grove status .` and `grove status PATH` (from inside a checkout or worktree) are covered by the integration
tests, because a transcript cannot change directory.
