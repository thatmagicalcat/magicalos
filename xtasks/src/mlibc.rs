use color_eyre::Result;
use xshell::{cmd, Shell};
use xtasks::project_root;

pub fn setup(sh: &Shell, _quiet: bool) -> Result<()> {
    let project_root = project_root();
    let sysroot = project_root.join("sysroot");
    let finished_marker = project_root.join("mlibc/.finished");

    sh.create_dir(&sysroot)?;

    if finished_marker.exists() {
        println!("[xtask]: mlibc already built");
        return Ok(());
    }

    println!("[xtask]: mlibc already built");

    // ignore the error
    _ = cmd!(
        sh,
        "git clone https://github.com/thatmagicalcat/magicalos-mlibc mlibc"
    )
    .run();

    let _dir = sh.push_dir(project_root.join("mlibc"));
    let _env = sh.push_env("DESTDIR", sysroot);

    cmd!(
        sh,
        "meson setup
            --cross-file=magicalos.cross-file
            --prefix=/usr
            -Dheaders_only=true
            headers-build"
    )
    .run()?;

    cmd!(sh, "ninja -C headers-build install").run()?;

    cmd!(
        sh,
        "meson
            setup
            --cross-file=magicalos.cross-file
            --prefix=/usr
            -Ddefault_library=static
            -Dno_headers=true
            -Dlibgcc_dependency=false
            build"
    )
    .run()?;

    let parallelism = String::from_utf8(cmd!(sh, "nproc").output()?.stdout).unwrap();
    let parallelism = parallelism.trim();

    cmd!(sh, "ninja -C build install -j{parallelism}").run()?;

    std::fs::File::create(finished_marker)?;
    Ok(())
}
