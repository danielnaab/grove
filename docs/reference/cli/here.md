---
status: draft
sources:
  - grove design record (potting-shed, 2026-11-19-grove-design/design.md), §1 "grove here", §4 "Host reports", §8 G1
  - tests/fixtures/README.md, "The estate"
---

# grove here

`grove here [--report] [--json]` says what this host owes: the steps that would bring it level with its config
repositories and their providers' latest releases, each with its command. It runs none of them.

A host's config repositories are the keys of `hosts.HOST.after` in grove.toml, and each value is the command that
activates that repository on HOST. The commit a host activated is read from `~/.config/grove/activated/REPO`
(`$XDG_CONFIG_HOME`), a file the host's config writes when it is activated (for Home Manager:
`xdg.configFile."grove/activated/.workstation".text = self.rev or self.dirtyRev or "unknown";`).

Steps, in the order they apply:

1. **pull**: the config checkout is behind its upstream (as of the last fetch).
2. **bump**: `HEAD` pins an older release than the provider's latest.
3. **switch**: the activated commit is not `HEAD`.

```console
$ grove here
testhost owes 1 step
  switch ~/.workstation: activated [..], HEAD [..]
    home-manager switch --flake ~/.workstation

REPO          PROVIDER  ACTIVATED  HEAD    LATEST
.workstation  tool      v0.1.0     v0.2.0  v0.2.0

```

On a host with nothing in `hosts.HOST.after`, `grove here` says so and exits 1:

```text
grove: elsewhere: grove.toml has no hosts.elsewhere.after, so grove does not know which repositories configure this host
  try: grove config check
```

## Reports

`grove here --report` prints this host's report: the commit it activated of each config repository and the pins
at that commit. It is always JSON. grove holds no keys and does not send it: the host signs it with
`ssh-keygen -Y sign -n grove-report` and copies the report and its `.sig` to the always-on host's
`~/.local/state/grove/hosts/HOST.json` (see [setup](../../guides/setup.md#host-reports)).

There, `grove rollout` checks the signature with `ssh-keygen -Y verify` against `grove/allowed_signers` in the
config repository at `HEAD` (principal: the host name, namespace `grove-report`), and that the report names the
host its file does. A report that fails either check is shown as unknown, with its age.

```console
$ grove here --report
{
  "schema": "grove/1",
  "command": "here",
  "host": "testhost",
  "observed_at": "[..]",
  "configs": [
    {
      "repo": ".workstation",
      "activated": "[..]",
      "pins": [
        {
          "via": "flake input nixpkgs",
          "url": "https://github.com/NixOS/nixpkgs",
          "tag": null,
          "commit": "d3b13ad75cc492690728f9307e443dec9ef9a20e"
        },
        {
          "via": "flake input tool",
          "url": "https://git.example.org/daniel/tool.git",
          "tag": "v0.1.0",
          "commit": "f3f45c4f1d129528ed6ebe66e5672579c43682dd"
        }
      ]
    }
  ]
}

```
