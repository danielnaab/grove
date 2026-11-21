//! `grove config check [--file PATH]`: validate grove.toml. List every value with the file and line that set it,
//! and note when the working tree differs from HEAD.

pub fn run(_ctx: &crate::context::Context, file: Option<&std::path::Path>) -> crate::error::Result<crate::exit::Exit> {
    Err(crate::error::Error::new("not built yet"))
}
