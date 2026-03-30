use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let level = std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into());
    match level.as_str() {
        "trace" => println!("cargo:rustc-cfg=log_level=\"trace\""),
        "debug" => println!("cargo:rustc-cfg=log_level=\"debug\""),
        "info" => println!("cargo:rustc-cfg=log_level=\"info\""),
        "warn" => println!("cargo:rustc-cfg=log_level=\"warn\""),
        "error" => println!("cargo:rustc-cfg=log_level=\"error\""),
        _ => panic!("invalid log level"),
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_file = PathBuf::from(&out_dir).join("boot.o");

    println!("cargo:rerun-if-changed=src/arch/x86_64/boot.asm");
    println!("cargo:rerun-if-changed=linker.ld");

    let status = Command::new("nasm")
        .args([
            "-f",
            "elf64",
            "src/arch/x86_64/boot.asm",
            "-o",
            out_file.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute nasm. Make sure it is installed.");

    if !status.success() {
        panic!("nasm failed to compile boot.asm");
    }

    println!("cargo:rustc-link-arg={}", out_file.display());
    println!("cargo:rustc-link-arg=-Tlinker.ld");
}
