//! Exit codes, the stable part of grove's interface for agents.
//!
//! `Ok` = 0, `Error` = 1, `Partial` = 3 (output valid, some sources unreadable or remotes failed).
//! 10 ("needs you") is reserved and no G0 command returns it. 125 belongs to pot and is never produced.
//!
//! A command computes its outcome from its results. Usage errors map to `Error`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Exit {
    Ok,
    Error,
    Partial,
}

impl From<Exit> for std::process::ExitCode {
    fn from(exit: Exit) -> Self {
        std::process::ExitCode::from(match exit {
            Exit::Ok => 0,
            Exit::Error => 1,
            Exit::Partial => 3,
        })
    }
}
