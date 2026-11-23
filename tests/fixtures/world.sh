#!/usr/bin/env bash
# Build grove's fixture world: a throwaway HOME with checkouts in every state grove reports.
#
#   tests/fixtures/world.sh DIR     # DIR must be empty or absent
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
# Every commit has a fixed author, committer and date, so commit IDs are the same on every run.
set -euo pipefail

root=${1:?usage: world.sh DIR}
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
