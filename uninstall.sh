#!/usr/bin/env bash
set -euo pipefail

say() { printf '%s\n' "$*"; }

ask_yn() {
  local prompt="$1"
  local default="${2:-Y}"
  local reply
  while true; do
    if [ "$default" = "Y" ]; then
      printf '%s [Y/n]: ' "$prompt"
    else
      printf '%s [y/N]: ' "$prompt"
    fi
    IFS= read -r reply || true
    reply="${reply:-$default}"
    case "$reply" in
      Y|y|yes|YES) return 0 ;;
      N|n|no|NO) return 1 ;;
    esac
  done
}

remove_path() {
  local path="$1"
  local use_sudo="${2:-false}"

  if [ ! -e "$path" ]; then
    return 0
  fi

  if [ "$use_sudo" = "true" ] && [ "$(id -u)" -ne 0 ]; then
    if command -v sudo >/dev/null 2>&1; then
      sudo rm -f "$path"
      return 0
    fi
    say "Skipping $path (need sudo)."
    return 0
  fi

  rm -f "$path"
}

remove_dir() {
  local path="$1"
  if [ -d "$path" ]; then
    rm -rf "$path"
  fi
}

if ! ask_yn "Uninstall spotifytui and remove local data?" "Y"; then
  say "Abort."
  exit 0
fi

say "Removing installed command..."
remove_path "/usr/local/bin/spotifytui" true
remove_path "$HOME/.local/bin/spotifytui" false

say "Removing app data..."
remove_dir "$HOME/.config/spotifytui"
remove_dir "$HOME/.cache/spotifytui"
remove_dir "$HOME/.local/share/spotifytui"

say "Done."
