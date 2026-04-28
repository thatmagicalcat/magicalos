use color_eyre::eyre::{Result, WrapErr, eyre};
use std::io;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use xshell::{Shell, cmd};
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

pub fn debug() -> Result<()> {
    println!("[xtask]: Running the OS in QEMU debug mode...");

    let project_root = project_root();
    let iso_path = project_root.join("build/magical.iso");
    let kernel_bin = project_root.join("target/x86_64/release/magicalos-kernel");

    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-no-reboot")
        .arg("-cdrom")
        .arg(&iso_path)
        .arg("-m")
        .arg("2G")
        .arg("-vga")
        .arg("virtio")
        .arg("-debugcon")
        .arg("file:build/qemu-debug.log")
        .arg("-enable-kvm")
        .arg("-cpu")
        .arg("host")
        .arg("-display")
        .arg("gtk,zoom-to-fit=off,show-menubar=off")
        .arg("-device")
        .arg("isa-debug-exit,iobase=0xf4,iosize=0x04")
        .arg("-s")
        .arg("-S")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let qemu_child = qemu
        .spawn()
        .wrap_err("Failed to start QEMU in debug mode")?;
    let mut qemu_guard = QemuGuard::new(qemu_child);

    println!("[xtask]: QEMU is waiting for GDB on localhost:1234");
    println!("[xtask]: Starting debugger...");

    let status = run_debugger(&kernel_bin)?;
    qemu_guard.stop()?;

    if !status.success() {
        return Err(eyre!("Debugger exited with status: {status}"));
    }

    Ok(())
}

fn run_debugger(kernel_bin: &Path) -> Result<ExitStatus> {
    let mut rust_gdb = build_debugger_command("rust-gdb", kernel_bin);
    match rust_gdb.status() {
        Ok(status) => Ok(status),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            println!("[xtask]: rust-gdb was not found, falling back to gdb...");
            build_debugger_command("gdb", kernel_bin)
                .status()
                .wrap_err("Failed to start gdb")
        }
        Err(err) => Err(err).wrap_err("Failed to start rust-gdb"),
    }
}

fn build_debugger_command(debugger: &str, kernel_bin: &Path) -> Command {
    let mut cmd = Command::new(debugger);
    cmd.arg(kernel_bin)
        .arg("-ex")
        .arg("target remote localhost:1234");
    cmd
}

struct QemuGuard {
    child: Option<Child>,
}

impl QemuGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            if child.try_wait()?.is_none() {
                child.kill().wrap_err("Failed to terminate QEMU")?;
            }
            child.wait().wrap_err("Failed to wait for QEMU")?;
        }

        Ok(())
    }
}

impl Drop for QemuGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}
