develop := "nix --extra-experimental-features flakes \
    --extra-experimental-features nix-command develop --command"
deck := "DECK_CONFIG='" + env("DECK_CONFIG", "") + "'"
delay := env("DELAY", "3")
nix := if env("IN_NIX_SHELL", "") == "" { develop } else { "" }
profile := env("PROFILE", "dev")
remote := env("REMOTE", "")
remote_dir := env("REMOTE_DIR", "/tmp/deck-rs")
resolution := env("RESOLUTION", "3840x2160")
rsync := "rsync -a --delete --exclude=.git/ --exclude=result --exclude=target/ \
    --exclude=.deck-rs.pid --exclude=deck-rs.log"

default:
    @just --list

build *args:
    {{ nix }} cargo build --profile {{ profile }} {{ args }}

check: lint

format:
    {{ nix }} cargo fmt
    {{ nix }} nixfmt flake.nix

lint:
    {{ nix }} cargo fmt --check
    {{ nix }} cargo clippy --all-targets -- -D warnings
    {{ nix }} nixfmt --check flake.nix

remote:
    #!/usr/bin/env bash
    set -euo pipefail
    test -n "{{ remote }}" || { printf '%s\n' 'Set REMOTE=user@host'; exit 1; }
    stop="pidfile={{ remote_dir }}/.deck-rs.pid; \
        if [ -r \$pidfile ]; then \
            pid=\$(cat \$pidfile); \
            case \$pid in ''|*[!0-9]*) ;; *) \
                if [ \"\$(readlink -f /proc/\$pid/cwd 2>/dev/null)\" = '{{ remote_dir }}' ]; then \
                    kill -TERM -- -\$pid 2>/dev/null || true; \
                    i=0; \
                    while kill -0 -- -\$pid 2>/dev/null; do \
                        i=\$((i + 1)); \
                        [ \$i -lt 50 ] || { kill -KILL -- -\$pid 2>/dev/null || true; break; }; \
                        sleep 0.1; \
                    done; \
                fi ;; \
            esac; \
            rm -f \$pidfile; \
        fi"
    start="cd {{ remote_dir }} || exit; \
        setsid bash -c 'printf \"%s\\n\" \"\$\$\" >\"\$1\"; shift; exec \"\$@\"' \
            _ '{{ remote_dir }}/.deck-rs.pid' env {{ deck }} PROFILE={{ profile }} \
            XDG_RUNTIME_DIR=\${XDG_RUNTIME_DIR:-/run/user/\$(id -u)} \
            WAYLAND_DISPLAY=\${WAYLAND_DISPLAY:-wayland-0} XDG_SESSION_TYPE=wayland \
            {{ develop }} cargo run --profile {{ profile }} & \
        pid=\$!; \
        cleanup() { \
            signal=\$1; \
            trap - INT TERM; \
            kill -TERM -- -\$pid 2>/dev/null || true; \
            wait \$pid 2>/dev/null || true; \
            rm -f {{ remote_dir }}/.deck-rs.pid; \
            exit \$signal; \
        }; \
        trap 'cleanup 130' INT; \
        trap 'cleanup 143' TERM; \
        if wait \$pid; then status=0; else status=\$?; fi; \
        rm -f {{ remote_dir }}/.deck-rs.pid; \
        trap - INT TERM; \
        exit \$status"
    ssh "{{ remote }}" "mkdir -p {{ remote_dir }}"
    {{ rsync }} ./ "{{ remote }}:{{ remote_dir }}/"
    ssh "{{ remote }}" "$stop"
    ssh -t "{{ remote }}" "$start"

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
    {{ deck }} SLINT_BACKEND=winit-software {{ nix }} cargo run --profile {{ profile }} \
        >target/shots/{{ name }}.log 2>&1 &
    sleep {{ delay }}
    for key in {{ keys }}; do {{ nix }} wtype -s 300 -k "$key" -s 300; done
    sleep 0.5
    {{ nix }} grim target/shots/{{ name }}.png
    printf '%s\n' target/shots/{{ name }}.png

watch *args:
    {{ nix }} watchexec --restart --watch src --watch assets \
        --watch src/build.rs --watch Cargo.toml -- \
        cargo run --profile {{ profile }} {{ args }}
