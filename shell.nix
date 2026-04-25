{
    pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
    buildInputs = with pkgs; [
        meson
        ninja
        libclang
        llvmPackages_20.bintools-unwrapped
        llvmPackages_20.clang-unwrapped
    ];

    LIBCLANG_PATH="${pkgs.libclang.lib}/lib";
}
