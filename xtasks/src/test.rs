use color_eyre::{eyre::eyre, Result};
use std::fs;
use std::process::Command;
use xshell::{cmd, Shell};
use xtasks::project_root;

use crate::buildenv;

pub fn run(sh: &Shell, quiet: bool) -> Result<()> {
    println!("[xtask]: Building kernel test binary...");

    let root = project_root();
    let kernel_dir = root.join("kernel");
    let test_target = root.join("target/x86_64/release/deps/magicalos_kernel");
    let target_dir = root.join("target");

    {
        let _dir = sh.push_dir(&kernel_dir);
        cmd!(
            sh,
            "cargo test --release --target x86_64.json --lib --no-run --target-dir {target_dir}"
        )
        .run()?;
    }

    let test_bin = discover_test_binary(&test_target)?;
    let iso_path = buildenv::setup_for_kernel(sh, &test_bin, "magical-test.iso", quiet)?;

    println!("[xtask]: Running kernel tests in QEMU...");

    let status = Command::new("qemu-system-x86_64")
        .arg("-no-reboot")
        .arg("-cdrom")
        .arg(&iso_path)
        .arg("-m")
        .arg("2G")
        .arg("-vga")
        .arg("virtio")
        .arg("-enable-kvm")
        .arg("-debugcon")
        .arg("stdio")
        .arg("-cpu")
        .arg("host")
        .arg("-display")
        .arg("none")
        .arg("-device")
        .arg("isa-debug-exit,iobase=0xf4,iosize=0x04")
        .status()?;

    match status.code() {
        Some(code) if code == ((0x10u32 << 1) as i32 | 1) => {
            println!("[xtask]: tests passed");
            Ok(())
        }

        Some(code) => Err(eyre!("kernel tests failed with QEMU exit status {code}")),
        None => Err(eyre!("kernel tests failed: QEMU terminated by signal")),
    }
}

fn discover_test_binary(pattern: &std::path::Path) -> Result<std::path::PathBuf> {
    let parent = pattern
        .parent()
        .ok_or_else(|| eyre!("invalid kernel test target directory"))?;
    let prefix = pattern
        .file_name()
        .ok_or_else(|| eyre!("invalid kernel test target prefix"))?
        .to_string_lossy();

    let mut candidates = fs::read_dir(parent)?
        .filter_map(core::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy())
                .is_some_and(|name| name.starts_with(prefix.as_ref()) && !name.ends_with(".d"))
        })
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();

    candidates.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });

    candidates
        .pop()
        .ok_or_else(|| eyre!("unable to locate kernel test binary in target directory"))
}
