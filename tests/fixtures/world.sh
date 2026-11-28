#!/usr/bin/env bash
# Build grove's fixture world: a throwaway HOME with checkouts in every state grove reports.
#
#   tests/fixtures/world.sh DIR [estate]   # DIR must be empty or absent
#
# Layout (HOME = DIR/home), described for readers in tests/fixtures/README.md:
#
#   home/src/alpha        clean, even with origin; its remote has one commit alpha has not fetched yet
#   home/src/beta         dirty (1 modified, 1 untracked), 2 commits ahead of origin
#   home/src/delta        local only: no remotes
#   home/src/epsilon      a remote that cannot be fetched, never fetched
#   home/src/gamma        clean, 1 behind origin; production on host "testhost"
#   home/src/worktrees/gamma/feature   gamma's worktree on branch "feature", no upstream
#   home/src/notes        not a git checkout (skipped silently)
#   home/src/scratch      a checkout matched by [discover] ignore
#   home/.workstation     the config repo: grove/grove.toml committed, plus an uncommitted edit
#   remotes/*.git         bare remotes. Clones use https://git.example.org/daniel/NAME.git, which a
#                         repo-local url.insteadOf maps to these, so forge pages and fetches both work offline.
#
#
# With `estate`, the world also has providers, consumers and host reports for `grove rollout` and `grove here`
# (see the end of this file and tests/fixtures/README.md).
#
# Every commit has a fixed author, committer and date, so commit IDs are the same on every run (except the
# estate's ~/.workstation commits, which hold a public key made on each run).
set -euo pipefail

root=${1:?usage: world.sh DIR [estate]}
estate=${2:-}
mkdir -p "$root"
root=$(cd "$root" && pwd -P)
[ -z "$(ls -A "$root")" ] || { echo "world.sh: $root is not empty" >&2; exit 1; }

export HOME="$root/home"
# GIT_CONFIG_COUNT and GIT_CONFIG_PARAMETERS carry config past GIT_CONFIG_GLOBAL (an agent's commit signing, for
# one), and a signed commit's ID changes on every run
unset GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_TERMINAL_PROMPT=0 TZ=UTC LC_ALL=C
export GIT_AUTHOR_NAME=Fixture GIT_AUTHOR_EMAIL=fixture@example.org
export GIT_COMMITTER_NAME=Fixture GIT_COMMITTER_EMAIL=fixture@example.org
export GIT_AUTHOR_DATE='2026-11-01T12:00:00Z' GIT_COMMITTER_DATE='2026-11-01T12:00:00Z'

forge=https://git.example.org/daniel/
remotes="$root/remotes"
src="$HOME/src"
mkdir -p "$remotes" "$src" "$src/notes"

g() { git -c init.defaultBranch=main -c advice.detachedHead=false "$@"; }

# commit DIR FILE MESSAGE: write FILE (contents = MESSAGE) and commit it.
commit() {
  printf '%s\n' "$3" >>"$1/$2"
  g -C "$1" add "$2"
  g -C "$1" commit -q -m "$3"
}

# remote NAME: a bare remote seeded with one commit.
remote() {
  local seed="$root/seed-$1"
  g init -q "$seed"
  commit "$seed" README "$1: initial"
  g init -q --bare "$remotes/$1.git"
  g -C "$seed" push -q "$remotes/$1.git" main
  rm -rf "$seed"
}

# clone NAME: clone NAME into ~/src via its forge URL, keeping the insteadOf mapping in the clone.
clone() {
  g -c "url.$remotes/.insteadOf=$forge" clone -q "$forge$1.git" "$src/$1"
  g -C "$src/$1" config "url.$remotes/.insteadOf" "$forge"
  g -C "$src/$1" fetch -q origin # leaves FETCH_HEAD, which a plain clone does not
}

# upstream_commit NAME MESSAGE: someone else pushes a commit to NAME's remote.
upstream_commit() {
  local other="$root/other-$1"
  g clone -q "$remotes/$1.git" "$other"
  commit "$other" README "$2"
  g -C "$other" push -q origin main
  rm -rf "$other"
}

# alpha: clean and even, then someone pushes, so the next `grove sync` brings in one commit.
remote alpha
clone alpha
upstream_commit alpha "alpha: from elsewhere"

# beta: two local commits, one modified file and one untracked file.
remote beta
clone beta
commit "$src/beta" README "beta: local 1"
commit "$src/beta" README "beta: local 2"
echo "uncommitted" >>"$src/beta/README"
echo "scratch" >"$src/beta/notes.txt"

