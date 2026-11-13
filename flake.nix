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
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (runtimeLibraries pkgs);
          buildInputs = runtimeLibraries pkgs;
          packages = with pkgs; [
            cargo
            clippy
            just
            lld
            nixfmt
            pkg-config
            rust-analyzer
            rustc
            rustfmt
            trunk
            wasm-bindgen-cli
          ];
        };
      });
      formatter = each (pkgs: pkgs.nixfmt);
    };
}
