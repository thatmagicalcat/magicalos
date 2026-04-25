use color_eyre::eyre::{eyre, Result};
use std::env;
use xshell::Shell;

mod buildenv;
mod mlibc;
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

    let quiet = args.any(|arg| arg == "-q" || arg == "--quiet");
    let sh = Shell::new()?;

    match command.as_str() {
        "k" | "kernel" => kernel::build(&sh, quiet)?,
        "s" | "setup" => buildenv::setup(&sh, quiet)?,
        "q" | "qemu" => qemu::run(&sh)?,
        "t" | "test" => test::run(&sh, quiet)?,
        "mlibc" => mlibc::setup(&sh, quiet)?,

        "make" => {
            kernel::build(&sh, quiet)?;
            buildenv::setup(&sh, quiet)?;
        }

        "r" | "run" => {
            kernel::build(&sh, quiet)?;
            buildenv::setup(&sh, quiet)?;
            qemu::run(&sh)?;
        }

        "clean" => {
            kernel::clean(&sh)?;
            buildenv::clean(&sh)?;
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
    eprintln!("[xtask]:     setup, s  - setup build environment & make ISO");
    eprintln!("[xtask]:     test, t   - build and run kernel tests");
    eprintln!("[xtask]:     run, r    - build & run the OS in QEMU");
    eprintln!("[xtask]:     qemu, q   - run the ISO in QEMU");
    eprintln!("[xtask]:     mlibc     - build and setup mlibc");
    eprintln!("[xtask]:     clean     - remove all the build artifacts");
    eprintln!("[xtask]: Flag(s):");
    eprintln!("[xtask]:     --quiet/q - supress build script messages");
}

fn clean(sh: &Shell) -> Result<()> {
    let root = project_root();
    sh.change_dir(root);
    xshell::cmd!(sh, "cargo clean").run()?;
    Ok(())
}
