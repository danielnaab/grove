//! grove.toml: load, validate, explain.
//!
//! - Read `grove/grove.toml` from the committed tree of `~/.workstation` (`git show HEAD:grove/grove.toml`),
//!   never the working tree. `config check --file PATH` reads a draft and says that grove does not use it.
//! - Schema `version = 1`: `[discover] scan` / `ignore` (globs, `~` expanded), `[tags] NAME = [ADDRESS, ...]`,
//!   `[hosts.HOST] production = [ADDRESS, ...]`. Unknown keys are errors that name the key and its line.
//! - Every value keeps its span, so `config check` can say which file and line set it.
//! - Order: TOML syntax, then the secret scan (`secrets`), then the schema. A secret is reported even inside an
//!   unknown key, and its value is never printed.
//! - No committed grove.toml: use the defaults (`scan = ["~/src/*", "~/src/worktrees/*/*"]`) and say so.

mod secrets;
