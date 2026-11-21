//! One module per command. Each takes resolved repositories and returns a result for `output` and an exit code.
//! Specified by `docs/reference/cli/<command>.md`.

mod config_check;
mod ls;
mod open;
mod status;
mod sync;
