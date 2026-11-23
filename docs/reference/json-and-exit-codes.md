---
status: draft
sources:
  - grove design record, "For agents" (quoted in the G0 brief, 2026-11-21)
---

# JSON output and exit codes

## Exit codes

| code | meaning |
|---|---|
| 0 | ok. Facts from an absent tool (not on `PATH`) are `unknown`, and that alone is still 0 |
| 1 | error: nothing useful was produced. This includes usage errors |
| 3 | partial: the output is valid, but some sources were unreadable or some remotes failed |
| 10 | needs you: a person has to decide. Reserved; no G0 command returns it |

grove never exits 125, which belongs to pot (design record, "For agents").

## JSON

Every command takes `--json`. Output is one JSON document on stdout, even for errors:

```json
{
  "schema": "grove/1",
  "command": "status",
  "error": { "message": "no repository named zeta", "try": "grove ls" }
}
```

A **fact** looks like this:

```json
{ "value": "main", "source": "git", "observed_at": "2026-11-21T17:02:11Z" }
```

An unknown fact has `unknown` (the reason) and no `value`:

```json
{ "unknown": "tuteur not found on PATH", "source": "tuteur", "observed_at": "2026-11-21T17:02:11Z" }
```

`grove/1` grows by adding fields. Renaming or removing one is `grove/2`. Paths in JSON are absolute. The
per-command shapes are shown in the [CLI transcripts](cli/).
