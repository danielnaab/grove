//! `grove sync [ADDRESS]`: `git fetch` every remote of every addressed repository in parallel. It never merges.
//! Production repositories on this host are skipped. A failed remote makes the exit code 3. Outcomes are recorded
//! in `state`.

pub fn run(_ctx: &crate::context::Context, address: Option<&str>) -> crate::error::Result<crate::exit::Exit> {
    Err(crate::error::Error::new("not built yet"))
}
