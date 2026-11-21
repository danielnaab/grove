---
status: draft
sources:
  - grove design record, "Config" (quoted in the G0 brief, 2026-11-21)
---

# grove config check

`grove config check [--file PATH] [--json]` validates grove.toml and shows each value with the file and line
that set it. By default it checks what grove uses: `grove/grove.toml` at `HEAD` of `~/.workstation`. It also
says when the working tree differs from HEAD, because grove ignores uncommitted edits. See the
[grove.toml reference](../config.md).

```console
$ grove config check
ok: ~/.workstation/grove/grove.toml at HEAD (448ebef)
  version                    1                             grove.toml:1
  discover.scan              ~/src/*, ~/src/worktrees/*/*  grove.toml:4
  discover.ignore            ~/src/scratch                 grove.toml:5
  tags.tools                 alpha, beta                   grove.toml:8
  tags.infra                 gamma                         grove.toml:9
  hosts.testhost.production  gamma                         grove.toml:12

this host is testhost: gamma is production here (observed only)
note: grove.toml in the working tree differs from HEAD, and grove reads HEAD
  fix: git -C ~/.workstation commit grove/grove.toml

```

## Drafts

`--file` checks a file as written, for example a draft before you commit it:

```console
$ grove config check --file draft.toml
ok: draft.toml (a draft: grove reads ~/.workstation/grove/grove.toml at HEAD)
  version        1                   draft.toml:1
  discover.scan  ~/src/*             draft.toml:4
  tags.tools     alpha, beta, delta  draft.toml:7

```

## Problems

Unknown keys are errors, with their line:

```console
$ grove config check --file typo.toml
? 1
grove: typo.toml:4: unknown key discover.scna (did you mean discover.scan?)
  fix: $EDITOR typo.toml

```

Anything that looks like a secret is rejected before anything else, even inside an unknown key. The value is
never printed:

```console
$ grove config check --file secret.toml
? 1
grove: secret.toml looks like it contains secrets (values not shown)
  secret.toml:8: tags.mirror[0]: a URL with a password
  secret.toml:11: hosts.testhost.api_token: the key name contains "token"
  fix: $EDITOR secret.toml, then rotate any secret that was ever committed

```

## JSON

```console
$ grove config check --json
{
  "schema": "grove/1",
  "command": "config check",
  "ok": true,
  "file": "[HOME]/.workstation/grove/grove.toml",
  "commit": "448ebefd4de262403f65614ad4a3279e1cb25c3d",
  "working_tree_differs": true,
  "host": "testhost",
  "values": [
    {
      "key": "version",
      "value": 1,
      "line": 1
    },
    {
      "key": "discover.scan",
      "value": [
        "~/src/*",
        "~/src/worktrees/*/*"
      ],
      "line": 4
    },
    {
      "key": "discover.ignore",
      "value": [
        "~/src/scratch"
      ],
      "line": 5
    },
    {
      "key": "tags.tools",
      "value": [
        "alpha",
        "beta"
      ],
      "line": 8
    },
    {
      "key": "tags.infra",
      "value": [
        "gamma"
      ],
      "line": 9
    },
    {
      "key": "hosts.testhost.production",
      "value": [
        "gamma"
      ],
      "line": 12
    }
  ],
  "problems": []
}

```

A grove.toml that is not committed, and the defaults that apply without one, are covered by the integration
tests.
