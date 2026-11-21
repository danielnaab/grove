---
status: draft
sources:
  - grove design record, "Addressing" (quoted in the G0 brief, 2026-11-21)
---

# grove open

`grove open ADDRESS [--print] [--json]` opens the repository's forge page. The page comes from the `origin`
remote (else the first remote), as written in git config: `git@host:owner/name.git`,
`ssh://host/owner/name.git` and `https://host/owner/name.git` all become `https://host/owner/name`, without any
user or password. grove opens it with `$BROWSER`, else `xdg-open` (Linux) or `open` (macOS). `--print` prints the
URL instead.

```console
$ grove open alpha --print
https://git.example.org/daniel/alpha

$ grove open daniel/beta --print
https://git.example.org/daniel/beta

$ grove open alpha --print --json
{
  "schema": "grove/1",
  "command": "open",
  "repo": "alpha",
  "url": "https://git.example.org/daniel/alpha",
  "opened": false
}

```

## Errors

```console
$ grove open delta
? 1
grove: delta has no remote, so it has no forge page
  try: grove status delta

$ grove open alpha
? 1
grove: cannot open a browser: BROWSER is unset and xdg-open is not on PATH
  try: grove open alpha --print

$ grove open @tools --print
? 1
grove: open takes one repository, and @tools has 2: alpha, beta
  try: grove ls @tools

```
