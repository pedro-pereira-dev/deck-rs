{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "aarch64-linux"
        "x86_64-linux"
      ];
      each = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      runtimeLibraries =
        pkgs: with pkgs; [
          fontconfig
          libGL
          libx11
          libxcursor
          libxi
          libxkbcommon
          libxrandr
          vulkan-loader
          wayland
        ];
    in
    {
      packages = each (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "deck-rs";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [
            pkgs.makeWrapper
            pkgs.pkg-config
          ];
          buildInputs = runtimeLibraries pkgs;
          postInstall = ''
            wrapProgram $out/bin/deck-rs \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (runtimeLibraries pkgs)}
          '';
        };
      });
      devShells = each (pkgs: {
        default = pkgs.mkShell {
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (runtimeLibraries pkgs);
          buildInputs = runtimeLibraries pkgs;
          packages = with pkgs; [
            cargo
            clippy
            fswatch
            just
            lld
            mold
            nixfmt
            pkg-config
            rsync
            rust-analyzer
            rustc
            rustfmt
            trunk
            wasm-bindgen-cli
            watchexec
          ];
        };
      });
      formatter = each (pkgs: pkgs.nixfmt);
    };
}
