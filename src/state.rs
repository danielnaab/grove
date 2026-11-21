//! Machine state: `$XDG_STATE_HOME/grove` (default `~/.local/state/grove`). It is safe to delete.
//!
//! G0 keeps only the last `grove sync` outcome per remote (`sync.json`: ok or failed, message, time), so
//! `grove status` can show a failed fetch and its age. Losing the file loses nothing that git does not also know.
