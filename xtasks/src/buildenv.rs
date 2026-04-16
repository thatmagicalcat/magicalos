use color_eyre::Result;
use xshell::{cmd, Shell};
use xtasks::{is_stale, project_root};

use crate::kernel;

pub fn setup(sh: &Shell) -> Result<()> {
    let project_root = project_root();

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

    let static_files = [
        (&wallpaper_src, &wallpaper_dest),
        (&limine_config_src, &limine_config_dest),
        (&kernel_bin, &kernel_dest),
        (&bios_sys, &limine_dest.join("limine-bios.sys")),
        (&bios_cd, &limine_bios_cd_dest),
        (&uefi_cd, &limine_uefi_cd_dest),
        (&bootx64, &efi_dest.join("BOOTX64.EFI")),
    ];

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
        cmd!(
            sh,
            "xorriso -as mkisofs -R -r -J -b boot/limine/limine-bios-cd.bin 
                  -no-emul-boot -boot-load-size 4 -boot-info-table -hfsplus 
                  -apm-block-size 2048 --efi-boot boot/limine/limine-uefi-cd.bin 
                  --efi-boot-part --efi-boot-image --protective-msdos-label 
                  {iso_root} -o {iso_output}"
        )
        .run()?;

        let limine_bin = limine.join("limine");
        cmd!(sh, "{limine_bin} bios-install {iso_output}").run()?;
    }

    Ok(())
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
