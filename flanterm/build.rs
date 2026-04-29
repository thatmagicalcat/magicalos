use std::{path::PathBuf, process::Command};

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .use_core()
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(&out_dir);
    bindings
        .write_to_file(out_path.join("flanterm_bindings.rs"))
        .expect("Couldn't write bindings!");

    let sources = [
        "./Flanterm/flanterm.c",
        "./Flanterm/flanterm_backends/fb.c",
    ];

    let objects = sources.map(|src| format!("{}/{}.o", out_dir, src.replace("/", "_")));

    let args = [
        "-target",
        "x86_64-unknown-none",
        "-ffreestanding",
        "-fno-stack-protector",
        "-mno-red-zone",
        "-mno-mmx",
        "-mno-sse",
        "-mno-sse2",
        "-mno-sse3",
        "-mno-ssse3",
        "-mno-sse4.1",
        "-mno-sse4.2",
        "-mno-avx",
        "-mno-avx2",
        "-nostdlib",
        "-mno-red-zone",
    ];

    for (src, obj) in sources.iter().zip(objects.iter()) {
        let status = Command::new("clang")
            .args(args)
            .args(["-c", src, "-o", obj])
            .status()
            .unwrap();
        assert!(status.success());
    }

    let mut ar = Command::new("ar");
    ar.arg("rcs").arg(format!("{}/libflanterm.a", out_dir));

    for obj in &objects {
        ar.arg(obj);
    }

    let status = ar.status().unwrap();
    assert!(status.success());

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=flanterm");
}
