use color_eyre::Result;
use xshell::{cmd, Shell};
use xtasks::project_root;

pub fn run(sh: &Shell) -> Result<()> {
    println!("[xtask]: Running the OS in QEMU...");

    let project_root = project_root();
    let iso_path = project_root.join("build/magical.iso");

    cmd!(
        sh,
        "qemu-system-x86_64
            -no-reboot
            -cdrom {iso_path}
            -m 2G
            -vga
            virtio
            -enable-kvm
            -debugcon stdio
            -cpu host
            -display gtk,zoom-to-fit=off,show-menubar=off
            -device isa-debug-exit,iobase=0xf4,iosize=0x04
        "
    )
    .run()?;

    Ok(())
}
