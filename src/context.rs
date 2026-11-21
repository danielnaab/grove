//! What every command needs to know about where it runs: home, host, the time, and the output mode.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::{Error, Result};

pub struct Context {
    /// `$HOME`, canonical.
    pub home: PathBuf,
    /// The host name used for `hosts.HOST` (`GROVE_HOST`, else the machine's hostname).
    pub host: String,
    /// When this invocation observes. Facts read during it are `now`.
    pub now: SystemTime,
    pub json: bool,
}

impl Context {
    pub fn from_env(json: bool) -> Result<Context> {
        let home = std::env::var_os("HOME")
            .filter(|h| !h.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| Error::new("HOME is not set").try_("echo $HOME"))?;
        let home = home.canonicalize().unwrap_or(home);
        Ok(Context {
            home,
            host: host(),
            now: SystemTime::now(),
            json,
        })
    }

    /// A path for people: `~/src/beta`.
    pub fn tilde(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => path.display().to_string(),
        }
    }

    /// Expand a leading `~/`.
    pub fn expand(&self, path: &str) -> PathBuf {
        match path.strip_prefix("~/") {
            Some(rest) => self.home.join(rest),
            None if path == "~" => self.home.clone(),
            None => PathBuf::from(path),
        }
    }

    /// `$XDG_STATE_HOME/grove`, default `~/.local/state/grove`.
    pub fn state_dir(&self) -> PathBuf {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| self.home.join(".local/state"))
            .join("grove")
    }
}

fn host() -> String {
    if let Some(host) = std::env::var("GROVE_HOST").ok().filter(|h| !h.is_empty()) {
        return host;
    }
    let from_file = std::fs::read_to_string("/proc/sys/kernel/hostname").ok();
    let from_command = || {
        let out = std::process::Command::new("hostname").output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    };
    from_file
        .or_else(from_command)
        .map(|h| h.trim().split('.').next().unwrap_or_default().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "localhost".into())
}
