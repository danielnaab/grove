//! Pins: one repository or host naming one commit of another repository.
//!
//! A pin comes from a tuteur pattern or tuteur itself (`status/1`, read in `sources::tuteur`), or from a flake
//! input (`flake.lock`, read here: the root's direct inputs, `git` and `github` types). A host report carries
//! pins in the same shape (`via`, `url`, `tag`, `commit`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How a pin is held, which says how to move it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Via {
    /// A tuteur pattern (a submodule), by name.
    Pattern(String),
    /// tuteur itself, pinned by the repository.
    Tuteur,
    /// A flake input, by name; `true` when its URL names a tag (`ref=refs/tags/...`).
    Flake(String, bool),
    /// From a host report: moved by the host's `after` command.
    #[default]
    Report,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pin {
    /// For people: `pattern kit`, `tuteur`, `flake input tool`.
    pub via: String,
    pub url: String,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(skip)]
    pub how: Via,
}

impl Pin {
    pub fn new(how: Via, url: impl Into<String>, tag: Option<String>, commit: Option<String>) -> Pin {
        let via = match &how {
            Via::Pattern(name) => format!("pattern {name}"),
            Via::Tuteur => "tuteur".into(),
            Via::Flake(name, _) => format!("flake input {name}"),
            Via::Report => "report".into(),
        };
        Pin {
            via,
            url: url.into(),
            tag,
            commit,
            how,
        }
    }
}

/// `v1.2.3` as `(1, 2, 3)`. Anything else (a suffix, a missing part) is not a release version.
pub fn version(tag: &str) -> Option<(u64, u64, u64)> {
    let mut parts = tag.strip_prefix('v')?.split('.');
    let mut next = || -> Option<u64> {
        let part = parts.next()?;
        (!part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())).then(|| part.parse().ok())?
    };
    let v = (next()?, next()?, next()?);
    parts.next().is_none().then_some(v)
}

/// The pins in a `flake.lock`: the root node's direct inputs of type `git` or `github`. Inputs that follow
/// another input's (`["a", "b"]`) are not the root's own pins.
pub fn from_flake_lock(text: &str) -> Result<Vec<Pin>, String> {
    let lock: Value = serde_json::from_str(text).map_err(|e| format!("flake.lock is not JSON: {e}"))?;
    let nodes = lock["nodes"].as_object().ok_or("flake.lock has no nodes")?;
    let root = lock["root"].as_str().unwrap_or("root");
    let Some(inputs) = nodes.get(root).and_then(|r| r["inputs"].as_object()) else {
        return Ok(Vec::new());
    };
    let mut pins = Vec::new();
    for (name, node) in inputs {
        let Some(node) = node.as_str().and_then(|n| nodes.get(n)) else {
            continue;
        };
        let (locked, original) = (&node["locked"], &node["original"]);
        let url = match locked["type"].as_str() {
            Some("git") => locked["url"].as_str().map(ToString::to_string),
            Some("github") => match (locked["owner"].as_str(), locked["repo"].as_str()) {
                (Some(owner), Some(repo)) => Some(format!("https://github.com/{owner}/{repo}")),
                _ => None,
            },
            _ => None,
        };
        let Some(url) = url else { continue };
        let reference = locked["ref"].as_str().or_else(|| original["ref"].as_str());
        let tag = reference.and_then(|r| {
            r.strip_prefix("refs/tags/")
                .or_else(|| version(r).map(|_| r))
                .map(ToString::to_string)
        });
        let commit = locked["rev"].as_str().map(ToString::to_string);
        let by_tag = tag.is_some();
        pins.push(Pin::new(Via::Flake(name.clone(), by_tag), url, tag, commit));
    }
    Ok(pins)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(version("v1.20.3"), Some((1, 20, 3)));
        for bad in ["1.2.3", "v1.2", "v1.2.3-rc.1", "v1.2.3.4", "v1..3", "main"] {
            assert_eq!(version(bad), None, "{bad}");
        }
    }

    #[test]
    fn flake_lock_root_inputs() {
        let lock = r#"{"nodes": {
            "root": {"inputs": {"grove": "grove", "nixpkgs": "nixpkgs_2", "hm": ["x", "nixpkgs"], "local": "local"}},
            "grove": {"locked": {"type": "git", "url": "https://github.com/danielnaab/grove.git",
                                 "ref": "refs/tags/v0.1.0", "rev": "e4947eb"}},
            "nixpkgs_2": {"locked": {"type": "github", "owner": "NixOS", "repo": "nixpkgs", "rev": "d3b13ad"},
                          "original": {"type": "github", "owner": "NixOS", "repo": "nixpkgs", "ref": "nixos-unstable"}},
            "local": {"locked": {"type": "path", "path": "/x"}}
        }, "root": "root", "version": 7}"#;
        let pins = from_flake_lock(lock).unwrap();
        assert_eq!(
            pins,
            [
                Pin::new(
                    Via::Flake("grove".into(), true),
                    "https://github.com/danielnaab/grove.git",
                    Some("v0.1.0".into()),
                    Some("e4947eb".into())
                ),
                Pin::new(
                    Via::Flake("nixpkgs".into(), false),
                    "https://github.com/NixOS/nixpkgs",
                    None,
                    Some("d3b13ad".into())
                ),
            ]
        );
        assert!(from_flake_lock("{").is_err());
    }
}
