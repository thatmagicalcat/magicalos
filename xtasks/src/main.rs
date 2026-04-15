#![allow(unused)]

use std::env;

mod buildenv;
mod iso;
mod kernel;
mod qemu;

use xtasks::*;

fn main() -> Result<(), DynError> {
    let mut args = env::args();
    _ = args.next().expect("Program name");

    let Some(command) = args.next() else {
        print_usage();
        return Err("Missing command".into());
    };

    match command.as_str() {
        "k" | "kernel" => kernel::build()?,
        "s" | "setup" => buildenv::setup()?,
        "iso" => iso::create()?,

        "make" => {
            kernel::build()?;
            buildenv::setup()?;
        }

        "r" | "run" => {
            kernel::build()?;
            buildenv::setup()?;
            qemu::run()?;
        }

        "clean" => {
            // kernel::clean()?;
            iso::clean()?;
            clean()?;
        }

        "h" | "help" => print_usage(),

        _ => panic!("Unknown command: {command}"),
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
    eprintln!("[xtask]:     run, r    - run the OS in QEMU");
    eprintln!("[xtask]:     clean     - remove all the build artifacts");
}

fn clean() -> Result<(), DynError> {
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to find project root");
    env::set_current_dir(project_root)?;
    std::process::Command::new("cargo")
        .arg("clean")
        .status()?
        .early_ret()?;
    Ok(())
}
