//! The fixture world for integration tests: a throwaway HOME built by `tests/fixtures/world.sh`.
//!
//! grove runs with a clean environment pointed at the world, so no test touches the real `~/src`,
//! `~/.workstation` or `~/.local/state`. `PATH` holds only `git` (plus any stubs a test adds), so `tuteur`,
//! `graft`, `pot` and `xdg-open` are absent.
#![allow(dead_code)] // each test binary uses a different part of this module

use std::path::{Path, PathBuf};
use std::process::Command;

pub struct World {
    _dir: tempfile::TempDir,
    /// The world's root: `home/` and `remotes/` live here.
    pub root: PathBuf,
    pub home: PathBuf,
    /// The only directory on grove's `PATH`.
    pub bin: PathBuf,
}

impl World {
    pub fn build() -> World {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let root = dir
            .path()
            .canonicalize()
            .expect("canonical temp dir")
            .join("world");
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/world.sh");
        let status = Command::new("bash")
            .arg(&script)
            .arg(&root)
            .status()
            .expect("bash runs world.sh");
        assert!(status.success(), "world.sh failed");

        let bin = dir.path().join("bin");
        std::fs::create_dir(&bin).expect("bin dir");
        std::os::unix::fs::symlink(which("git"), bin.join("git")).expect("git symlink");

        let home = root.join("home");
        World {
            _dir: dir,
            root,
            home,
            bin,
        }
    }

    /// The environment grove runs with. Every variable that could reach outside the world is set here.
    pub fn env(&self) -> Vec<(&'static str, String)> {
        let home = self.home.display().to_string();
        vec![
            ("HOME", home.clone()),
            ("PATH", self.bin.display().to_string()),
            ("GROVE_HOST", "testhost".into()),
            ("XDG_STATE_HOME", format!("{home}/.local/state")),
            ("XDG_CONFIG_HOME", format!("{home}/.config")),
            ("GIT_CONFIG_NOSYSTEM", "1".into()),
            ("GIT_CONFIG_GLOBAL", "/dev/null".into()),
            ("BROWSER", String::new()),
            ("NO_COLOR", "1".into()),
            ("TZ", "UTC".into()),
            ("LC_ALL", "C".into()),
        ]
    }

    /// `grove ARGS` in the world, from `cwd` (default: HOME).
    pub fn grove(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_grove"));
        cmd.env_clear()
            .envs(self.env())
            .current_dir(&self.home)
            .args(args);
        cmd
    }

    /// Put an executable script named `name` on grove's `PATH`.
    pub fn stub(&self, name: &str, script: &str) {
        use std::os::unix::fs::PermissionsExt;
        let path = self.bin.join(name);
        std::fs::write(&path, script).expect("write stub");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod stub");
    }
}

/// The absolute path of a program on the test's own `PATH`.
pub fn which(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("{program} not found on PATH"))
}

impl World {
    /// Run git in the world (to set up a case), with the world's environment. Panics on failure.
    pub fn git(&self, dir: &Path, args: &[&str]) -> String {
        let out = Command::new(which("git"))
            .env_clear()
            .envs(self.env())
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.org")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.org")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    pub fn src(&self, name: &str) -> PathBuf {
        self.home.join("src").join(name)
    }
}

/// What grove ran: exit code, stdout, stderr.
pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {}", self.stdout))
    }
}

pub fn run(mut cmd: Command) -> Run {
    let out = cmd.output().expect("grove runs");
    Run {
        code: out.status.code().expect("an exit code"),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}
