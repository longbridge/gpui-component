{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }: let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
    ];

    overlays = {
      rust-overlay = rust-overlay.overlays.default;
      rust-toolchain = final: prev: {
        rustToolchain = final.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      };
    };

    mkPkgs = system:
      import nixpkgs {
        inherit system;
        overlays = builtins.attrValues overlays;
      };

    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f (mkPkgs system));
  in {
    packages = forAllSystems (pkgs: {
      default = pkgs.zed-editor;
    });

    devShells = forAllSystems (pkgs: {
      default = pkgs.mkShell rec {
        buildInputs = [
          pkgs.openssl
          pkgs.stdenv.cc.cc
          pkgs.zlib
          pkgs.rustToolchain
          pkgs.libxkbcommon
          # pkgs.wayland
          # pkgs.xorg.libxcb
          pkgs.vulkan-loader
          pkgs.gdk-pixbuf
          pkgs.gtk3
          pkgs.atkmm
          pkgs.libsoup_3
          pkgs.webkitgtk_4_1
          pkgs.cairo
          pkgs.pango
          pkgs.glib
          pkgs.gdk-pixbuf
          pkgs.pkg-config

          pkgs.nixd
        ];

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
      };
    });

    overlays =
      overlays
      // {
        default = nixpkgs.lib.composeManyExtensions (builtins.attrValues overlays);
      };
  };
}
