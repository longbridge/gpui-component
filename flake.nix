{
  description = "gpui-component";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        build-dependencies = with pkgs; [
          pkg-config # For dynamically linked libraries
          makeWrapper # To provide LD_LIBRARY_PATH to the final binary
          (rust-bin.beta.latest.default.override {
            extensions = [ "rust-src" ];
          })
        ];
        dynamic-libraries = with pkgs; [
          wayland
          
          vulkan-headers
          vulkan-loader
          
          libxcb
          libxkbcommon
          
          atk
          fontconfig
          gio-sharp
          glib
          gtk3
        ];
      in
      {
        defaultPackage = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
          pname = "gpui-component-story";
          version = "0.5.1";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };
          nativeBuildInputs = build-dependencies;
          buildInputs = dynamic-libraries;
          postFixup = ''
            wrapProgram $out/bin/gpui-component-story \
            --suffix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath dynamic-libraries}
          '';
        });
        devShells.default = with pkgs; mkShell {
          buildInputs = build-dependencies ++ dynamic-libraries;

          env = {
            RUST_BACKTRACE = "1";
            LD_LIBRARY_PATH = lib.makeLibraryPath dynamic-libraries;
          };
        };
      }
    );
}
