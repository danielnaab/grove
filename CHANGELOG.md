# Changelog

grove is pre-1.0: `grove/1` JSON grows by adding fields, and a minor version may add commands or config keys.

## 0.2.0

G1: the convergence view and host reports.

- `grove rollout [PROVIDER]`: each provider's latest signed release (`vMAJOR.MINOR.PATCH` with a signature, from
  local tags) against every pin of it: tuteur patterns and tuteur itself (`status/1`), flake inputs
  (`flake.lock` at `HEAD`), and hosts. Each pin is `current`, `behind` (with the next command and where it runs)
  or `off release`.
- `grove here [--report]`: what this host owes (pull, bump, switch) for the config repositories in
  `hosts.HOST.after`, from the commit in `~/.config/grove/activated/REPO`. `--report` prints the host's report.
- Reports from other hosts are read from `~/.local/state/grove/hosts/HOST.json`, verified with
  `ssh-keygen -Y verify` against `grove/allowed_signers` in `~/.workstation` at `HEAD`; one that fails is unknown.
- grove.toml: `hosts.HOST.after.ADDRESS = "COMMAND"` and `aliases.ADDRESS = [URL, ...]`. A grove.toml that uses
  them is rejected by 0.1.0 (unknown keys), so upgrade grove before committing them.

## 0.1.0

G0: `ls`, `status`, `sync`, `open`, `config check`; pins from `tuteur status --json`, runs from `pot runs --json`.
