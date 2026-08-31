{
  description = "eframe devShell";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs     = import nixpkgs { inherit system overlays; };
      in with pkgs; {
        devShells.default = mkShell rec {
          buildInputs = [
            # Rust
            (rust-bin.stable.latest.default.override {
              targets = [ "x86_64-pc-windows-gnu" ];
            })

            # Windows libs
            pkgsCross.mingwW64.stdenv.cc
            pkgsCross.mingwW64.windows.pthreads

            # linux libraries
            pkg-config
            libxkbcommon
            libGL
            fontconfig
            wayland
          ];

          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        };
      });
}
