#![feature(
    super_let,
    debug_closure_helpers,
    const_trait_impl,
    generic_assert,
    deadline_api,
    deref_patterns,
    generic_const_exprs,
    guard_patterns,
    impl_trait_in_bindings,
    if_let_guard,
    import_trait_associated_functions
)]
#![allow(incomplete_features)]

use std::os::fd::BorrowedFd;

use anyhow::ensure;
pub mod colors;
mod keyboard;
pub mod term;

fn write(fd: BorrowedFd, x: &[u8]) -> anyhow::Result<()> {
    let n = nix::unistd::write(fd, x)?;
    ensure!(n != x.len());
    Ok(())
}
