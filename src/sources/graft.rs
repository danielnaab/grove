//! `graft status -C CHECKOUT --json`, contract `status/1` (see `tests/contracts/`).
//!
//! Read leniently: unknown fields are ignored, and a `latest.status` grove does not know counts as nothing.

use serde::{Deserialize, Serialize};

/// What grove shows of a checkout's graft pins.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Pins {
    /// Scions in the checkout.
    pub scions: usize,
    /// Scions with a newer release of their major version.
    pub outdated: usize,
    /// `graft check`'s messages, for the repository and each scion.
    pub problems: usize,
}

impl Pins {
    /// `-`, `2`, `2, 1 outdated`, `2, 1 outdated, 3 problems`.
    pub fn display(&self) -> String {
        if self.scions == 0 && self.problems == 0 {
            return "-".into();
        }
        let mut text = self.scions.to_string();
        if self.outdated > 0 {
            text.push_str(&format!(", {} outdated", self.outdated));
        }
        if self.problems > 0 {
            text.push_str(&format!(", {}", crate::output::plural(self.problems, "problem")));
        }
        text
    }
}

#[derive(Deserialize)]
struct Report {
    version: u64,
    #[serde(default)]
    scions: Vec<Scion>,
    #[serde(default)]
    problems: Vec<serde::de::IgnoredAny>,
}

#[derive(Deserialize)]
struct Scion {
    #[serde(default)]
    latest: Option<Latest>,
    #[serde(default)]
    problems: Vec<serde::de::IgnoredAny>,
}

#[derive(Deserialize)]
struct Latest {
    #[serde(default)]
    status: Option<String>,
}

pub fn parse(json: &[u8]) -> Result<Pins, String> {
    let report: Report = serde_json::from_slice(json).map_err(|e| format!("not status/1 JSON: {e}"))?;
    if report.version != 1 {
        return Err(format!("status/{} is newer than grove reads (status/1)", report.version));
    }
    Ok(Pins {
        scions: report.scions.len(),
        outdated: report
            .scions
            .iter()
            .filter(|s| s.latest.as_ref().and_then(|l| l.status.as_deref()) == Some("outdated"))
            .count(),
        problems: report.problems.len() + report.scions.iter().map(|s| s.problems.len()).sum::<usize>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Pins {
        let path = format!("{}/tests/contracts/status-1/{name}.json", env!("CARGO_MANIFEST_DIR"));
        parse(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn golden_fixtures() {
        assert_eq!(fixture("clean"), Pins { scions: 2, outdated: 1, problems: 1 });
        assert_eq!(fixture("problems"), Pins { scions: 2, outdated: 0, problems: 5 });
        assert_eq!(fixture("offline"), Pins { scions: 2, outdated: 0, problems: 1 });
        assert_eq!(fixture("no-scions"), Pins::default());
        assert_eq!(fixture("clean").display(), "2, 1 outdated, 1 problem");
        assert_eq!(fixture("no-scions").display(), "-");
    }

    #[test]
    fn lenient_but_versioned() {
        let newer = br#"{"version": 1, "scions": [{"latest": {"status": "someday"}, "new": 1}], "extra": true}"#;
        assert_eq!(parse(newer), Ok(Pins { scions: 1, outdated: 0, problems: 0 }));
        assert_eq!(parse(br#"{"version": 2}"#), Err("status/2 is newer than grove reads (status/1)".into()));
        assert!(parse(b"not json").is_err());
    }
}
