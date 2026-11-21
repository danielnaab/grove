---
status: current
sources:
  - graft repository, branch contracts, commit 99028f7caa789acaeee280c3fd44d4382b0e5073
---

# Contract fixtures grove reads

Copied, unchanged, from graft's repository (branch `contracts`, commit `99028f7`). grove's readers in
`src/sources/` are tested against them. Copy them again when a contract changes, and note the new commit here.

| here | there |
|---|---|
| `status-1.schema.json`, `status-1/` | `contracts/status-1.schema.json`, `contracts/fixtures/status-1/` (`graft status --json`) |
| `runs-1.schema.json`, `runs-1.event.schema.json`, `runs-1/` | `pot/contracts/runs-1*.schema.json`, `pot/contracts/fixtures/runs-1/` (`pot runs --json`) |

The prose of each contract is in graft's repository: `contracts/status-1.md` and `pot/contracts/runs-1.md`. grove
reads leniently: it ignores fields it does not know and treats values it does not know as ones it does not act on.
