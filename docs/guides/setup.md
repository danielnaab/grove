---
status: draft
sources:
  - grove design record, "Config" (quoted in the G0 brief, 2026-11-21)
---

# Set up grove

1. **Install.** grove is a Nix flake. Add it to your Home Manager packages, or try it with
   `nix run .#grove -- ls` from a checkout of this repository.

2. **Write `grove.toml`** in your workstation repository at `grove/grove.toml`:

   ```toml
   version = 1

   [discover]
   scan = ["~/src/*", "~/src/worktrees/*/*"]
   ignore = ["~/src/scratch"]

   [tags]
   tools = ["grove", "pot", "tuteur"]

   [hosts.platform-vm]
   production = ["homelab"]
   ```

   Without a committed grove.toml, grove uses the `scan` shown above, with no tags or production repositories.

3. **Commit it.** grove reads `grove.toml` from `HEAD`, never the working tree (design record, "Config").
   Check it first:

   ```sh
   grove config check --file ~/.workstation/grove/grove.toml   # the draft
   git -C ~/.workstation commit grove/grove.toml
   grove config check                                          # what grove uses
   ```

4. **Add `gcd`**, to jump to a checkout by name:

   ```sh
   gcd() { cd "$(grove ls --path "$1")" || return; }
   ```

   `grove ls --path ADDRESS` prints the main checkout path of each repository in ADDRESS, one per line.

5. Run `grove sync` when you want fresh ahead/behind counts. `grove ls` never fetches.

## Hosts and the convergence view

`grove rollout` shows a host's pins when grove.toml names the repositories that configure it, and the command
that activates each:

```toml
[hosts.platform-vm.after]
".workstation" = "home-manager switch --flake ~/.workstation#platform-vm"
```

The host's config writes the commit it activated to `~/.config/grove/activated/REPO` (REPO as written in
`after`). With Home Manager, in a module that has the flake's `inputs`:

```nix
xdg.configFile."grove/activated/.workstation".text = inputs.self.rev or inputs.self.dirtyRev or "unknown";
```

A pin is matched with a repository by URL. When consumers pin a mirror (a GitHub copy of a Forgejo
repository), name it:

```toml
[aliases]
knowledge-base-kit = ["https://github.com/danielnaab/knowledge-base-kit"]
```

## Host reports

A host that is not the one you run `grove rollout` on (a Mac) sends a signed report there. On the Mac, with a key
`~/.ssh/grove_report` (no passphrase, or one ssh-agent holds) and `platform-vm` as the always-on host:

```sh
host=$(hostname -s)
grove here --report >"/tmp/grove-$host.json"
ssh-keygen -Y sign -n grove-report -f ~/.ssh/grove_report "/tmp/grove-$host.json"
ssh platform-vm mkdir -p .local/state/grove/hosts
scp "/tmp/grove-$host.json" "platform-vm:.local/state/grove/hosts/$host.json"
scp "/tmp/grove-$host.json.sig" "platform-vm:.local/state/grove/hosts/$host.json.sig"
```

On the always-on host, list the Mac's key in `grove/allowed_signers` beside grove.toml, and commit it (grove
reads it from `HEAD`). The principal is the host name grove uses (`hostname -s`, or `GROVE_HOST`):

```text
flexion-mac-L4C4WDQCLK namespaces="grove-report" ssh-ed25519 AAAA...
```

A report that is unsigned, signed by another key, or names another host shows as unknown in `grove rollout`,
with its age. Nothing acts on it.
