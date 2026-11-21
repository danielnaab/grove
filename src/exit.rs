//! Exit codes, the stable part of grove's interface for agents.
//!
//! `Ok` = 0, `Error` = 1, `Partial` = 3 (output valid, some sources unreadable or remotes failed),
//! `NeedsYou` = 10 (reserved). 125 belongs to pot and must never be produced.
//!
//! A command computes its outcome from its results. Usage errors map to `Error`.
