# deck-rs

Fullscreen TV launcher built with [iced](https://iced.rs), laid out like the PlayStation 4 settings screen and
Windows Media Center: a vertical menu of apps, network, bluetooth, settings and power on the left, and the
focused entry's options on the right. Controlled by keyboard, remote or gamepad.

## Controls

| Action | Keyboard / remote       | Gamepad           |
| ------ | ----------------------- | ----------------- |
| Move   | Arrows                  | D-pad, left stick |
| Select | Enter, Space, Select    | South (A / Cross) |
| Back   | Escape, Backspace, Back | East (B / Circle) |
| Power  | Home, Menu, Power       | Start, Guide      |

Right or select enters the options pane; left or back returns to the menu. Select on an app launches it. In
the pane, left and right adjust volume, brightness and power profile.

## Configuration

Defaults live in `config.toml` and are embedded in the binary. Settings are layered in this order, later
files overriding earlier ones (tables merge, arrays replace):

1. the embedded `config.toml`
2. `DECK_DEFAULT_CONFIG`, set by the Nix package to its baked-in settings
3. `DECK_CONFIG`, or `$XDG_CONFIG_HOME/deck-rs/config.toml` when unset

```toml
[[app]]
command = ["steam", "-gamepadui"]
env = { GDK_BACKEND = "wayland,x11" }
icon = "\U000F04D3"
name = "steam"

[network]
backend = "iwd"

[ui]
font = "/path/to/your-font.ttf"
font_family = "Symbols Nerd Font Mono"
```

| Section        | Backends           |
| -------------- | ------------------ |
| `[audio]`      | `wpctl`, `pactl`   |
| `[bluetooth]`  | `bluez`            |
| `[brightness]` | `logind`           |
| `[network]`    | `iwd`              |
| `[power]`      | `logind`           |
| `[profile]`    | `ppd`              |

`auto` selects a detected backend and `none` disables the module. A module only appears when its D-Bus
service, executable or backlight device is present. For iwd, enable the service and persist `/var/lib/iwd`.

The Symbols Nerd Font Mono icon font is embedded in Cargo builds; Nix packages pass it through `DECK_FONT`.
Set `[ui].font` to a replacement font file and `[ui].font_family` to its family name. The clock uses `date`.

The default apps are steam, jellyfin-desktop and Grayjay through Waydroid; install them on the host as needed.

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) and Cargo for a direct, non-flake build.
- [Nix](https://nixos.org/download) for the package and development shell.
- [just](https://just.systems), or use the one provided by the development shell.

Recipes run through `nix develop` automatically, or directly when already inside the development shell. Nix
flakes only see git-tracked files, so keep `flake.nix`, `flake.lock`, `config.toml` and the font asset tracked.

## Usage

| Command       | Description                                              |
| ------------- | -------------------------------------------------------- |
| `just`        | List recipes                                             |
| `just build`  | Build the app                                            |
| `just check`  | Run lint and tests                                       |
| `just format` | Format Rust and Nix files                                |
| `just lint`   | Check formatting and run clippy                          |
| `just remote` | Sync to `REMOTE=user@host` and run it on change          |
| `just run`    | Run the app                                              |
| `just shot`   | Screenshot headlessly, e.g. `just shot power Home Right` |
| `just test`   | Run tests                                                |
| `just watch`  | Rebuild and rerun the app on change                      |

Extra arguments are passed to cargo, e.g. `just test config::tests`. Screenshots are written to
`target/shots/`; `RESOLUTION` (default `3840x2160`) and `DELAY` (default `3`) tune `just shot`.

The build profile defaults to `dev`; override it with `PROFILE`:

```sh
PROFILE=release just build
PROFILE=release just run
```

## Development shell

```sh
nix --extra-experimental-features "flakes nix-command" develop
```

Provides the Rust toolchain, rust-analyzer, just, nixfmt, Nix and the runtime libraries iced needs.

Update dependencies with `nix flake update --extra-experimental-features "flakes nix-command"`.

## Install on NixOS

The flake exports `packages.<system>.default`, `nixosModules.default` and `homeManagerModules.default`. Set
`programs.deck-rs.settings` to override the defaults. Replace `<owner>` with the account hosting this repository:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    deck-rs.url = "github:<owner>/deck-rs";
    deck-rs.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { nixpkgs, deck-rs, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        deck-rs.nixosModules.default
        {
          programs.deck-rs = {
            enable = true;
            settings.network.backend = "iwd";
          };
        }
      ];
    };
  };
}
```

Rebuild with `sudo nixos-rebuild switch --flake .#my-host`. For a local checkout, use
`deck-rs.url = "path:/path/to/deck-rs";`. Home Manager users add `deck-rs.homeManagerModules.default` and set
the same options. For a standalone package with custom settings, use `deck-rs.lib.settingsPackage pkgs settings`.
