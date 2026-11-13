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
          udev
          vulkan-loader
          wayland
        ];
      font =
        pkgs:
        "${pkgs.nerd-fonts.meslo-lg}/share/fonts/truetype/NerdFonts/MesloLG/MesloLGSNerdFont-Regular.ttf";
      tools =
        pkgs: with pkgs; [
          bluez
          brightnessctl
          curl
          git
          iproute2
          iputils
          iwd
          power-profiles-daemon
          systemd
          wireplumber
        ];
    in
    {
      packages = each (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "deck-rs";
          version = (pkgs.lib.importTOML ./Cargo.toml).package.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [
            pkgs.makeWrapper
            pkgs.pkg-config
          ];
          buildInputs = runtimeLibraries pkgs;
          postInstall = ''
            wrapProgram $out/bin/deck-rs \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (runtimeLibraries pkgs)} \
              --suffix PATH : ${pkgs.lib.makeBinPath (tools pkgs)} \
              --set-default DECK_FONT ${font pkgs}
          '';
        };
      });
      devShells = each (pkgs: {
        default = pkgs.mkShell {
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
          DECK_FONT = font pkgs;
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (runtimeLibraries pkgs);
          buildInputs = runtimeLibraries pkgs;
          packages =
            with pkgs;
            [
              cargo
              clippy
              fswatch
              just
              mold
              nixfmt
              pkg-config
              rsync
              rust-analyzer
              rustc
              rustfmt
              watchexec
            ]
            ++ tools pkgs;
        };
      });
      formatter = each (pkgs: pkgs.nixfmt);
    };
}
