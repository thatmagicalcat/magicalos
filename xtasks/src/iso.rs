use std::{path::Path, process::Command};

use xtasks::*;

pub fn create() -> Result<(), DynError> {
    println!("[xtask]: Building ISO image...");

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to find project root");

    let iso_root = project_root.join("build/isodir");
    let iso_output = project_root.join("build/magical.iso");

    Command::new("xorriso")
        .args([
            "-as",
            "mkisofs",
            "-R",
            "-r",
            "-J",
            "-b",
            "boot/limine/limine-bios-cd.bin",
            "-no-emul-boot",
            "-boot-load-size",
            "4",
            "-boot-info-table",
            "-hfsplus",
            "-apm-block-size",
            "2048",
            "--efi-boot",
            "boot/limine/limine-uefi-cd.bin",
            "--efi-boot-part",
            "--efi-boot-image",
            "--protective-msdos-label",
            iso_root.to_str().unwrap(),
            "-o",
            iso_output.to_str().unwrap(),
        ])
        .status()?
        .early_ret()?;

    Ok(())
}

pub fn clean() -> Result<(), DynError> {
    println!("[xtask]: Cleaning ISO image...");

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to find project root");
    let iso_output = project_root.join("build/magical.iso");

    if iso_output.exists() {
        std::fs::remove_file(iso_output)?;
    }

    Ok(())
}
