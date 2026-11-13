develop := "nix --extra-experimental-features 'flakes nix-command' develop --command"
deck := "DECK_CHANNEL='" + env("DECK_CHANNEL", "") + "' DECK_PING='" + env("DECK_PING", "") + \
    "' DECK_UPDATES='" + env("DECK_UPDATES", "") + "'"
nix := if env("IN_NIX_SHELL", "") == "" { develop } else { "" }
profile := env("PROFILE", "dev")
remote := env("REMOTE", "tv@192.168.1.203")
remote_dir := env("REMOTE_DIR", "/tmp/deck-rs")
rsync := "rsync -a --delete --exclude=.git/ --exclude=result --exclude=target/"

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

remote:
    #!/bin/sh
    set -eu
    ssh {{ remote }} "mkdir -p {{ remote_dir }}"
    {{ nix }} {{ rsync }} ./ {{ remote }}:{{ remote_dir }}/
    ssh -t {{ remote }} "cd {{ remote_dir }} && PROFILE={{ profile }} {{ deck }} \
        XDG_RUNTIME_DIR=\${XDG_RUNTIME_DIR:-/run/user/\$(id -u)} WAYLAND_DISPLAY=\${WAYLAND_DISPLAY:-wayland-0} \
        XDG_SESSION_TYPE=wayland \
        {{ develop }} just watch" &
    trap 'kill $! 2>/dev/null' EXIT INT TERM
    {{ nix }} sh -c "fswatch --recursive --one-per-batch --exclude='/\.git/' --exclude='/result$' \
        --exclude='/target/' . | while read -r _; do \
        {{ rsync }} ./ {{ remote }}:{{ remote_dir }}/; done"

run *args:
    {{ nix }} cargo run --profile {{ profile }} {{ args }}

test *args:
    {{ nix }} cargo test {{ args }}

watch *args:
    {{ nix }} watchexec --restart --watch src --watch Cargo.toml -- cargo run --profile {{ profile }} {{ args }}

