use std::{path::Path, process::Command};

use xtasks::*;

pub fn run() -> Result<(), DynError> {
    println!("[xtask]: Running OS in QEMU...");

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to find project root");
    let iso_path = project_root.join("build/magical.iso");

    Command::new("qemu-system-x86_64")
        .args([
            "-no-reboot",
            "-cdrom",
            iso_path.to_str().unwrap(),
            "-m",
            "2G",
            "-vga",
            "virtio",
            "-enable-kvm",
            "-debugcon",
            "stdio",
            "-cpu",
            "host",
            "-display",
            "gtk,zoom-to-fit=off,show-menubar=off",
        ])
        .status()?
        .early_ret()?;

    Ok(())
}
