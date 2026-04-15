use std::{env, path::Path, process::Command};

use xtasks::*;

use crate::DynError;

pub fn build() -> Result<(), DynError> {
    println!("[xtask]: Building the kernel...");

    let cwd = env::current_dir()?;
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to find project root");

    env::set_current_dir(project_root.join("kernel"))?;
    Command::new("cargo")
        .args(["build", "--release"])
        .status()?
        .early_ret()?;

    // restore
    env::set_current_dir(cwd)?;

    Ok(())
}

pub fn clean() -> Result<(), DynError> {
    println!("[xtask]: Cleaning the kernel...");

    let cwd = env::current_dir()?;
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to find project root");

    env::set_current_dir(project_root.join("kernel"))?;
    Command::new("cargo").arg("clean").status()?.early_ret()?;

    // restore
    env::set_current_dir(cwd)?;

    Ok(())
}
