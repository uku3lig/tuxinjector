{
  description = "Tuxinjector Linux - Minecraft speedrunning overlay (Rust + Vulkan)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };

        # Native build dependencies
        buildInputs = with pkgs; [
          vulkan-headers
          vulkan-loader
          vulkan-validation-layers
          shaderc
          libGL
          libGLU
          mesa
          wayland
          wayland-protocols
          libxkbcommon
          libx11
          libxrandr
          libxinerama
          libxcursor
          libxi
          libxext
          pipewire
          dbus
        ];

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          cmake
          ninja
          gcc
          python3
          clang
          llvmPackages.libclang
          python3Packages.mkdocs
          python3Packages.mkdocs-material
          python3Packages.pymdown-extensions
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath buildInputs}:$LD_LIBRARY_PATH"
            export VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d"
            export SHADERC_LIB_DIR="${pkgs.shaderc.lib}/lib"
            export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
            echo "tuxinjector dev shell ready"
            echo "  cargo build --release    # build the .so"
            echo "  cargo test               # run tests"
            echo "  cargo clippy             # lint"
            echo "  mkdocs serve             # preview docs"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "tuxinjector";
          version = "1.0.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          inherit buildInputs nativeBuildInputs;

          SHADERC_LIB_DIR = "${pkgs.shaderc.lib}/lib";

          meta = with pkgs.lib; {
            description = "Minecraft speedrunning overlay for Linux";
            license = licenses.mit;
            platforms = platforms.linux;
          };
        };

        # MkDocs documentation
        devShells.docs = pkgs.mkShell {
          packages = with pkgs; [
            python3
          clang
          llvmPackages.libclang
            python3Packages.mkdocs
            python3Packages.mkdocs-material
            python3Packages.pymdown-extensions
          ];

          shellHook = ''
            echo "docs shell ready"
            echo "  mkdocs serve     # preview docs at localhost:8000"
            echo "  mkdocs build     # build static site"
          '';
        };
      }
    );
}
