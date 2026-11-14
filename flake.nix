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
      defaults = builtins.fromTOML (builtins.readFile ./config.toml);
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
          fontFile = pkgs.runCommand "symbols-nerd-font-mono.ttf" { } ''
            mkdir -p $out
            cp ${pkgs.nerd-fonts.symbols-only}/share/fonts/truetype/NerdFonts/Symbols/SymbolsNerdFontMono-Regular.ttf \
              $out/symbols.ttf
          '';
        in
        pkgs.rustPlatform.buildRustPackage {
          pname = "deck-rs";
          version = (pkgs.lib.importTOML ./Cargo.toml).package.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [
            pkgs.makeWrapper
            pkgs.pkg-config
          ];
          buildInputs = libraries pkgs;
          passthru.settings = settings;
          postInstall = ''
            wrapProgram $out/bin/deck-rs \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (libraries pkgs)} \
              --set-default DECK_DEFAULT_CONFIG ${configFile} \
              --set-default DECK_FONT ${fontFile}/symbols.ttf
          '';
        };
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
              default = mkPackage pkgs (lib.recursiveUpdate defaults cfg.settings);
            };
            settings = lib.mkOption {
              type = lib.types.attrsOf lib.types.anything;
              default = { };
            };
          };
          config = lib.mkIf cfg.enable (lib.setAttrByPath packages [ cfg.package ]);
        };
      overlay = final: prev: {
        deck-rs = final.lib.makeOverridable (
          settings: mkPackage final (final.lib.recursiveUpdate defaults settings)
        ) { };
      };
    in
    {
      packages = each (pkgs: {
        default = mkPackage pkgs defaults;
      });
      overlays.default = overlay;
      lib.settingsPackage = pkgs: settings: mkPackage pkgs (pkgs.lib.recursiveUpdate defaults settings);
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
            nixfmt
            pkg-config
            rsync
            rust-analyzer
            rustc
            rustfmt
            sway
            watchexec
            wtype
            nix
          ];
        };
      });
      formatter = each (pkgs: pkgs.nixfmt);
    };
}
