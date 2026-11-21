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
   tools = ["grove", "pot", "graft"]

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
