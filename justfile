develop := "nix --extra-experimental-features 'flakes nix-command' develop --command"
nix := if env("IN_NIX_SHELL", "") == "" { develop } else { "" }
profile := env("PROFILE", "dev")
web_address := env("WEB_ADDRESS", "127.0.0.1")
web_port := env("WEB_PORT", "8080")

default:
    @just --list

build *args:
    {{ nix }} cargo build --profile {{ profile }} {{ args }}

check: lint test

format:
    {{ nix }} cargo fmt
    {{ nix }} nixfmt flake.nix

lint:
    {{ nix }} cargo fmt --check
    {{ nix }} cargo clippy --all-targets -- -D warnings
    {{ nix }} nixfmt --check flake.nix

run *args:
    {{ nix }} cargo run --profile {{ profile }} {{ args }}

test *args:
    {{ nix }} cargo test {{ args }}

web *args:
    {{ nix }} trunk serve --address {{ web_address }} --port {{ web_port }} \
        {{ if profile == "release" { "--release" } else { "" } }} {{ args }}
