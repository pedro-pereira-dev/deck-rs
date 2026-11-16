# deck-rs

Fullscreen TV launcher built with [iced](https://iced.rs), laid out like the PlayStation 3 XrossMediaBar: a
horizontal row of categories with the focused category's items listed vertically beneath it. Controlled by
keyboard, remote, gamepad or mouse.

## Architecture

The source is organized by responsibility:

- `src/deck.rs` owns Iced application state, navigation, updates and subscriptions.
- `src/input.rs` translates keyboard and gamepad events into navigation messages.
- `src/menu.rs` builds app, status and system-control menus.
- `src/view.rs` draws the interface and translates mouse events into the same messages.
- `src/config.rs` loads defaults and user settings.
- `src/system/` queries host and services, performs actions, and launches processes.
- `src/main.rs` starts the application.

System queries run concurrently when a snapshot is refreshed. Menus are rebuilt from each new snapshot and
selection is clamped to available rows. Menu movement uses a short 120 fps slide by default; no animation frames are
scheduled when the target position has been reached.

## Controls

| Action | Keyboard / remote       | Gamepad           |
| ------ | ----------------------- | ----------------- |
| Move   | Arrows                  | D-pad, left stick |
| Select | Enter, Space, Select    | South (A / Cross) |
| Back   | Escape, Backspace, Back | East (B / Circle) |
| Power  | Home, Menu, Power       | Start, Guide      |

Left and right switch categories, up and down move through items and select runs an item. Items with options
open a submenu on the right, up to three levels deep (settings, bluetooth, device, action); back or left closes
one level. Mouse clicks focus or run items, right click goes back and the wheel scrolls. Every move plays a
navigation sound. The top right shows the time, date, cpu, temperature, memory, disk, ping, volume and the
bluetooth state with connected devices.

## Configuration

Defaults live in `assets/config.defaults.toml` and are embedded in the binary. Nix package settings are baked
into that embedded file when the package is built. The user configuration is layered over those defaults
(tables merge, arrays replace):

1. the embedded `assets/config.defaults.toml` (including Nix package settings, if configured)
2. `DECK_CONFIG`, or `$XDG_CONFIG_HOME/deck-rs/config.toml` when unset

```toml
[[app]]
category = "games"
command = ["steam", "-gamepadui"]
env = { GDK_BACKEND = "wayland,x11" }
icon = "\U000F04D3"
name = "steam"

[ui]
animation_frequency = 120
background = "/path/to/wallpaper.png"
categories = ["games"]
favorites = ["steam"]
font = "/path/to/your-font.ttf"
font_family = "Symbols Nerd Font Mono"
icons = { games = "\U000F0297" }
ping = "9.9.9.9"
navigation_sound = "/path/to/nav.wav"
```

`Home` and `Favorites` are always shown first, followed by configured extra categories, then `Settings` and
`Power`; Favorites is focused at startup. `[ui].categories` adds app categories between Favorites and Settings
and cannot remove or reorder the built-ins. Category matching and duplicate filtering are case-insensitive. Each
extra category lists the `[[app]]` entries with that `category`, using its glyph from `[ui].icons`. `favorites`
names apps from any category and always ends with suspend. The default font and sound use the bundled assets; an empty
`background` is black and an empty `navigation_sound` uses the embedded click through `pw-play` or `paplay`.
`[ui].animation_frequency` sets the animation frame rate in frames per second; it defaults to `120`.

System controls are detected automatically. Audio controls appear when `wpctl` or `pactl` is installed;
Bluetooth, Wi-Fi, power, brightness, and power-profile controls appear when their supported system services or
devices are available. No backend selection is needed in the configuration. For Wi-Fi controls, enable iwd and
persist `/var/lib/iwd`.

The Symbols Nerd Font Mono icon font is bundled with the application. Set `[ui].font` to a replacement font
file and `[ui].font_family` to its family name. The clock uses `date`.

No apps are configured by default.

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) and Cargo for a direct, non-flake build.
- [Nix](https://nixos.org/download) for the package and development shell.
- [just](https://just.systems), or use the one provided by the development shell.

Recipes run through `nix develop` automatically, or directly when already inside the development shell. Nix
flakes only see git-tracked files, so keep `flake.nix`, `flake.lock`, `assets/config.defaults.toml` and
the font asset tracked.

## Usage

| Command       | Description                                              |
| ------------- | -------------------------------------------------------- |
| `just`        | List recipes                                             |
| `just build`  | Build the app                                            |
| `just check`  | Run formatting and lint checks                          |
| `just format` | Format Rust and Nix files                                |
| `just lint`   | Check formatting and run clippy                          |
| `just remote` | Sync to `REMOTE=user@host` and run it on change          |
| `just run`    | Run the app                                              |
| `just shot`   | Screenshot headlessly, e.g. `just shot power Home Right` |
| `just watch`  | Rebuild and rerun the app on change                      |

Screenshots are written to `target/shots/`; `RESOLUTION` (default `3840x2160`) and `DELAY` (default `3`) tune
`just shot`.

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
