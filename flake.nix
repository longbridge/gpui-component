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
      in
      {
        devShells.default = with pkgs; mkShell {
          buildInputs = [
            openssl
            pkg-config
            xorg.libX11
            glib
            pango
            atkmm
            gdk-pixbuf
            gtk3
            libsoup_3
            webkitgtk_4_1
            libxkbcommon
            vulkan-loader
            (rust-bin.beta.latest.default.override {
              extensions = [ "rust-src" ];
            })
          ];

          env = {
            RUST_BACKTRACE = "1";
            LD_LIBRARY_PATH = lib.makeLibraryPath [ vulkan-loader ];
          };
        };
      }
    );
}
