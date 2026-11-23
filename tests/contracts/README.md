---
status: current
sources:
  - tuteur repository, tag v0.1.0, commit e8236dd2e40b5517a21824dceb31dc37588f507a (status/1)
  - graft repository, branch contracts, commit 99028f7caa789acaeee280c3fd44d4382b0e5073 (runs/1, first copy)
  - pot repository, main, commit 1db155dcc0213eaa9e00d53cc5b8ba10a8214103 (runs/1, checked unchanged 2026-11-22)
---

# Contract fixtures grove reads

Copied, unchanged, from the repositories that own each contract. grove's readers in `src/sources/` are tested
against them. Copy them again when a contract changes, and note the new commit here.

| here | there |
|---|---|
| `status-1.schema.json`, `status-1/` | tuteur v0.1.0: `contracts/status-1.schema.json`, `contracts/fixtures/status-1/` (`tuteur status --json`) |
| `runs-1.schema.json`, `runs-1.event.schema.json`, `runs-1/` | pot: `contracts/runs-1*.schema.json`, `contracts/fixtures/runs-1/` (`pot runs --json`) |

They were first copied from graft's `contracts` branch (commit `99028f7`). On 2026-11-22 tuteur v0.1.0's
`status/1` files and pot's `runs/1` files were byte-identical to those copies, so only the provenance changed.
tuteur keeps graft's field names in `status/1` (`scions` are its patterns, `graft` is its own pin), and produces
it from 0.1.0 (graft from v2.22.0), so grove reads either tool's output with one parser.

The prose of each contract is in its owner's repository: tuteur's `contracts/status-1.md` and pot's
`contracts/runs-1.md`. grove reads leniently: it ignores fields it does not know and treats values it does not
know as ones it does not act on.
