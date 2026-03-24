{
  description = "EMF MMF";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        waylandDeps = with pkgs; [
          wayland
          vulkan-loader
          libxkbcommon
          udev
          alsa-lib
        ];

        rustNightly = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustfmt"
            "rustc-codegen-cranelift-preview"
          ];
          targets = [
            "wasm32-unknown-unknown"
            "x86_64-pc-windows-gnu"
          ];
        };

        rustStable = pkgs.rust-bin.stable."1.94.0".default.override {
          targets = [
            "wasm32-unknown-unknown"
            "x86_64-pc-windows-gnu"
          ];
        };

        crossDeps = with pkgs; [
          wasm-bindgen-cli
          binaryen
          pkgsCross.mingwW64.stdenv.cc
          pkgsCross.mingwW64.windows.pthreads
        ];

        libPath = pkgs.lib.makeLibraryPath waylandDeps;

      in
      {
        devShells = {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              pkg-config
              lld
              mold
            ];
            buildInputs =
              waylandDeps
              ++ crossDeps
              ++ [
                rustNightly
                pkgs.dioxus-cli
              ];

            LD_LIBRARY_PATH = libPath;
            CARGO_PROFILE_DEV_CODEGEN_BACKEND = "cranelift";
          };

          build = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              pkg-config
              lld
            ];
            buildInputs =
              waylandDeps
              ++ crossDeps
              ++ [
                rustStable
                pkgs.dioxus-cli
              ];

            LD_LIBRARY_PATH = libPath;
          };
        };

        packages = let
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustStable;
            rustc = rustStable;
          };
        in {
          default = self.packages.${system}.linux;

          linux = rustPlatform.buildRustPackage {
            pname = "emf-mmf-linux";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = waylandDeps;
            
            postPatch = ''
              rm -f .cargo/config.toml
            '';
          };

          windows = rustPlatform.buildRustPackage {
            pname = "emf-mmf-windows";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = crossDeps;
            
            TARGET_CC = "${pkgs.pkgsCross.mingwW64.stdenv.cc}/bin/x86_64-w64-mingw32-gcc";
            CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "${pkgs.pkgsCross.mingwW64.stdenv.cc}/bin/x86_64-w64-mingw32-gcc";
            CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
            
            postPatch = ''
              rm -f .cargo/config.toml
            '';
            doCheck = false;
          };

          web = rustPlatform.buildRustPackage {
            pname = "emf-mmf-web";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            
            nativeBuildInputs = [ pkgs.wasm-bindgen-cli pkgs.binaryen ];
            
            CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
            cargoBuildType = "wasm-release";

            postPatch = ''
              rm -f .cargo/config.toml
            '';
            doCheck = false;
            
            installPhase = ''
              mkdir -p $out
              wasm-bindgen --out-dir $out --target web target/wasm32-unknown-unknown/wasm-release/emf-mmf.wasm
              echo '<!DOCTYPE html><html><head><meta charset="utf-8"/><title>EMF-MMF Simulator</title></head><body><script type="module">import init from "./emf-mmf.js"; init();</script></body></html>' > $out/index.html
            '';
          };
        };

        apps = {
          default = {
            type = "app";
            program = "${pkgs.writeShellScriptBin "run-dev" ''
              export BEVY_ASSET_ROOT="."
              export LD_LIBRARY_PATH="${libPath}:$LD_LIBRARY_PATH"
              ${pkgs.dioxus-cli}/bin/dx serve --hot-patch --features bevy/hotpatching
            ''}/bin/run-dev";
          };

          web = {
            type = "app";
            program = "${pkgs.writeShellScriptBin "run-web" ''
              export CARGO_PROFILE_DEV_CODEGEN_BACKEND="llvm"
              export BEVY_ASSET_ROOT="./src"
              export LD_LIBRARY_PATH="${libPath}:$LD_LIBRARY_PATH"
              ${pkgs.dioxus-cli}/bin/dx serve --platform web
            ''}/bin/run-web";
          };

          build-web = {
            type = "app";
            program = "${pkgs.writeShellScriptBin "build-web" ''
              export PATH="${rustStable}/bin:$PATH"
              export BEVY_ASSET_ROOT="."
              ${pkgs.dioxus-cli}/bin/dx build --platform web --release
            ''}/bin/build-web";
          };

          build-linux = {
            type = "app";
            program = "${pkgs.writeShellScriptBin "build-linux" ''
              export PATH="${rustStable}/bin:$PATH"
              export BEVY_ASSET_ROOT="."
              export LD_LIBRARY_PATH="${libPath}:$LD_LIBRARY_PATH"
              ${pkgs.dioxus-cli}/bin/dx build --release
            ''}/bin/build-linux";
          };

          build-windows = {
            type = "app";
            program = "${pkgs.writeShellScriptBin "build-windows" ''
              export PATH="${rustStable}/bin:$PATH"
              export BEVY_ASSET_ROOT="."
              export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="${pkgs.pkgsCross.mingwW64.stdenv.cc}/bin/x86_64-w64-mingw32-gcc"
              ${rustStable}/bin/cargo build --release --target x86_64-pc-windows-gnu
            ''}/bin/build-windows";
          };
        };

      }
    );
}
