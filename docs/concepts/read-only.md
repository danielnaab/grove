---
status: current
sources:
  - grove design record, "Read only" (quoted in the G0 brief, 2026-11-21)
---

# Read only

grove observes. It never changes HEAD, a branch, the index or the working tree of any repository it shows
(design record, "Read only"). This is what makes it safe to run anywhere, at any time, by a person or an agent.

What that means in practice:

- **`grove sync` only fetches.** `git fetch` updates remote-tracking refs (`origin/main`) and `FETCH_HEAD`.
  It never touches your branches or files. grove never merges, rebases or pulls. You do that when you choose to.
- **Reading does not write either.** A plain `git status` refreshes the index and briefly takes `index.lock`,
  which can collide with an editor or another git command. grove runs it with `--no-optional-locks`.
- **Production repositories are observed only.** If `grove.toml` lists a repository under this host's
  `production`, grove shows it but never fetches it.
- **No credentials.** grove holds no keys, tokens or push rights. Fetches use whatever your git already uses,
  with prompts turned off (`GIT_TERMINAL_PROMPT=0`), so a fetch that needs a password fails instead of hanging.
  `grove config check` rejects anything in `grove.toml` that looks like a secret.
- **Config from the committed tree.** grove reads `grove.toml` at `HEAD` of `~/.workstation`, never the working
  tree, so what grove does is always something you committed.

Machine state (the last sync outcome) lives in `~/.local/state/grove/` and is safe to delete.
