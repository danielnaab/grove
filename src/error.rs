//! grove's error type and how errors are shown.
//!
//! Every error carries what failed (naming the repository, file or key) and either a `fix` (a command that fixes
//! it) or a `try` (a command that shows more). Human form:
//!
//! ```text
//! grove: no repository named zeta
//!   try: grove ls
//! ```
//!
//! JSON form: `{"schema": "grove/1", "command": ..., "error": {"message": ..., "fix"|"try": ...}}` on stdout.
