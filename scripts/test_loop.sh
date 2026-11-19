#!/usr/bin/env bash

log() {
  printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*" >&2
}

args=("$@")
debounce=${DEBOUNCE:-5}
repo_root=$(cd "$(dirname "$0")/.." && pwd)

run_action() {
  bash "$repo_root/scripts/test_remote.sh" "${args[@]}" </dev/null &
}

cleanup() {
  trap - EXIT INT TERM HUP
  bash "$repo_root/scripts/test_remote.sh" --stop "${args[@]}"
  kill 0
}

watch_changes() {
  local pending=0 status
  while :; do
    IFS= read -r -t "$debounce" line
    status=$?
    if ((status == 0)); then
      [[ -f ${line#*:} ]] && pending=1
    elif ((status > 128)); then
      if ((pending)); then
        log 'change detected'
        run_action
      fi
      pending=0
    else
      break
    fi
  done
}

log 'building'
bash "$repo_root/scripts/test_remote.sh" --build-only "${args[@]}" || exit 1

trap cleanup EXIT
trap 'exit 130' INT TERM HUP

log 'running'
run_action

log "watching with debounce $debounce"
nix --extra-experimental-features 'nix-command flakes' run nixpkgs#watchexec -- \
  --emit-events-to=stdio \
  --only-emit-events \
  --project-origin "$repo_root" \
  -w "$repo_root" | watch_changes
