use crate::buildenv;
use color_eyre::Result;
use xshell::Shell;

pub fn create(sh: &Shell) -> Result<()> {
    buildenv::setup(sh)
}

pub fn clean(sh: &Shell) -> Result<()> {
    buildenv::clean(sh)
}
