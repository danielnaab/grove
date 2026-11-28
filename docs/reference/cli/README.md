---
status: draft
sources:
  - grove design record (quoted in the G0 brief, 2026-11-21)
  - tests/fixtures/README.md (the world these transcripts run in)
---

# CLI reference

Each page is a transcript: commands as you would type them, their exit code (`? N`, when not 0), and their
exact output. The pages are also grove's tests (see [develop and test grove](../../guides/develop.md)). They
run in the [fixture world](../../../tests/fixtures/README.md), on host `testhost`, where `tuteur` and `pot` are
not installed. That is why pins and runs show as `?`.

- [`grove ls`](ls.md): list repositories
- [`grove status`](status.md): one repository, fact by fact
- [`grove sync`](sync.md): fetch everything, merge nothing
- [`grove open`](open.md): the forge page
- [`grove rollout`](rollout.md): each provider's latest release against its consumers and hosts (the
  convergence view; runs in the [estate](../../../tests/fixtures/README.md#the-estate))
- [`grove here`](here.md): what this host owes, and its report for other hosts
- [`grove config check`](config-check.md): validate grove.toml

```console
$ grove --help
Keep your git checkouts in view. Read only.

Usage: grove [OPTIONS] <COMMAND>

Commands:
  ls       List repositories: branch, tree, ahead/behind, pins, runs
  status   Everything grove knows about one repository, with sources and ages
  sync     Fetch every remote, in parallel. Never merges
  open     Open a repository's forge page
  rollout  Each provider's latest release against every pin of it, in repositories and on hosts
  here     What this host owes: pull, bump, switch, with the commands
  config   Check grove.toml
  help     Print this message or the help of the given subcommand(s)

Options:
      --json     Print JSON (schema grove/1)
  -h, --help     Print help
  -V, --version  Print version

$ grove --version
grove 0.2.0

```

A usage error exits 1. grove maps clap's 2 to 1, so the exit codes stay 0, 1, 3 and 10:

```console
$ grove frobnicate
? 1
error: unrecognized subcommand 'frobnicate'

Usage: grove [OPTIONS] <COMMAND>

For more information, try '--help'.

```
