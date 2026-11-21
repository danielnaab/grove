//! A fact: one thing grove knows about a checkout or repository.
//!
//! `Fact<T>` is either known (`value`) or `unknown` (a reason). Both kinds carry `source` (`git`, `git config`,
//! `FETCH_HEAD`, `grove.toml:LINE`, `graft`, `pot`, `grove state`) and `observed_at` (RFC 3339, UTC).
//!
//! Display ages: a fact observed during this invocation shows as `now`. A fact from the state cache shows its
//! real age (`12s ago`, `5m ago`, `3h ago`, `2d ago`).

use std::time::SystemTime;

use serde::ser::{Serialize, SerializeMap, Serializer};

use crate::output;

#[derive(Debug, Clone)]
pub struct Fact<T> {
    pub value: Result<T, String>,
    pub source: String,
    pub observed_at: SystemTime,
}

impl<T> Fact<T> {
    pub fn known(value: T, source: impl Into<String>, observed_at: SystemTime) -> Self {
        Fact {
            value: Ok(value),
            source: source.into(),
            observed_at,
        }
    }

    pub fn unknown(reason: impl Into<String>, source: impl Into<String>, observed_at: SystemTime) -> Self {
        Fact {
            value: Err(reason.into()),
            source: source.into(),
            observed_at,
        }
    }

    pub fn get(&self) -> Option<&T> {
        self.value.as_ref().ok()
    }

    /// The human source column: `git, now`, `grove state, 3m ago`, `graft: not found on PATH, now`.
    pub fn provenance(&self, now: SystemTime) -> String {
        let when = if self.observed_at >= now {
            "now".to_string()
        } else {
            output::age(now, self.observed_at)
        };
        match &self.value {
            Ok(_) => format!("{}, {when}", self.source),
            Err(reason) => {
                let reason = reason
                    .strip_prefix(&format!("{} ", self.source))
                    .unwrap_or(reason);
                format!("{}: {reason}, {when}", self.source)
            }
        }
    }
}

impl<T: Serialize> Serialize for Fact<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        match &self.value {
            Ok(value) => map.serialize_entry("value", value)?,
            Err(reason) => map.serialize_entry("unknown", reason)?,
        }
        map.serialize_entry("source", &self.source)?;
        map.serialize_entry("observed_at", &output::timestamp(self.observed_at))?;
        map.end()
    }
}