# gamma: one commit behind (already fetched), plus a worktree on a local branch.
remote gamma
clone gamma
upstream_commit gamma "gamma: from elsewhere"
g -C "$src/gamma" fetch -q origin
mkdir -p "$src/worktrees/gamma"
g -C "$src/gamma" worktree add -q -b feature "$src/worktrees/gamma/feature"
commit "$src/worktrees/gamma/feature" FEATURE "gamma: feature work"

# delta: local only.
g init -q "$src/delta"
commit "$src/delta" README "delta: initial"

# epsilon: its remote does not exist, so fetching fails, quickly and offline.
g init -q "$src/epsilon"
commit "$src/epsilon" README "epsilon: initial"
g -C "$src/epsilon" remote add origin "${forge}epsilon.git"
g -C "$src/epsilon" config "url.$remotes/.insteadOf" "$forge"

# scratch: a checkout that [discover] ignore hides.
g init -q "$src/scratch"
commit "$src/scratch" README "scratch: initial"

# The config repo. grove reads HEAD; the working tree has an uncommitted edit that grove must not see.
ws="$HOME/.workstation"
g init -q "$ws"
mkdir -p "$ws/grove"
cat >"$ws/grove/grove.toml" <<'TOML'
version = 1

[discover]
scan = ["~/src/*", "~/src/worktrees/*/*"]
ignore = ["~/src/scratch"]

[tags]
tools = ["alpha", "beta"]
infra = ["gamma"]

[hosts.testhost]
production = ["gamma"]
TOML
g -C "$ws" add grove/grove.toml
g -C "$ws" commit -q -m "grove: config"
printf '\n# draft, not committed\n' >>"$ws/grove/grove.toml"

[ "$estate" = estate ] || exit 0

# ---- The estate: providers with signed release tags, the repositories that pin them, and hosts. ----
#
#   home/src/kit     provider: signed v1.0.0, v1.1.0; v1.2.0 is an unsigned tag (not a release)
#   home/src/tool    provider: signed v0.1.0, v0.2.0
#   home/src/app     consumer (status/1): kit at v1.0.0, behind
#   home/src/site    consumer (status/1): kit at v1.1.0 through its mirror URL (an alias), plus a pattern whose
#                    provider grove has no checkout of
#   home/.workstation  consumer (flake.lock): tool at v0.1.0 (activated on testhost), then v0.2.0 (HEAD)
#   home/.config/grove/activated/.workstation   testhost activated the older commit
#   home/.local/state/grove/hosts/mac.json      a signed report: mac activated HEAD
#   home/.local/state/grove/hosts/old-mac.json  a report with no signature
#   stubs/tuteur     prints stubs/status/NAME.json for `tuteur status -C .../NAME --json`

keys="$root/keys"
mkdir -p "$keys"
ssh-keygen -q -t ed25519 -N '' -C fixture -f "$keys/release"
ssh-keygen -q -t ed25519 -N '' -C mac -f "$keys/mac"

# provider NAME TAG... : a remote with one commit per tag; a tag ending in `!` is left unsigned (lightweight).
provider() {
  local name=$1 seed="$root/seed-$1"
  shift
  g init -q "$seed"
  for tag in "$@"; do
    commit "$seed" CHANGES "$name ${tag%!}"
    if [ "${tag%!}" != "$tag" ]; then
      g -C "$seed" tag "${tag%!}"
    else
      g -C "$seed" -c gpg.format=ssh -c user.signingkey="$keys/release" tag -s -m "$name ${tag}" "$tag"
    fi
  done
  g init -q --bare "$remotes/$name.git"
  g -C "$seed" push -q --tags "$remotes/$name.git" main
  rm -rf "$seed"
  clone "$name"
}
provider kit v1.0.0 v1.1.0 'v1.2.0!'
provider tool v0.1.0 v0.2.0

rev() { g -C "$src/$1" rev-parse "$2^{commit}"; }
kit_v1_0=$(rev kit v1.0.0)
kit_v1_1=$(rev kit v1.1.0)
tool_v0_1=$(rev tool v0.1.0)
tool_v0_2=$(rev tool v0.2.0)

# consumer NAME: a local repository with a .gitmodules, which is what makes grove ask tuteur about it.
consumer() {
  g init -q "$src/$1"
  printf '[submodule "patterns/kit"]\n\tpath = patterns/kit\n\turl = %s\n' "$2" >"$src/$1/.gitmodules"
  g -C "$src/$1" add .gitmodules
  g -C "$src/$1" commit -q -m "$1: patterns"
}
consumer app "${forge}kit.git"
consumer site https://mirror.example.com/daniel/kit

