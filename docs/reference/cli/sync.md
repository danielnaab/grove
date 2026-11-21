---
status: draft
sources:
  - grove design record, "Read only", "For agents" (quoted in the G0 brief, 2026-11-21)
---

# grove sync

`grove sync [ADDRESS] [--json]` fetches every remote of every addressed repository, in parallel, and then shows
each repository's sync state. It runs `git fetch` and nothing else: no merge, no rebase, no checkout. Your
branches and files do not change. Prompts are off, so a remote that needs a password fails instead of waiting.

- Repositories listed in this host's `production` are skipped: they are observed only.
- If any remote fails, the exit code is 3 (partial). The other results are still valid.
- Each outcome is saved in `~/.local/state/grove/sync.json`, which `grove status` shows as `last sync`.

`grove ls` never fetches, so it still shows alpha as even, although its remote has a new commit. A sync brings
that commit in:

```console
$ grove sync @tools
REPO   REMOTE  RESULT               SYNC
alpha  origin  updated origin/main  ↓1
beta   origin  no changes           ↑2

fetched 2 of 2 repos. Nothing was merged or checked out.

$ grove ls @tools
REPO   BRANCH  TREE   SYNC  PINS  RUNS  FETCHED
alpha  main    clean  ↓1    ?     ?     [..]
beta   main    dirty  ↑2    ?     ?     [..]

2 repos in @tools: 1 dirty, 1 unpushed, 1 behind
? pins: graft not found on PATH
? runs: pot not found on PATH

```

Everything, with one failing remote and one production repository:

```console
$ grove sync
? 3
REPO     REMOTE  RESULT               SYNC
alpha    origin  no changes           ↓1
beta     origin  no changes           ↑2
delta            no remotes           local only
epsilon  origin  failed               no upstream
gamma            skipped: production  ↓1

fetched 2 of 5 repos: 1 failed, 1 skipped (production on testhost), 1 local only. Nothing was merged or checked out.
grove: fetching epsilon (origin) failed: '[..]/remotes/epsilon.git' does not appear to be a git repository
  try: git -C ~/src/epsilon fetch origin

$ grove status epsilon
epsilon  daniel/epsilon  ~/src/epsilon

~/src/epsilon
  branch      main   git, now
  tree        clean  git, now
  upstream    none   git, now
  pins        ?      graft: not found on PATH, now
  runs        ?      pot: not found on PATH, now

repository
  fetched     never                                              FETCH_HEAD, now
  remote      origin https://git.example.org/daniel/epsilon.git  git config, now
  forge       https://git.example.org/daniel/epsilon             remote origin, now
  tags        none                                               grove.toml, now
  production  no                                                 grove.toml, now
  last sync   failed[..]

$ grove sync gamma
REPO   REMOTE  RESULT               SYNC
gamma          skipped: production  ↓1

fetched 0 of 1 repo: 1 skipped (production on testhost). Nothing was merged or checked out.

```

## JSON

`result` is one of `updated`, `unchanged`, `failed`, `skipped` or `no_remotes`. `updated` lists the
remote-tracking refs the fetch moved.

```console
$ grove sync beta --json
{
  "schema": "grove/1",
  "command": "sync",
  "results": [
    {
      "repo": "beta",
      "remote": "origin",
      "result": "unchanged",
      "updated": [],
      "error": null,
      "observed_at": "[..]"
    }
  ],
  "summary": {
    "repos": 1,
    "fetched": 1,
    "failed": 0,
    "skipped": 0,
    "local_only": 0
  }
}

```
