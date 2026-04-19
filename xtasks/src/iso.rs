use crate::buildenv;
use color_eyre::Result;
use xshell::Shell;

pub fn create(sh: &Shell, quiet: bool) -> Result<()> {
    buildenv::setup(sh, quiet)
}

pub fn clean(sh: &Shell) -> Result<()> {
    buildenv::clean(sh)
}
