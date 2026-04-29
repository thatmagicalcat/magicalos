use color_eyre::Result;
use std::path::{Path, PathBuf};
use xshell::{cmd, Shell};
use xtasks::{is_stale, project_root};

use crate::kernel;

pub fn setup(sh: &Shell, quiet: bool) -> Result<()> {
    let project_root = project_root();
    let kernel_bin = project_root.join("target/x86_64/release/magicalos-kernel");
    _ = setup_for_kernel(sh, &kernel_bin, "magical.iso", quiet)?;
    Ok(())
}

pub fn setup_for_kernel(
    sh: &Shell,
    kernel_bin: &Path,
    iso_name: &str,
    quiet: bool,
) -> Result<PathBuf> {
    let project_root = project_root();

    // inputs
    let limine = project_root.join("limine/Limine");
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
    let iso_output = build.join(iso_name);

    sh.create_dir(&build)?;
    sh.create_dir(&limine_dest)?;
    sh.create_dir(&efi_dest)?;

    // incremental initramfs
    if is_stale(&[&initramfs], &[&initramfs_dest])? {
        println!("[xtask]: Creating initramfs...");
        cmd!(sh, "tar cvf {initramfs_dest} -C {initramfs} .").run()?;
    }

    // Incremental copy of static files
    let bios_cd = limine.join("limine-bios-cd.bin");
    let uefi_cd = limine.join("limine-uefi-cd.bin");
    let bios_sys = limine.join("limine-bios.sys");
    let bootx64 = limine.join("BOOTX64.EFI");

    let limine_bios_cd_dest = limine_dest.join("limine-bios-cd.bin");
    let limine_uefi_cd_dest = limine_dest.join("limine-uefi-cd.bin");
    let limine_bios_sys_dest = limine_dest.join("limine-bios.sys");
    let bootx64_dest = efi_dest.join("BOOTX64.EFI");

    let static_files = [
        (wallpaper_src.as_path(), wallpaper_dest.as_path()),
        (limine_config_src.as_path(), limine_config_dest.as_path()),
        (bios_sys.as_path(), limine_bios_sys_dest.as_path()),
        (bios_cd.as_path(), limine_bios_cd_dest.as_path()),
        (uefi_cd.as_path(), limine_uefi_cd_dest.as_path()),
        (bootx64.as_path(), bootx64_dest.as_path()),
    ];

    sh.copy_file(kernel_bin, &kernel_dest)?;

    for (src, dest) in static_files {
        if is_stale(&[src], &[dest])? {
            sh.copy_file(src, dest)?;
        }
    }

    // Incremental ISO generation
    let iso_inputs = [
        kernel_dest.as_path(),
        initramfs_dest.as_path(),
        limine_config_dest.as_path(),
        wallpaper_dest.as_path(),
        limine_bios_cd_dest.as_path(),
        limine_uefi_cd_dest.as_path(),
    ];

    if is_stale(&iso_inputs, &[&iso_output])? {
        println!("[xtask]: Generating ISO...");
        let mut c = cmd!(
            sh,
            "xorriso -as mkisofs -R -r -J -b boot/limine/limine-bios-cd.bin 
                  -no-emul-boot -boot-load-size 4 -boot-info-table -hfsplus 
                  -apm-block-size 2048 --efi-boot boot/limine/limine-uefi-cd.bin 
                  --efi-boot-part --efi-boot-image --protective-msdos-label 
                  {iso_root} -o {iso_output}"
        );

        if quiet {
            c = c.ignore_stdout().ignore_stderr();
        }

        c.run()?;

        let limine_bin = limine.join("limine");
        let mut c = cmd!(sh, "{limine_bin} bios-install {iso_output}");

        if quiet {
            c = c.ignore_stdout().ignore_stderr();
        }

        c.run()?;
    }

    Ok(iso_output)
}

pub fn clean(sh: &Shell) -> Result<()> {
    println!("[xtask]: Cleaning build environment...");
    kernel::clean(sh)?;

    let build = project_root().join("build");
    if build.exists() {
        sh.remove_path(build)?;
    }

    Ok(())
}
