use color_eyre::eyre::{eyre, Result};
use std::env;
use xshell::Shell;

mod buildenv;
mod iso;
mod kernel;
mod qemu;
mod test;

use xtasks::*;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut args = env::args();
    _ = args.next().expect("Program name");

    let Some(command) = args.next() else {
        print_usage();
        return Err(eyre!("Missing command"));
    };

    let sh = Shell::new()?;

    match command.as_str() {
        "k" | "kernel" => kernel::build(&sh)?,
        "s" | "setup" => buildenv::setup(&sh)?,
        "q" | "qemu" => qemu::run(&sh)?,
        "t" | "test" => test::run(&sh)?,
        "iso" => iso::create(&sh)?,

        "make" => {
            kernel::build(&sh)?;
            buildenv::setup(&sh)?;
        }

        "r" | "run" => {
            kernel::build(&sh)?;
            buildenv::setup(&sh)?;
            qemu::run(&sh)?;
        }

        "clean" => {
            iso::clean(&sh)?;
            clean(&sh)?;
        }

        "h" | "help" => print_usage(),

        _ => return Err(eyre!("Unknown command: {command}")),
    }

    Ok(())
}

fn print_usage() {
    eprintln!("[xtask]: Usage: cargo x <command>");
    eprintln!("[xtask]: Available commands:");
    eprintln!("[xtask]:     help, h   - prints this message");
    eprintln!("[xtask]:     make      - build & run the entire project");
    eprintln!("[xtask]:     kernel, k - build kernel");
    eprintln!("[xtask]:     setup, s  - setup build environment");
    eprintln!("[xtask]:     iso       - build ISO image");
    eprintln!("[xtask]:     test, t   - build and run kernel tests");
    eprintln!("[xtask]:     run, r    - build & run the OS in QEMU");
    eprintln!("[xtask]:     qemu, q   - run the ISO in QEMU");
    eprintln!("[xtask]:     clean     - remove all the build artifacts");
}

fn clean(sh: &Shell) -> Result<()> {
    let root = project_root();
    sh.change_dir(root);
    xshell::cmd!(sh, "cargo clean").run()?;
    Ok(())
}
