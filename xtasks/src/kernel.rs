use color_eyre::Result;
use xshell::{Shell, cmd};
use xtasks::project_root;

pub fn build(sh: &Shell, quiet: bool) -> Result<()> {
    println!("[xtask]: Building the kernel...");

    let root = project_root();
    let _dir = sh.push_dir(root.join("kernel"));

    if quiet {
        cmd!(sh, "cargo build -r -q").run()?;
    } else {
        cmd!(sh, "cargo build -r").run()?;
    }

    Ok(())
}

pub fn clean(sh: &Shell) -> Result<()> {
    println!("[xtask]: Cleaning the kernel...");

    let root = project_root();
    let _dir = sh.push_dir(root.join("kernel"));

    cmd!(sh, "cargo clean").run()?;

    Ok(())
}
