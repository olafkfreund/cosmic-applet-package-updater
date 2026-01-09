{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    cargo
    rustc
    pkg-config

    # Required system libraries for libcosmic
    libxkbcommon
    wayland
    wayland-protocols

    # Build tools
    just
    git
  ];

  shellHook = ''
    echo "COSMIC Package Updater development environment"
    echo "Run 'just build-release' to build"
    echo "Run 'just run' to test"
  '';
}
