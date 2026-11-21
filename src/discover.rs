//! Discovery: from `[discover] scan` globs to repositories.
//!
//! 1. Expand each scan glob and drop matches of any `ignore` glob. A directory is a checkout when
//!    `git rev-parse --git-common-dir` succeeds in it. Other directories are skipped silently.
//! 2. Group checkouts by canonical git common dir. One common dir is one repository, and its worktrees are merged
//!    under it.
//! 3. Add worktrees that the scan missed, from `git worktree list --porcelain`.
//! 4. Name each repository by its main checkout's directory name. `owner/name` comes from the forge remote.
