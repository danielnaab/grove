---
status: draft
sources:
  - grove design record, "Config" (quoted in the G0 brief, 2026-11-21)
---

# grove.toml

Location: `~/.workstation/grove/grove.toml`, read with `git -C ~/.workstation show HEAD:grove/grove.toml`, never
from the working tree. Unknown keys are errors.

| key | type | meaning |
|---|---|---|
| `version` | integer, required | must be `1` |
| `discover.scan` | list of globs | directories that may be checkouts. `~` means your home. Default: `["~/src/*", "~/src/worktrees/*/*"]` |
| `discover.ignore` | list of globs | matches are never shown |
| `tags.NAME` | list of addresses | defines the group `@NAME`. It may not be named `dirty`, `unpushed` or `behind` |
| `hosts.HOST.production` | list of addresses | on host HOST, these repositories are observed only and never fetched. HOST is the machine's hostname (`GROVE_HOST` overrides it) |
| `hosts.HOST.after.ADDRESS` | string (a command) | ADDRESS configures HOST, and the command activates it there. `grove here` and `grove rollout` print it when HOST is behind |
| `aliases.ADDRESS` | list of URLs | other URLs that name the repository at ADDRESS (a mirror), so pins of them match it in `grove rollout` |

Beside grove.toml, `grove/allowed_signers` (an `ssh-keygen` allowed signers file, read from `HEAD`) lists the
keys that sign host reports, one principal per host name, namespace `grove-report`
([setup](../guides/setup.md#host-reports)).

## Secrets

`grove config check`, and every command that loads the config, refuses a grove.toml that looks like it contains
a secret. It reports the file, line, key and rule, and never the value. Rules:

- a key whose name contains `token`, `secret`, `password`, `passwd`, `api_key`, `apikey` or `private_key`
- a URL with a password (`https://user:pass@host/...`)
- a PEM header (`-----BEGIN`) or a known credential prefix
- a long random-looking string that is not a path or glob

## Environment

| variable | effect |
|---|---|
| `GROVE_HOST` | the host name used for `hosts.HOST` (default: the machine's hostname) |
| `XDG_STATE_HOME` | state goes in `$XDG_STATE_HOME/grove` (default `~/.local/state/grove`); other hosts' reports in its `hosts/` |
| `XDG_CONFIG_HOME` | this host's activated commits are read from `$XDG_CONFIG_HOME/grove/activated/` (default `~/.config`) |
| `BROWSER` | used by `grove open` before `xdg-open` or `open` |
| `NO_COLOR` | no color, even on a terminal |
