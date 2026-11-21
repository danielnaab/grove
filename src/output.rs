//! Rendering: human tables and the `grove/1` JSON envelope.
//!
//! - Human: columns padded by character count and separated by two spaces, no trailing whitespace, `~` for the
//!   home directory, and a summary line. Color only on a terminal without `NO_COLOR`.
//! - JSON: pretty-printed, `{"schema": "grove/1", "command": ...}` first. Absolute paths.
