//! Running git, read only.
//!
//! Every call goes through here with `--no-optional-locks`, `GIT_TERMINAL_PROMPT=0` and no pager. Reads:
//! `status --porcelain=v2 --branch` (branch, upstream, ahead/behind, tree), `config --get-regexp ^remote\.`
//! (raw remote URLs, before `insteadOf`), `worktree list --porcelain`, and the mtime of `FETCH_HEAD`.
//!
//! The only command that changes anything is `fetch` (for `grove sync`), with
//! `-c gc.auto=0 -c maintenance.auto=false`. It updates remote-tracking refs only.
