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
      defaults = builtins.fromTOML (builtins.readFile ./assets/config.defaults.toml);
      libraries =
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
      mkPackage =
        pkgs: settings:
        let
          configFile = (pkgs.formats.toml { }).generate "deck-rs-default.toml" settings;
        in
        pkgs.rustPlatform.buildRustPackage {
          pname = "deck-rs";
          version = (pkgs.lib.importTOML ./Cargo.toml).package.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          postPatch = ''
            cp ${configFile} assets/config.defaults.toml
          '';
          nativeBuildInputs = [
            pkgs.makeWrapper
            pkgs.pkg-config
          ];
          buildInputs = libraries pkgs;
          passthru.settings = settings;
          postInstall = ''
            wrapProgram $out/bin/deck-rs \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (libraries pkgs)}
          '';
        };
      settingsPackage = pkgs: settings: mkPackage pkgs (pkgs.lib.recursiveUpdate defaults settings);
      module =
        packages:
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.programs.deck-rs;
        in
        {
          options.programs.deck-rs = {
            enable = lib.mkEnableOption "deck-rs";
            package = lib.mkOption {
              type = lib.types.package;
              default = settingsPackage pkgs cfg.settings;
            };
            settings = lib.mkOption {
              type = lib.types.attrsOf lib.types.anything;
              default = { };
            };
          };
          config = lib.mkIf cfg.enable (lib.setAttrByPath packages [ cfg.package ]);
        };
      overlay = final: prev: {
        deck-rs = final.lib.makeOverridable (settingsPackage final) { };
      };
    in
    {
      packages = each (pkgs: {
        default = mkPackage pkgs defaults;
      });
      overlays.default = overlay;
      lib.settingsPackage = settingsPackage;
      nixosModules.default = module [
        "environment"
        "systemPackages"
      ];
      homeManagerModules.default = module [
        "home"
        "packages"
      ];
      devShells = each (pkgs: {
        default = pkgs.mkShell {
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (libraries pkgs);
          buildInputs = libraries pkgs;
          packages = with pkgs; [
            cargo
            clippy
            fswatch
            grim
            just
            mold
            nix
            nixfmt
            pkg-config
            rsync
            rust-analyzer
            rustc
            rustfmt
            sway
            watchexec
            wtype
          ];
        };
      });
      formatter = each (pkgs: pkgs.nixfmt);
    };
}
