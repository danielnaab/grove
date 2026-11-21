//! `grove open ADDRESS [--print]`: open the forge page with `$BROWSER`, else `xdg-open` (Linux) or `open`
//! (macOS). `--print` prints the URL instead.

pub fn run(_ctx: &crate::context::Context, address: &str, print: bool) -> crate::error::Result<crate::exit::Exit> {
    Err(crate::error::Error::new("not built yet"))
}
