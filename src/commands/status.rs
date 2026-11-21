//! `grove status [ADDRESS]`: everything grove knows about one repository, per checkout and for the repository,
//! each fact with its source and age. The default address is `.`.

pub fn run(_ctx: &crate::context::Context, address: &str) -> crate::error::Result<crate::exit::Exit> {
    Err(crate::error::Error::new("not built yet"))
}
