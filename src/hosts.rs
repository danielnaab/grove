//! Hosts: what each host activated of its config repositories, and the pins at that commit.
//!
//! - A host's config repositories are the keys of `hosts.HOST.after` in grove.toml.
//! - This host: the activated commit of REPO is in `$XDG_CONFIG_HOME/grove/activated/REPO` (default
//!   `~/.config`), which the host's config writes when it is activated. A trailing `-dirty` (a flake's
//!   `dirtyRev`) is dropped. The pins are `flake.lock` at that commit, read from the local checkout.
//! - Other hosts: `$XDG_STATE_HOME/grove/hosts/HOST.json`, written by `grove here --report` on HOST and copied
//!   there with its signature, `HOST.json.sig` (`ssh-keygen -Y sign -n grove-report`). grove verifies it with
//!   `ssh-keygen -Y verify` against `grove/allowed_signers` in `~/.workstation` at HEAD, principal HOST, and
//!   checks that the report names HOST. A report that fails is an unknown fact, never a guess.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::config::{After, Config};
use crate::context::Context;
use crate::discover::Repo;
use crate::fact::Fact;
use crate::git;
use crate::output;
use crate::pins::{self, Pin};

/// The namespace of a report's signature (`ssh-keygen -Y sign -n grove-report`).
pub const NAMESPACE: &str = "grove-report";
/// Where the signers of host reports are listed, in the config repository at HEAD.
pub const SIGNERS: &str = "grove/allowed_signers";

/// One config repository on one host: the commit it activated and the pins at that commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    /// The repository, as `hosts.HOST.after` names it.
    pub repo: String,
    #[serde(default)]
    pub activated: Option<String>,
    /// Why `activated` (or the pins) could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unknown: Option<String>,
    #[serde(default)]
    pub pins: Vec<Pin>,
}

/// A host and its configs, as a fact: from this host's own files, or from its report.
pub struct Host {
    pub name: String,
    pub configs: Fact<Vec<HostConfig>>,
    /// The report could not be read at all (not JSON, unreadable): the command's output is partial.
    pub unreadable: bool,
}

/// What `grove here --report` prints.
#[derive(Serialize, Deserialize)]
pub struct Report {
    pub host: String,
    pub observed_at: String,
    pub configs: Vec<HostConfig>,
}

/// This host's config repository REPO, as `grove here` shows it.
pub struct Local<'a> {
    pub after: &'a After,
    pub repo: Option<&'a Repo>,
    /// The activated commit, from the activated file.
    pub activated: Fact<String>,
    pub head: Option<String>,
    /// Pins at the activated commit and at HEAD.
    pub at_activated: Result<Vec<Pin>, String>,
    pub at_head: Result<Vec<Pin>, String>,
}

pub fn activated_file(ctx: &Context, repo: &str) -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| ctx.home.join(".config"))
        .join("grove/activated")
        .join(repo)
}

/// The repository an `after` key names: by name, `owner/name` or path.
pub fn find_repo<'a>(ctx: &Context, repos: &'a [Repo], address: &str) -> Option<&'a Repo> {
    let path = git::canonical(&ctx.expand(address));
    repos
        .iter()
        .find(|r| r.name == address || r.forge.as_ref().is_some_and(|f| f.slug == address) || r.path == path)
}

fn flake_pins(repo: &Repo, commit: &str) -> Result<Vec<Pin>, String> {
    match git::show(&repo.path, commit, "flake.lock") {
        Ok(Some(text)) => pins::from_flake_lock(&text),
        Ok(None) => Ok(Vec::new()),
        Err(e) => Err(e.message),
    }
}

/// This host's config repositories (`hosts.THIS.after`), read now.
pub fn local<'a>(ctx: &Context, config: &'a Config, repos: &'a [Repo]) -> Vec<Local<'a>> {
    config
        .after_on(&ctx.host)
        .into_iter()
        .map(|after| {
            let repo = find_repo(ctx, repos, &after.repo);
            let file = activated_file(ctx, &after.repo);
            let source = file.display().to_string();
            let activated = match std::fs::read_to_string(&file) {
                Ok(text) => {
                    let commit = text.trim();
                    let commit = commit.strip_suffix("-dirty").unwrap_or(commit);
                    if commit.is_empty() || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
                        Fact::unknown(
                            format!("{} does not hold a commit", ctx.tilde(&file)),
                            source,
                            ctx.now,
                        )
                    } else {
                        Fact::known(commit.to_string(), source, ctx.now)
                    }
                }
                Err(_) => Fact::unknown(
                    format!(
                        "{} is missing: the host's config writes it when activated",
                        ctx.tilde(&file)
                    ),
                    source,
                    ctx.now,
                ),
            };
            let missing = || format!("grove found no repository {}", after.repo);
            let head = repo.and_then(|r| git::rev_parse(&r.path, "HEAD").ok().flatten());
            let at = |commit: Option<&String>| match (repo, commit) {
                (Some(repo), Some(commit)) => flake_pins(repo, commit),
                (None, _) => Err(missing()),
                (_, None) => Err("no commit".into()),
            };
            Local {
                after,
                repo,
                at_activated: at(activated.get()),
                at_head: at(head.as_ref()),
                activated,
                head,
            }
        })
        .collect()
}

