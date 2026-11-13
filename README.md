# deck-rs

Fullscreen TV launcher built with [iced](https://iced.rs): app tiles, network, bluetooth, audio, display and
power panels, system stats, kernel and NixOS generation. Controlled by keyboard, remote or gamepad; the cursor
stays hidden until the mouse moves.

## Controls

| Action | Keyboard / remote                   | Gamepad              |
| ------ | ----------------------------------- | -------------------- |
| Move   | Arrows                              | D-pad, left stick    |
| Select | Enter, Space, Select                | South (A / Cross)    |
| Back   | Escape, Backspace, Back             | East (B / Circle)    |
| Power  | Home, Menu, Power                   | Start, Guide         |

In panels, left and right adjust volume, brightness and power profile.

## Configuration

Apps come from `$XDG_CONFIG_HOME/deck-rs/apps.toml`, defaulting to steam, jellyfin and grayjay:

```toml
[[app]]
command = ["steam", "-gamepadui"]
icon = "\U000F04D3"
name = "steam"
```

Grayjay's default command launches `com.futo.platformplayer` through Waydroid. Install Grayjay in Waydroid first.

`icon` is any MesloLGS Nerd Font glyph. `DECK_FONT` points to the font file and is set by the package and the
development shell. System control uses `systemctl`, `loginctl`, `iwctl`, `ping`, `bluetoothctl`, `wpctl`,
`brightnessctl` and `powerprofilesctl`, which the package adds to `PATH`.

Wi-Fi uses iwd, enable it with `networking.wireless.iwd.enable = true;` and keep `/var/lib/iwd` persistent.
Power actions fall back to `sudo -n systemctl`, so allow `systemctl poweroff|reboot|suspend` with
`security.sudo.extraRules` when they are denied.
The footer pings `DECK_PING` (default `9.9.9.9`).

The footer checks for updates hourly: the running nixpkgs revision against `DECK_CHANNEL` (default
`nixos-unstable`), and, when `DECK_UPDATES` is set to a git URL, its `HEAD` against the system
configuration revision, which requires `system.configurationRevision = self.rev or self.dirtyRev;`.

## Requirements

- [Nix](https://nixos.org/download); recipes enable flakes themselves.
- [just](https://just.systems), or use the one provided by the development shell.

Recipes run through `nix develop` automatically, or directly when already inside the development shell.

Nix flakes only see git-tracked files, so keep `flake.nix`, `flake.lock` and `justfile` tracked.

## Usage

| Command       | Description                              |
| ------------- | ---------------------------------------- |
| `just`        | List recipes                             |
| `just build`  | Build the app                            |
| `just check`  | Run lint and test                        |
| `just format` | Format Rust and Nix files                |
| `just lint`   | Check formatting and run clippy          |
| `just run`    | Run the app                              |
| `just remote` | Sync to `REMOTE` and run it live there   |
| `just test`   | Run tests                                |
| `just watch`  | Rebuild and rerun the app on change      |

Extra arguments are passed to cargo, e.g. `just test some_test`.

The build profile defaults to `dev`; override it with `PROFILE`:

```sh
PROFILE=release just build
PROFILE=release just run
```

## Development shell

```sh
nix --extra-experimental-features "flakes nix-command" develop
```

Provides the Rust toolchain, rust-analyzer, just, nixfmt, the runtime libraries iced needs and the system tools.

Update dependencies with `nix flake update --extra-experimental-features "flakes nix-command"`.

## Install on NixOS

The flake exports the application as `packages.<system>.default` for `x86_64-linux` and
`aarch64-linux`. Add it as an input to the flake that manages your NixOS configuration, replacing
`<owner>` with the GitHub account or organization that hosts this repository:

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
        ({ pkgs, ... }: {
          environment.systemPackages = [
            deck-rs.packages.${pkgs.system}.default
          ];
        })
      ];
    };
  };
}
```

Replace `my-host` with your NixOS configuration name, then rebuild with
`sudo nixos-rebuild switch --flake .#my-host`. The `deck-rs` executable will be available on your
system path. For a local checkout, use `deck-rs.url = "path:/path/to/deck-rs";` instead.
