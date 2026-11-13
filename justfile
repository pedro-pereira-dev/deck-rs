develop := "nix --extra-experimental-features 'flakes nix-command' develop --command"
nix := if env("IN_NIX_SHELL", "") == "" { develop } else { "" }
profile := env("PROFILE", "dev")
remote := env("REMOTE", "tv@192.168.1.203")
remote_dir := env("REMOTE_DIR", "/tmp/deck-rs")
rsync := "rsync -a --delete --exclude=.git/ --exclude=dist/ --exclude=result --exclude=target/"
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

remote:
    #!/bin/sh
    set -eu
    ssh {{ remote }} "mkdir -p {{ remote_dir }}"
    {{ nix }} {{ rsync }} ./ {{ remote }}:{{ remote_dir }}/
    ssh -t {{ remote }} "cd {{ remote_dir }} && PROFILE={{ profile }} \
        XDG_RUNTIME_DIR=\${XDG_RUNTIME_DIR:-/run/user/\$(id -u)} WAYLAND_DISPLAY=\${WAYLAND_DISPLAY:-wayland-0} \
        nix --extra-experimental-features 'flakes nix-command' develop --command just watch" &
    trap 'kill $! 2>/dev/null' EXIT INT TERM
    {{ nix }} sh -c "fswatch --recursive --one-per-batch --exclude='/\.git/' --exclude='/dist/' \
        --exclude='/result$' --exclude='/target/' . | while read -r _; do \
        {{ rsync }} ./ {{ remote }}:{{ remote_dir }}/; done"

run *args:
    {{ nix }} cargo run --profile {{ profile }} {{ args }}

test *args:
    {{ nix }} cargo test {{ args }}

watch *args:
    {{ nix }} watchexec --restart --watch src --watch Cargo.toml -- cargo run --profile {{ profile }} {{ args }}

web *args:
    {{ nix }} trunk serve --address {{ web_address }} --port {{ web_port }} \
        {{ if profile == "release" { "--release" } else { "" } }} {{ args }}
