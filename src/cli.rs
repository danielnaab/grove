//! The command line, declared with clap (derive).
//!
//! - Global flags: `--json`.
//! - `ls [ADDRESS] [--path]`, `status [ADDRESS]`, `sync [ADDRESS]`, `open ADDRESS [--print]`,
//!   `config check [--file PATH]`.
//! - Parse with `try_parse`. `--help` and `--version` exit 0, and every usage error exits 1, not clap's 2
//!   (see `exit`).
