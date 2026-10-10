#!/usr/bin/env bash
set -euo pipefail

REPO="Freaction/Aquilum"
APP_DIR="$HOME/Applications"
APP_PATH="$APP_DIR/Aquilum.app"

if [[ "$(uname -s)" == Linux ]]; then
  os_release_file="${OS_RELEASE_FILE:-/etc/os-release}"
  if [[ "$(uname -m)" != x86_64 || ! -r "$os_release_file" ]]; then
    printf 'Aquilum Linux installer supports Ubuntu 22.04 or later on x86_64 only.\n' >&2
    exit 1
  fi
  . "$os_release_file"
  if [[ "${ID:-}" != ubuntu || ! "${VERSION_ID:-}" =~ ^[0-9]+\.[0-9]+$ ]]; then
    printf 'Aquilum Linux installer supports Ubuntu 22.04 or later on x86_64 only.\n' >&2
    exit 1
  fi
  version_major="${VERSION_ID%%.*}"
  version_minor="${VERSION_ID#*.}"
  if (( version_major < 22 || (version_major == 22 && version_minor < 4) )); then
    printf 'Aquilum Linux installer supports Ubuntu 22.04 or later on x86_64 only.\n' >&2
    exit 1
  fi

  work_dir="$(mktemp -d)"
  trap 'rm -rf "$work_dir"' EXIT
  curl -fsSL --proto '=https' --proto-redir '=https' "https://api.github.com/repos/$REPO/releases/latest" -o "$work_dir/release.json"
  download_url="$(sed -n 's/^[[:space:]]*"browser_download_url":[[:space:]]*"\(.*\)",\{0,1\}$/\1/p' "$work_dir/release.json" | grep -E '^https://github.com/Freaction/Aquilum/releases/download/[^/]+/[^/]+_amd64\.deb$' | head -n 1 || true)"
  if [[ -z "$download_url" ]]; then
    printf 'Latest GitHub release has no valid Ubuntu x86_64 DEB package.\n' >&2
    exit 1
  fi

  curl -fL --proto '=https' --proto-redir '=https' --tlsv1.2 "$download_url" -o "$work_dir/Aquilum.deb"
  sudo apt-get install -y "$work_dir/Aquilum.deb"
  printf 'Installed Aquilum from %s\n' "$download_url"
  exit 0
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  printf 'Aquilum curl installer currently supports macOS only.\n' >&2
  exit 1
fi

case "$(uname -m)" in
  arm64) target="aarch64" ;;
  x86_64) target="x64" ;;
  *) printf 'Aquilum DMG installer supports Apple Silicon and Intel Macs only.\n' >&2; exit 1 ;;
esac

work_dir="$(mktemp -d)"
mount_dir="$work_dir/mount"
mkdir "$mount_dir"
mounted=false
cleanup() {
  if [[ "$mounted" == true ]]; then
    hdiutil detach "$mount_dir" -quiet || true
  fi
  rm -rf "$work_dir"
}
trap cleanup EXIT

curl -fsSL --proto '=https' --proto-redir '=https' "https://api.github.com/repos/$REPO/releases/latest" -o "$work_dir/release.json"
download_url="$(sed -n 's/^[[:space:]]*"browser_download_url":[[:space:]]*"\(.*\)",\{0,1\}$/\1/p' "$work_dir/release.json" | grep "_${target}\.dmg$" | head -n 1 || true)"
if [[ -z "$download_url" ]]; then
  printf 'Latest GitHub release has no macOS %s DMG.\n' "$target" >&2
  exit 1
fi

curl -fL --proto '=https' --proto-redir '=https' --tlsv1.2 "$download_url" -o "$work_dir/Aquilum.dmg"
hdiutil attach -nobrowse -readonly -mountpoint "$mount_dir" "$work_dir/Aquilum.dmg" >/dev/null
mounted=true
source_app="$(find "$mount_dir" -maxdepth 3 -type d -name 'Aquilum.app' -print -quit)"
if [[ -z "$source_app" ]]; then
  printf 'Aquilum.app was not found in the downloaded DMG.\n' >&2
  exit 1
fi

mkdir -p "$APP_DIR"
rm -rf "$APP_PATH"
ditto "$source_app" "$APP_PATH"
printf 'Installed Aquilum to %s\n' "$APP_PATH"
