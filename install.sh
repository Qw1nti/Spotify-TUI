#!/usr/bin/env bash
set -euo pipefail

APP_NAME="spotifytui"
REPO_SLUG_DEFAULT="Qw1nti/Spotify-TUI"
REPO_REF_DEFAULT="main"

say() { printf '%s\n' "$*"; }
warn() { printf 'Warning: %s\n' "$*" >&2; }

ask() {
  local prompt="$1"
  local default="${2:-}"
  local reply
  if [ -n "$default" ]; then
    printf '%s [%s]: ' "$prompt" "$default"
  else
    printf '%s: ' "$prompt"
  fi
  IFS= read -r reply || true
  reply="${reply:-$default}"
  printf '%s' "$reply"
}
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
path_contains_dir() {
  case ":$PATH:" in
    *":$1:"*) return 0 ;;
    *) return 1 ;;
  esac
}

if [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi

for tool in cargo curl tar install find mktemp id; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    case "$tool" in
      cargo)
        say "Rust toolchain missing."
        say "Install Rust first: https://rustup.rs"
        ;;
      *)
        say "$tool missing."
        ;;
    esac
    exit 1
  fi
done

if ! ask_yn "Install spotifytui now?" "Y"; then
  say "Abort."
  exit 0
fi

repo_ref="${SPOTIFYTUI_REF:-$REPO_REF_DEFAULT}"
source_dir="${SPOTIFYTUI_SOURCE_DIR:-}"
tmpdir="$(mktemp -d)"
cleanup() { rm -rf "$tmpdir"; }
trap cleanup EXIT

if [ -n "$source_dir" ] && [ -d "$source_dir" ]; then
  workdir="$source_dir"
else
  repo_slug="${SPOTIFYTUI_REPO:-$REPO_SLUG_DEFAULT}"

  archive_url="https://codeload.github.com/${repo_slug}/tar.gz/refs/heads/${repo_ref}"
  say "Downloading $repo_slug@$repo_ref"
  curl -fsSL "$archive_url" -o "$tmpdir/source.tar.gz"
  tar -xzf "$tmpdir/source.tar.gz" -C "$tmpdir"
  workdir="$(find "$tmpdir" -mindepth 1 -maxdepth 1 -type d -name '*-*' -print -quit)"
fi

if [ -z "${workdir:-}" ] || [ ! -f "$workdir/Cargo.toml" ]; then
  say "Could not find Cargo.toml in source."
  exit 1
fi

install_global=true
if ask_yn "Install globally to /usr/local/bin?" "Y"; then
  prefix="/usr/local"
else
  install_global=false
  prefix="${HOME}/.local"
fi

if [ "$install_global" = true ] && [ "$(id -u)" -ne 0 ] && ! command -v sudo >/dev/null 2>&1; then
  say "sudo missing; installing to $HOME/.local/bin instead."
  install_global=false
  prefix="${HOME}/.local"
fi

bin_dir="${prefix}/bin"
binary_path="${bin_dir}/${APP_NAME}"
data_dir="$HOME/.local/share/spotifytui"
mkdir -p "$HOME/.config/spotifytui" "$data_dir" "$HOME/.cache/spotifytui"

if [ "$install_global" = false ] || [ "$prefix" = "$HOME/.local" ]; then
  mkdir -p "$bin_dir"
fi

if [ -e "$binary_path" ]; then
  say "Updating existing install at $binary_path"
else
  say "Installing to $binary_path"
fi

say "Building release..."
(cd "$workdir" && cargo build --release --bin spt)

if [ "$install_global" = true ] && [ "$(id -u)" -ne 0 ]; then
  sudo install -Dm755 "$workdir/target/release/spt" "$binary_path"
else
  install -Dm755 "$workdir/target/release/spt" "$binary_path"
fi

say "Installed to: $binary_path"
say "App data dir: $data_dir"
if ! path_contains_dir "$bin_dir"; then
  warn "$bin_dir is not on PATH."
  warn "Add it to PATH or run $binary_path directly."
fi
say "Run: spotifytui"
