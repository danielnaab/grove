//! Addresses: how a person names repositories. There is no query language.
//!
//! - `name`: the directory name. It is an error when ambiguous, and the error lists `owner/name` choices.
//! - `owner/name`: from the forge remote.
//! - a path, or `.`: the repository that contains it, for any of its worktrees.
//! - `@group`: a `[tags]` group, or a derived one: `@dirty` (any checkout has changes), `@unpushed` (a checkout's
//!   branch is ahead of its upstream, or has no upstream while the repository has a remote), `@behind`.
//!   (`@consumers:P` comes later.)
