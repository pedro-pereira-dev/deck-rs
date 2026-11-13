# deck-rs

Desktop app built with [iced](https://iced.rs).

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

Provides the Rust toolchain, rust-analyzer, just, nixfmt and the runtime libraries iced needs.

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
