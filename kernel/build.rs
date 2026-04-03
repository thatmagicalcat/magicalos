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

    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rustc-link-arg=-Tlinker.ld");
}
