#!/usr/bin/env bash
set -euo pipefail

say() { printf '%s\n' "$*"; }
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

if [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
  say "Rust toolchain missing."
  say "Install Rust first: https://rustup.rs"
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  say "curl missing."
  exit 1
fi

if ! command -v tar >/dev/null 2>&1; then
  say "tar missing."
  exit 1
fi

if ! ask_yn "Install spotifytui now?" "Y"; then
  say "Abort."
  exit 0
fi

repo_ref="${SPOTIFYTUI_REF:-main}"
source_dir="${SPOTIFYTUI_SOURCE_DIR:-}"
default_repo_slug="Qw1nti/Spotify-TUI"
tmpdir="$(mktemp -d)"
cleanup() { rm -rf "$tmpdir"; }
trap cleanup EXIT

if [ -n "$source_dir" ] && [ -d "$source_dir" ]; then
  workdir="$source_dir"
else
  repo_slug="${SPOTIFYTUI_REPO:-$default_repo_slug}"

  archive_url="https://codeload.github.com/${repo_slug}/tar.gz/refs/heads/${repo_ref}"
  say "Downloading $repo_slug@$repo_ref"
  curl -fsSL "$archive_url" -o "$tmpdir/source.tar.gz"
  tar -xzf "$tmpdir/source.tar.gz" -C "$tmpdir"
  workdir="$(find "$tmpdir" -maxdepth 1 -type d -name '*-*' | head -n 1)"
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

bin_dir="${prefix}/bin"
binary_path="${bin_dir}/spotifytui"
mkdir -p "$bin_dir"

say "Building release..."
(cd "$workdir" && cargo build --release --bin spt)

if [ "$install_global" = true ] && [ "$(id -u)" -ne 0 ]; then
  if command -v sudo >/dev/null 2>&1; then
    sudo install -Dm755 "$workdir/target/release/spt" "$binary_path"
  else
    say "sudo missing; installing to $HOME/.local/bin instead."
    mkdir -p "$HOME/.local/bin"
    install -Dm755 "$workdir/target/release/spt" "$HOME/.local/bin/spotifytui"
    binary_path="$HOME/.local/bin/spotifytui"
  fi
else
  install -Dm755 "$workdir/target/release/spt" "$binary_path"
fi

say "Installed to: $binary_path"
say "Run: spotifytui"
