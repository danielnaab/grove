---
status: current
sources:
  - grove design record, "Addressing" and "Facts" (quoted in the G0 brief, 2026-11-21)
---

# Repositories, worktrees and addresses

## Checkouts and repositories

`[discover] scan` globs find **checkouts**: directories where git works. Checkouts that share a git common dir
are one **repository**, and its worktrees are listed under it (design record, "Facts"). A repository's name is
its main checkout's directory name. Its `owner/name` comes from its forge remote. Worktrees that the scan globs
miss are still found, through `git worktree list`.

Per checkout: branch, tree, upstream, pins, runs. Per repository: remotes, last fetch, tags, production.

## Addresses

You name repositories with an address (design record, "Addressing"). There is no query language.

| address | means |
|---|---|
| `beta` | the repository named `beta`. If two share a name, grove says so and lists their `owner/name` |
| `daniel/beta` | by forge owner and name |
| `~/src/beta`, `.` | the repository that contains this path, from any of its worktrees |
| `@tools` | a group from `[tags]` in grove.toml |
| `@dirty` | some checkout has uncommitted changes, or untracked files |
| `@unpushed` | some checkout's branch is ahead of its upstream, or has no upstream while the repository has a remote |
| `@behind` | some checkout's branch is behind its upstream, as of the last fetch |

`@consumers:P` (repositories that pin P) comes after G0, from tuteur's `status/1`.
