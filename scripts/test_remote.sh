#!/usr/bin/env bash
# shellcheck disable=SC2029

get_parameter() {
  local flag=$1
  shift
  while [[ $# -gt 0 ]]; do
    if [[ $1 == "$flag" ]]; then
      if [[ -n ${2-} && $2 != -* ]]; then
        printf '%s\n' "$2"
      fi
      return 0
    fi
    shift
  done
  return 1
}

ssh_opts=(
  # reuses ssh connection
  -o ControlMaster=auto
  -o ControlPath=/tmp/dots-ssh-%C
  # sets connection time to live
  -o ControlPersist=60
  # prevents host key change errors
  -o StrictHostKeyChecking=no
  -o UserKnownHostsFile=/dev/null
)

if ! get_parameter --remote "$@" >/dev/null; then
  printf 'Usage: test_remote.sh --remote U@HOST\n' >&2
  exit 1
fi

remote=$(get_parameter --remote "$@")
# starts ssh connection so ssh password prompt only happens here
if [[ -n $remote ]] && ! ssh "${ssh_opts[@]}" "$remote" true; then
  exit 1
fi

if get_parameter --stop "$@" >/dev/null; then
  ssh "${ssh_opts[@]}" "$remote" 'for p in /proc/[0-9]*; do
    [ "$(readlink "$p/cwd" 2>/dev/null)" = /tmp/deck-rs ] && kill "${p#/proc/}" 2>/dev/null
  done'
  exit 0
fi

remote_dir='/tmp/deck-rs/'
repo_root=$(cd "$(dirname "$0")/.." && pwd)

cargo_cmd=run
if get_parameter --build-only "$@" >/dev/null; then
  cargo_cmd=build
fi

ssh "${ssh_opts[@]}" "$remote" "mkdir -p $remote_dir"
rsync -a --delete \
  -e "ssh ${ssh_opts[*]}" \
  --exclude=.git/ \
  --exclude=result \
  --exclude=target/ \
  "$repo_root/" "$remote:$remote_dir"
ssh -t "${ssh_opts[@]}" "$remote" \
  "cd $remote_dir && pkill -x deck-rs; env XDG_RUNTIME_DIR=\${XDG_RUNTIME_DIR:-/run/user/\$(id -u)} \
  WAYLAND_DISPLAY=\${WAYLAND_DISPLAY:-wayland-0} XDG_SESSION_TYPE=wayland \
  flock -w 10 /tmp/deck-rs.lock \
  nix --extra-experimental-features 'nix-command flakes' develop --command \
  cargo $cargo_cmd --profile dev"
