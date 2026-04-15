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
        # Separate pkgs instance allowing unfree packages (required by Android SDK)
        pkgsAndroid = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;
          config.android_sdk.accept_license = true;
        };

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
            "aarch64-linux-android"
          ];
        };

        rustStable = pkgs.rust-bin.stable."1.94.0".default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustfmt"
          ];
          targets = [
            "wasm32-unknown-unknown"
            "x86_64-pc-windows-gnu"
            "aarch64-linux-android"
          ];
        };

        androidCompose = pkgsAndroid.androidenv.composeAndroidPackages {
          platformVersions = [ "31" ];
          buildToolsVersions = [ "31.0.0" ];
          abiVersions = [ "arm64-v8a" ];
          includeNDK = true;
          ndkVersions = [ "27.2.12479018" ];
        };

        androidSdk = androidCompose.androidsdk;
        androidSdkRoot = "${androidSdk}/libexec/android-sdk";
        androidNdkRoot = "${androidSdkRoot}/ndk/27.2.12479018";
        androidClang = "${androidNdkRoot}/toolchains/llvm/prebuilt/linux-x86_64/bin";
        androidBuildTools = "${androidSdkRoot}/build-tools/31.0.0";
        androidPlatformJar = "${androidSdkRoot}/platforms/android-31/android.jar";

        androidDeps = [
          androidSdk
          pkgs.jdk17
        ];

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
          unstable = pkgs.mkShell {
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
            # CARGO_PROFILE_DEV_CODEGEN_BACKEND = "cranelift";
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
            CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "${pkgs.pkgsCross.mingwW64.stdenv.cc}/bin/x86_64-w64-mingw32-gcc";
          };

          default = pkgs.mkShell {
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

        packages =
          let
            rustPlatform = pkgs.makeRustPlatform {
              cargo = rustNightly;
              rustc = rustNightly;
            };
          in
          {
            web = pkgs.stdenv.mkDerivation {
              pname = "emf-mmf-web";
              version = "0.1.0";
              src = ./.;

              cargoDeps = rustPlatform.importCargoLock {
                lockFile = ./Cargo.lock;
              };

              nativeBuildInputs = [
                rustPlatform.cargoSetupHook
                rustNightly
                pkgs.wasm-bindgen-cli
                pkgs.binaryen
                pkgs.pkg-config
              ]
              ++ waylandDeps;

              buildPhase = ''
                # export HOME=$(mktemp -d)
                cargo build --profile wasm-release --target wasm32-unknown-unknown --features web
              '';

              installPhase = ''
                mkdir -p $out
                # Generate bindings
                wasm-bindgen --out-dir ./out-wasm --target web target/wasm32-unknown-unknown/wasm-release/emf-mmf.wasm

                # Optimize
                wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
                  --strip-debug \
                  --strip-producers \
                  --dce \
                  --merge-blocks \
                  --optimize-instructions \
                  --output ./out-wasm/emf-mmf_bg.wasm \
                  ./out-wasm/emf-mmf_bg.wasm

                # Copy generated Wasm and JS
                cp ./out-wasm/* $out/

                # Copy index.html
                cp web/index.html $out/index.html
              '';
            };
            default = self.packages.${system}.linux;

            linux = pkgs.stdenv.mkDerivation {
              pname = "emf-mmf-linux";
              version = "0.1.0";
              src = ./.;

              cargoDeps = rustPlatform.importCargoLock {
                lockFile = ./Cargo.lock;
              };

              nativeBuildInputs = [
                rustPlatform.cargoSetupHook
                rustNightly
                pkgs.pkg-config
              ];

              buildInputs = waylandDeps;

              postPatch = ''
                rm -f .cargo/config.toml
              '';

              buildPhase = ''
                cargo build --profile performance
              '';

              installPhase = ''
                mkdir -p $out/bin
                cp target/performance/emf-mmf $out/bin/
              '';
            };

            windows = pkgs.stdenv.mkDerivation {
              pname = "emf-mmf-windows";
              version = "0.1.0";
              src = ./.;

              cargoDeps = rustPlatform.importCargoLock {
                lockFile = ./Cargo.lock;
              };

              nativeBuildInputs = [
                rustPlatform.cargoSetupHook
                rustNightly
                pkgs.pkg-config
              ];

              buildInputs = crossDeps ++ waylandDeps;

              TARGET_CC = "${pkgs.pkgsCross.mingwW64.stdenv.cc}/bin/x86_64-w64-mingw32-gcc";
              CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "${pkgs.pkgsCross.mingwW64.stdenv.cc}/bin/x86_64-w64-mingw32-gcc";
              CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";

              postPatch = ''
                rm -f .cargo/config.toml
              '';

              buildPhase = ''
                cargo build --profile performance --target x86_64-pc-windows-gnu
              '';

              installPhase = ''
                mkdir -p $out/bin
                cp target/x86_64-pc-windows-gnu/performance/emf-mmf.exe $out/bin/
              '';
            };

            android = pkgs.stdenv.mkDerivation {
              pname = "emf-mmf-android";
              version = "0.1.0";
              src = ./.;

              cargoDeps = rustPlatform.importCargoLock {
                lockFile = ./Cargo.lock;
              };

              nativeBuildInputs = [
                rustPlatform.cargoSetupHook
                rustNightly
                pkgs.pkg-config
              ]
              ++ androidDeps;

              # Android NDK clang cross-compiler (aarch64, API 31)
              CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = "${androidClang}/aarch64-linux-android31-clang";
              CC_aarch64_linux_android = "${androidClang}/aarch64-linux-android31-clang";
              CXX_aarch64_linux_android = "${androidClang}/aarch64-linux-android31-clang++";
              AR_aarch64_linux_android = "${androidClang}/llvm-ar";
              CARGO_BUILD_TARGET = "aarch64-linux-android";

              postPatch = ''
                rm -f .cargo/config.toml
              '';

              buildPhase = ''
                # 1) Cross-compile the shared library
                cargo build --profile performance --target aarch64-linux-android
              '';

              installPhase = ''
                mkdir -p $out

                export AAPT2="${androidBuildTools}/aapt2"
                export ZIPALIGN="${androidBuildTools}/zipalign"
                export APKSIGNER="${androidBuildTools}/apksigner"
                export ANDROID_JAR="${androidPlatformJar}"

                SO_FILE="target/aarch64-linux-android/performance/libemf_mmf.so"

                # 2) Create AndroidManifest.xml for NativeActivity
                cat > AndroidManifest.xml <<MANIFEST
                <?xml version="1.0" encoding="utf-8"?>
                <manifest xmlns:android="http://schemas.android.com/apk/res/android"
                    package="com.lalvesl.emfmmf"
                    android:versionCode="1"
                    android:versionName="0.1.0">
                    <uses-sdk android:minSdkVersion="31" android:targetSdkVersion="31" />
                    <uses-feature android:glEsVersion="0x00030000" android:required="true" />
                    <application
                        android:label="EMF-MMF"
                        android:hasCode="false">
                        <activity
                            android:name="android.app.NativeActivity"
                            android:exported="true"
                            android:configChanges="orientation|keyboardHidden|screenSize">
                            <meta-data android:name="android.app.lib_name" android:value="emf_mmf" />
                            <intent-filter>
                                <action android:name="android.intent.action.MAIN" />
                                <category android:name="android.intent.category.LAUNCHER" />
                            </intent-filter>
                        </activity>
                    </application>
                </manifest>
                MANIFEST

                # 3) Compile resources
                mkdir -p build/compiled_resources
                $AAPT2 compile --dir . -o build/compiled_resources/ 2>/dev/null || true

                # 4) Link into base APK
                $AAPT2 link \
                  --manifest AndroidManifest.xml \
                  -I $ANDROID_JAR \
                  -o build/app-unaligned.apk \
                  --min-sdk-version 31 \
                  --target-sdk-version 31

                # 5) Inject the native library into the APK
                mkdir -p build/apk_contents/lib/arm64-v8a
                cp $SO_FILE build/apk_contents/lib/arm64-v8a/libemf_mmf.so

                # Unzip the base APK, add the .so, repackage
                mkdir -p build/apk_extracted
                ${pkgs.unzip}/bin/unzip -o build/app-unaligned.apk -d build/apk_extracted
                cp -r build/apk_contents/lib build/apk_extracted/

                # Rebuild the APK with the .so included
                cd build/apk_extracted
                ${pkgs.zip}/bin/zip -r ../app-with-lib.apk .
                cd ../.. 

                # 6) Zipalign
                $ZIPALIGN -f 4 build/app-with-lib.apk build/app-aligned.apk

                # 7) Sign with a debug keystore
                export HOME=$(pwd)
                mkdir -p $HOME/.android
                keytool -genkey -v \
                  -keystore $HOME/.android/debug.keystore \
                  -alias androiddebugkey \
                  -keyalg RSA -keysize 2048 -validity 10000 \
                  -storepass android -keypass android \
                  -dname "CN=Android Debug,O=Android,C=US"

                $APKSIGNER sign \
                  --ks $HOME/.android/debug.keystore \
                  --ks-key-alias androiddebugkey \
                  --ks-pass pass:android \
                  --key-pass pass:android \
                  --out $out/emf-mmf.apk \
                  build/app-aligned.apk

                echo "APK built: $out/emf-mmf.apk"
              '';
            };

          };

        apps = {
          default = {
            type = "app";
            program = "${pkgs.writeShellScriptBin "run-dev" ''
              export PATH="${rustStable}/bin:${pkgs.pkg-config}/bin:${pkgs.lld}/bin:$PATH"
              export BEVY_ASSET_ROOT="."
              export LD_LIBRARY_PATH="${libPath}:./target/debug:./target/debug/deps:./target/dx/emf-mmf/debug/linux/app:$LD_LIBRARY_PATH"
              ${pkgs.dioxus-cli}/bin/dx serve --features hotpatching
            ''}/bin/run-dev";
          };



          web = {
            type = "app";
            program = "${pkgs.writeShellScriptBin "run-web" ''
              export PATH="${rustStable}/bin:${pkgs.pkg-config}/bin:${pkgs.lld}/bin:$PATH"
              export CARGO_PROFILE_DEV_CODEGEN_BACKEND="llvm"
              export BEVY_ASSET_ROOT="./src"
              export LD_LIBRARY_PATH="${libPath}:$LD_LIBRARY_PATH"
              ${pkgs.dioxus-cli}/bin/dx serve --platform web --features web
            ''}/bin/run-web";
          };



          build-web = {
            type = "app";
            program = "${pkgs.writeShellScriptBin "build-web" ''
              export PATH="${rustStable}/bin:$PATH"
              export BEVY_ASSET_ROOT="."
              ${pkgs.dioxus-cli}/bin/dx build --platform web --release --features web
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
