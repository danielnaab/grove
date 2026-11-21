//! `grove open ADDRESS [--print]`: open the forge page with `$BROWSER`, else `xdg-open` (Linux) or `open`
//! (macOS). `--print` prints the URL instead.

use std::process::{Command, Stdio};

use serde::Serialize;

use super::Setup;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::exit::Exit;
use crate::output;
use crate::sources::on_path;

const OPENER: &str = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };

pub fn run(ctx: &Context, address: &str, print: bool) -> Result<Exit> {
    let setup = Setup::load(ctx)?;
    let repo = &setup.repos[setup.one(ctx, address, "open")?];
    let try_status = format!("grove status {}", repo.name);
    let Some(remote) = repo.primary_remote() else {
        return Err(Error::new(format!("{} has no remote, so it has no forge page", repo.name)).try_(try_status));
    };
    let Some(forge) = &repo.forge else {
        return Err(Error::new(format!(
            "{}'s remote {} is not a forge URL, so it has no forge page",
            repo.name, remote.name
        ))
        .try_(try_status));
    };

    if !print {
        browse(&forge.url).map_err(|e| e.try_(format!("grove open {address} --print")))?;
    }
    if ctx.json {
        #[derive(Serialize)]
        struct Body<'a> {
            repo: &'a str,
            url: &'a str,
            opened: bool,
        }
        output::print_json("open", Body { repo: &repo.name, url: &forge.url, opened: !print });
    } else if print {
        println!("{}", forge.url);
    } else {
        println!("opened {}", forge.url);
    }
    Ok(Exit::Ok)
}

/// Open `url` with `$BROWSER` (its first entry; `%s` is the URL), else the platform's opener.
fn browse(url: &str) -> Result<()> {
    let browser = std::env::var("BROWSER").unwrap_or_default();
    let mut command = if let Some(entry) = browser.split(':').find(|e| !e.trim().is_empty()) {
        let mut words = entry.split_whitespace().map(|w| w.replace("%s", url));
        let mut command = Command::new(words.next().unwrap_or_default());
        let args: Vec<String> = words.collect();
        let has_url = entry.contains("%s");
        command.args(args);
        if !has_url {
            command.arg(url);
        }
        command
    } else {
        let opener = on_path(OPENER)
            .ok_or_else(|| Error::new(format!("cannot open a browser: BROWSER is unset and {OPENER} is not on PATH")))?;
        let mut command = Command::new(opener);
        command.arg(url);
        command
    };
    let status = command
        .stdin(Stdio::null())
        .status()
        .map_err(|e| Error::new(format!("cannot open a browser: {e}")))?;
    if status.success() { Ok(()) } else { Err(Error::new(format!("cannot open a browser: it exited with {status}"))) }
}
