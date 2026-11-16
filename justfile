develop := "nix --extra-experimental-features 'flakes nix-command' develop --command"
deck := "DECK_CONFIG='" + env("DECK_CONFIG", "") + "'"
delay := env("DELAY", "3")
nix := if env("IN_NIX_SHELL", "") == "" { develop } else { "" }
profile := env("PROFILE", "dev")
remote := env("REMOTE", "")
remote_dir := env("REMOTE_DIR", "/tmp/deck-rs")
resolution := env("RESOLUTION", "3840x2160")
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
    test -n "{{ remote }}" || { printf '%s\n' 'Set REMOTE=user@host'; exit 1; }
    ssh {{ remote }} "mkdir -p {{ remote_dir }}"
    {{ nix }} {{ rsync }} ./ {{ remote }}:{{ remote_dir }}/
    start="cd {{ remote_dir }} && PROFILE={{ profile }} {{ deck }} \
        XDG_RUNTIME_DIR=\${XDG_RUNTIME_DIR:-/run/user/\$(id -u)} WAYLAND_DISPLAY=\${WAYLAND_DISPLAY:-wayland-0} \
        XDG_SESSION_TYPE=wayland nohup {{ develop }} cargo run --profile {{ profile }} \
        >/tmp/deck-rs.log 2>&1 </dev/null &"
    ssh {{ remote }} "$start"
    while :; do
        {{ nix }} fswatch --recursive --one-batch --exclude='/\.git/' --exclude='/result$' \
            --exclude='/target/' . >/dev/null
        {{ nix }} {{ rsync }} ./ {{ remote }}:{{ remote_dir }}/
        ssh {{ remote }} "pkill -x deck-rs || true; $start"
    done

run *args:
    {{ nix }} cargo run --profile {{ profile }} {{ args }}

shot name="home" *keys:
    #!/bin/sh
    set -eu
    {{ nix }} cargo build --profile {{ profile }}
    dir=$(mktemp -d)
    trap 'kill $sway 2>/dev/null || true; rm -rf "$dir"' EXIT
    mkdir -p target/shots
    printf '%s\n' 'output * mode {{ resolution }} scale 1' 'default_border none' >"$dir/config"
    XDG_RUNTIME_DIR="$dir" WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman \
        {{ nix }} sway --config "$dir/config" >"$dir/sway.log" 2>&1 &
    sway=$!
    for _ in $(seq 50); do ls "$dir"/wayland-* >/dev/null 2>&1 && break; sleep 0.1; done
    display=$(basename "$(ls "$dir"/wayland-* | grep -v lock | head -n1)")
    export XDG_RUNTIME_DIR="$dir" WAYLAND_DISPLAY="$display" XDG_SESSION_TYPE=wayland
    {{ deck }} {{ nix }} cargo run --profile {{ profile }} >target/shots/{{ name }}.log 2>&1 &
    sleep {{ delay }}
    for key in {{ keys }}; do {{ nix }} wtype -s 300 -k "$key" -s 300; done
    sleep 0.5
    {{ nix }} grim target/shots/{{ name }}.png
    printf '%s\n' target/shots/{{ name }}.png

test *args:
    {{ nix }} cargo test {{ args }}

watch *args:
    {{ nix }} watchexec --restart --watch src --watch assets --watch Cargo.toml -- \
        cargo run --profile {{ profile }} {{ args }}