# status/1 as tuteur would print it for each consumer.
mkdir -p "$root/stubs/status"
scion() { # NAME URL TAG COMMIT
  printf '{"name": "%s", "path": "patterns/%s", "url": "%s", "registered": true, "commit": "%s", "tag": "%s",
    "tag_from": "upstream", "filled": true, "clean": true, "latest": {"status": "current"}, "problems": []}' \
    "$1" "$1" "$2" "$4" "$3"
}
status() { # NAME SCIONS
  printf '{"version": 1, "repo": "%s", "scions_dir": "patterns",
  "graft": {"url": null, "commit": null, "tag": null, "latest": {"status": "untagged"}},
  "scions": [%s], "problems": []}\n' "$src/$1" "$2" >"$root/stubs/status/$1.json"
}
status app "$(scion kit "${forge}kit.git" v1.0.0 "$kit_v1_0")"
status site "$(scion kit https://mirror.example.com/daniel/kit v1.1.0 "$kit_v1_1"),
  $(scion lib https://github.com/someone/lib v3.0.0 0123456789abcdef0123456789abcdef01234567)"
cat >"$root/stubs/tuteur" <<SH
#!/bin/sh
# tuteur status -C DIR --json, from the fixture's status/1 files.
[ "\$1" = status ] || exit 2
exec $(command -v cat) "$root/stubs/status/\${3##*/}.json"
SH
chmod +x "$root/stubs/tuteur"

# The config repository: grove.toml for the estate, the hosts' report signers, and two flake.lock commits.
g -C "$ws" checkout -q -- grove/grove.toml
sed 's|^scan = .*|scan = ["~/src/*", "~/src/worktrees/*/*", "~/.workstation"]|' "$ws/grove/grove.toml" >"$root/grove.toml"
mv "$root/grove.toml" "$ws/grove/grove.toml"
cat >>"$ws/grove/grove.toml" <<'TOML'

[hosts.testhost.after]
".workstation" = "home-manager switch --flake ~/.workstation"

[hosts.mac.after]
".workstation" = "sudo darwin-rebuild switch --flake ~/.workstation"

[aliases]
kit = ["https://mirror.example.com/daniel/kit"]
TOML
printf 'mac namespaces="grove-report" %s\n' "$(cat "$keys/mac.pub")" >"$ws/grove/allowed_signers"
lock() { # TOOL_TAG TOOL_REV
  cat >"$ws/flake.lock" <<JSON
{
  "nodes": {
    "nixpkgs": {
      "locked": { "owner": "NixOS", "repo": "nixpkgs", "rev": "d3b13ad75cc492690728f9307e443dec9ef9a20e", "type": "github" },
      "original": { "owner": "NixOS", "ref": "nixpkgs-unstable", "repo": "nixpkgs", "type": "github" }
    },
    "root": { "inputs": { "nixpkgs": "nixpkgs", "tool": "tool" } },
    "tool": {
      "locked": { "ref": "refs/tags/$1", "rev": "$2", "type": "git", "url": "${forge}tool.git" },
      "original": { "ref": "refs/tags/$1", "type": "git", "url": "${forge}tool.git" }
    }
  },
  "root": "root",
  "version": 7
}
JSON
  g -C "$ws" add -A
  g -C "$ws" commit -q -m "tool $1"
}
lock v0.1.0 "$tool_v0_1"
activated=$(g -C "$ws" rev-parse HEAD)
lock v0.2.0 "$tool_v0_2"
head=$(g -C "$ws" rev-parse HEAD)

# testhost (where the tests run) activated the older commit: it owes a switch.
mkdir -p "$HOME/.config/grove/activated"
printf '%s\n' "$activated" >"$HOME/.config/grove/activated/.workstation"

# Reports from two Macs, as `grove here --report` writes them; mac's is signed, old-mac's is not.
hosts="$HOME/.local/state/grove/hosts"
mkdir -p "$hosts"
report() { # HOST COMMIT TAG REV
  printf '{
  "schema": "grove/1",
  "command": "here",
  "host": "%s",
  "observed_at": "2026-11-01T12:00:00Z",
  "configs": [
    {
      "repo": ".workstation",
      "activated": "%s",
      "pins": [
        { "via": "flake input tool", "url": "%stool.git", "tag": "%s", "commit": "%s" }
      ]
    }
  ]
}\n' "$1" "$2" "$forge" "$3" "$4" >"$hosts/$1.json"
}
report mac "$head" v0.2.0 "$tool_v0_2"
ssh-keygen -q -Y sign -n grove-report -f "$keys/mac" "$hosts/mac.json"
report old-mac "$activated" v0.1.0 "$tool_v0_1"
