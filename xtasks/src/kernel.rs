use color_eyre::Result;
use xshell::{cmd, Shell};
use xtasks::project_root;

pub fn build(sh: &Shell) -> Result<()> {
    println!("[xtask]: Building the kernel...");

    let root = project_root();
    let _dir = sh.push_dir(root.join("kernel"));

    cmd!(sh, "cargo build --release").run()?;

    Ok(())
}

pub fn clean(sh: &Shell) -> Result<()> {
    println!("[xtask]: Cleaning the kernel...");

    let root = project_root();
    let _dir = sh.push_dir(root.join("kernel"));

    cmd!(sh, "cargo clean").run()?;

    Ok(())
}
