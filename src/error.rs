//! grove's error type and how errors are shown.
//!
//! Every error carries what failed (naming the repository, file or key) and either a `fix` (a command that fixes
//! it) or a `try` (a command that shows more). Human form, on stderr:
//!
//! ```text
//! grove: no repository named zeta
//!   try: grove ls
//! ```
//!
//! JSON form, on stdout: `{"schema": "grove/1", "command": ..., "error": {"message": ..., "fix"|"try": ...}}`.

use std::fmt;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Error {
    pub message: String,
    /// Lines under the message, such as one per problem found.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    #[serde(rename = "try", skip_serializing_if = "Option::is_none")]
    pub try_: Option<String>,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Error { message: message.into(), details: Vec::new(), fix: None, try_: None }
    }

    #[must_use]
    pub fn fix(mut self, command: impl Into<String>) -> Self {
        self.fix = Some(command.into());
        self
    }

    #[must_use]
    pub fn try_(mut self, command: impl Into<String>) -> Self {
        self.try_ = Some(command.into());
        self
    }

    #[must_use]
    pub fn details(mut self, details: Vec<String>) -> Self {
        self.details = details;
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "grove: {}", self.message)?;
        for detail in &self.details {
            write!(f, "\n  {detail}")?;
        }
        if let Some(fix) = &self.fix {
            write!(f, "\n  fix: {fix}")?;
        }
        if let Some(try_) = &self.try_ {
            write!(f, "\n  try: {try_}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}
