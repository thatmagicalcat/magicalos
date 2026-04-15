use std::{fs, path::Path, process::Command};

use xtasks::*;

use crate::DynError;

pub fn setup() -> Result<(), DynError> {
    println!("[xtask]: Setting up build environment...");

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to find project root");

    // inputs
    let kernel_bin = project_root.join("target/x86_64/release/kernel");
    let limine = project_root.join("limine/Limine/bin");
    let initramfs = project_root.join("initramfs");
    let wallpaper_src = project_root.join("wallpaper.png");
    let limine_config_src = project_root.join("limine.conf");

    // outputs
    let build = project_root.join("build");
    let iso_root = build.join("isodir");
    let limine_dest = iso_root.join("boot/limine");
    let kernel_dest = iso_root.join("boot/kernel");
    let initramfs_dest = iso_root.join("boot/initramfs.tar");
    let efi_dest = iso_root.join("EFI/BOOT");
    let limine_config_dest = limine_dest.join("limine.conf");
    let wallpaper_dest = limine_dest.join("wallpaper.png");
    let iso_output = build.join("magical.iso");

    create_dir(&build)?;
    create_dir(&limine_dest)?;
    create_dir(&efi_dest)?;

    Command::new("tar")
        .arg("cvf")
        .arg(&initramfs_dest)
        .arg("-C")
        .arg(&initramfs)
        .arg(".")
        .status()?
        .early_ret()?;

    fs::copy(&wallpaper_src, &wallpaper_dest)?;
    fs::copy(&limine_config_src, &limine_config_dest)?;
    fs::copy(&kernel_bin, &kernel_dest)?;
    fs::copy(
        limine.join("limine-bios.sys"),
        limine_dest.join("limine-bios.sys"),
    )?;
    fs::copy(
        limine.join("limine-bios-cd.bin"),
        limine_dest.join("limine-bios-cd.bin"),
    )?;
    fs::copy(
        limine.join("limine-uefi-cd.bin"),
        limine_dest.join("limine-uefi-cd.bin"),
    )?;
    fs::copy(limine.join("BOOTX64.EFI"), efi_dest.join("BOOTX64.EFI"))?;

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
        ])
        .arg(&iso_root)
        .arg("-o")
        .arg(&iso_output)
        .status()?
        .early_ret()?;

    Command::new(limine.join("limine"))
        .arg("bios-install")
        .arg(&iso_output)
        .status()?
        .early_ret()?;

    Ok(())
}

pub fn clean() -> Result<(), DynError> {
    println!("[xtask]: Cleaning build environment...");

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to find project root");
    let build = project_root.join("build");

    if build.exists() {
        fs::remove_dir_all(build)?;
    }

    Ok(())
}