/// This host's report: what `grove here --report` prints.
pub fn report(ctx: &Context, locals: &[Local]) -> Report {
    let configs = locals
        .iter()
        .map(|l| {
            let unknown = match (&l.activated.value, &l.at_activated) {
                (Err(reason), _) | (Ok(_), Err(reason)) => Some(reason.clone()),
                _ => None,
            };
            HostConfig {
                repo: l.after.repo.clone(),
                activated: l.activated.get().cloned(),
                unknown,
                pins: l.at_activated.clone().unwrap_or_default(),
            }
        })
        .collect();
    Report {
        host: ctx.host.clone(),
        observed_at: output::timestamp(ctx.now),
        configs,
    }
}

/// This host, for `grove rollout`, if grove.toml names its config repositories.
pub fn this_host(ctx: &Context, config: &Config, repos: &[Repo]) -> Option<Host> {
    let locals = local(ctx, config, repos);
    let first = locals.first()?;
    let report = report(ctx, &locals);
    let known: Vec<_> = report
        .configs
        .into_iter()
        .filter(|c| c.unknown.is_none())
        .collect();
    let configs = if known.is_empty() {
        let reason = first
            .activated
            .value
            .as_ref()
            .err()
            .cloned()
            .or_else(|| first.at_activated.as_ref().err().cloned())
            .unwrap_or_default();
        Fact::unknown(reason, first.activated.source.clone(), ctx.now)
    } else {
        Fact::known(known, first.activated.source.clone(), ctx.now)
    };
    Some(Host {
        name: ctx.host.clone(),
        configs,
        unreadable: false,
    })
}

/// Every other host's report in the state directory, verified, by host name.
pub fn reports(ctx: &Context) -> Vec<Host> {
    let dir = ctx.state_dir().join("hosts");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let signers = git::show(&ctx.home.join(".workstation"), "HEAD", SIGNERS)
        .ok()
        .flatten();
    files
        .iter()
        .filter_map(|file| {
            let name = file.file_stem()?.to_string_lossy().into_owned();
            (name != ctx.host).then(|| read_report(ctx, file, &name, signers.as_deref()))
        })
        .collect()
}

fn read_report(ctx: &Context, file: &Path, name: &str, signers: Option<&str>) -> Host {
    let mtime = std::fs::metadata(file)
        .and_then(|m| m.modified())
        .unwrap_or(ctx.now);
    let host = |configs, unreadable| Host {
        name: name.to_string(),
        configs,
        unreadable,
    };
    let report: Report = match std::fs::read(file)
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
    {
        Ok(report) => report,
        Err(e) => {
            return host(
                Fact::unknown(format!("report unreadable: {e}"), "report", mtime),
                true,
            );
        }
    };
    let at = output::parse_timestamp(&report.observed_at).unwrap_or(mtime);
    let unknown = |reason: String| {
        host(
            Fact::unknown(format!("report not verified: {reason}"), "report", at),
            false,
        )
    };
    if let Err(reason) = verify(ctx, file, name, signers) {
        return unknown(reason);
    }
    if report.host != name {
        return unknown(format!("it names host {}, not {name}", report.host));
    }
    host(Fact::known(report.configs, "report", at), false)
}

/// `ssh-keygen -Y verify` of `FILE.sig` against the signers' line for `principal`.
fn verify(ctx: &Context, file: &Path, principal: &str, signers: Option<&str>) -> Result<(), String> {
    let Some(signers) = signers else {
        return Err(format!("{SIGNERS} is not in ~/.workstation at HEAD"));
    };
    let listed = signers.lines().any(|line| {
        let line = line.trim();
        !line.starts_with('#')
            && line
                .split_whitespace()
                .next()
                .is_some_and(|p| p.split(',').any(|p| p == principal))
    });
    if !listed {
        return Err(format!("{SIGNERS} has no key for {principal}"));
    }
    let sig = PathBuf::from(format!("{}.sig", file.display()));
    if !sig.exists() {
        return Err(format!("no signature ({} is missing)", ctx.tilde(&sig)));
    }
    let Some(keygen) = crate::sources::on_path("ssh-keygen") else {
        return Err("ssh-keygen not found on PATH".into());
    };
    // ssh-keygen reads the signers from a file: a private copy in the state directory, removed after.
    let dir = ctx.state_dir();
    let copy = dir.join(format!(".allowed_signers.{}", std::process::id()));
    std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&copy, signers))
        .map_err(|e| format!("cannot write {}: {e}", ctx.tilde(&copy)))?;
    let message = std::fs::File::open(file).map_err(|e| e.to_string());
    let status = message.and_then(|message| {
        Command::new(keygen)
            .args(["-Y", "verify", "-n", NAMESPACE, "-I", principal, "-f"])
            .arg(&copy)
            .arg("-s")
            .arg(&sig)
            .stdin(message)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("ssh-keygen could not start: {e}"))
    });
    let _ = std::fs::remove_file(&copy);
    if status?.success() {
        Ok(())
    } else {
        Err(format!("the signature does not match {SIGNERS} for {principal}"))
    }
}
