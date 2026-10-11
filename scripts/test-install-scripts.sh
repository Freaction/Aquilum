#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
temporary_root="$(mktemp -d)"
trap 'rm -rf "$temporary_root"' EXIT
mock_bin="$temporary_root/bin"
mkdir -p "$mock_bin"

cat > "$mock_bin/uname" <<'EOF'
#!/usr/bin/env bash
case "$1" in
  -s) printf '%s\n' "$MOCK_UNAME_S" ;;
  -m) printf '%s\n' "$MOCK_UNAME_M" ;;
esac
EOF

cat > "$mock_bin/curl" <<'EOF'
#!/usr/bin/env bash
  output=""
  url=""
  printf '%s\n' "$*" >> "$CURL_LOG"
while (($#)); do
  case "$1" in
    -o) output="$2"; shift 2 ;;
    http*) url="$1"; shift ;;
    *) shift ;;
  esac
done
if [[ "$url" == https://api.github.com/repos/Freaction/Aquilum/releases/latest ]]; then
  cp "$RELEASE_JSON" "$output"
else
  printf '%s\n' "$url" >> "$DOWNLOAD_LOG"
  : > "$output"
fi
EOF

cat > "$mock_bin/sudo" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$SUDO_LOG"
EOF

cat > "$mock_bin/hdiutil" <<'EOF'
#!/usr/bin/env bash
if [[ "$1" == attach ]]; then
  mkdir -p "$MOCK_MOUNT/Aquilum.app"
fi
EOF

cat > "$mock_bin/find" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$MOCK_MOUNT/Aquilum.app"
EOF

cat > "$mock_bin/ditto" <<'EOF'
#!/usr/bin/env bash
mkdir -p "$2"
EOF

chmod +x "$mock_bin"/*

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  exit 1
}

assert_equal() {
  [[ "$1" == "$2" ]] || fail "expected '$2', got '$1'"
}

write_release() {
  cat > "$RELEASE_JSON"
}

run_install() {
  PATH="$mock_bin:/usr/bin:/bin" \
    HOME="$temporary_root/home" \
    MOCK_UNAME_S="$MOCK_UNAME_S" \
    MOCK_UNAME_M="$MOCK_UNAME_M" \
    MOCK_MOUNT="$MOCK_MOUNT" \
    RELEASE_JSON="$RELEASE_JSON" \
    CURL_LOG="$CURL_LOG" \
    DOWNLOAD_LOG="$DOWNLOAD_LOG" \
    SUDO_LOG="$SUDO_LOG" \
    OS_RELEASE_FILE="$OS_RELEASE_FILE" \
    bash "$script_dir/install.sh" > "$OUTPUT" 2> "$ERROR"
}

MOCK_UNAME_S=Linux
MOCK_UNAME_M=x86_64
MOCK_MOUNT="$temporary_root/mount"
RELEASE_JSON="$temporary_root/release.json"
DOWNLOAD_LOG="$temporary_root/downloads"
CURL_LOG="$temporary_root/curl"
SUDO_LOG="$temporary_root/sudo"
OUTPUT="$temporary_root/output"
ERROR="$temporary_root/error"
OS_RELEASE_FILE="$temporary_root/os-release"
mkdir -p "$temporary_root/home"
printf 'ID=ubuntu\nVERSION_ID="22.04"\n' > "$OS_RELEASE_FILE"
: > "$DOWNLOAD_LOG"
: > "$CURL_LOG"
: > "$SUDO_LOG"

write_release <<'EOF'
{
  "assets": [
    {
      "browser_download_url": "https://github.com/Freaction/Aquilum/releases/download/v1.2.3/Aquilum_1.2.3_amd64.deb"
    },
    {
      "browser_download_url": "https://github.com/Freaction/Aquilum/releases/download/v1.2.3/Aquilum_1.2.3.AppImage"
    }
  ]
}
EOF
run_install || { cat "$ERROR" >&2; fail "Ubuntu 22.04 installation failed"; }
assert_equal "$(cat "$DOWNLOAD_LOG")" "https://github.com/Freaction/Aquilum/releases/download/v1.2.3/Aquilum_1.2.3_amd64.deb"
[[ "$(cat "$SUDO_LOG")" =~ ^apt-get\ install\ -y\ .*/Aquilum\.deb$ ]] || fail "unexpected Ubuntu package install command"
[[ "$(grep -c -- '--proto-redir =https' "$CURL_LOG")" -eq 2 ]] || fail "Ubuntu downloads do not restrict redirects to HTTPS"

printf 'ID=ubuntu\nVERSION_ID="24.04"\n' > "$OS_RELEASE_FILE"
: > "$DOWNLOAD_LOG"
: > "$SUDO_LOG"
run_install || fail "Ubuntu 24.04 installation failed"
[[ "$(cat "$SUDO_LOG")" =~ ^apt-get\ install\ -y\ .*/Aquilum\.deb$ ]] || fail "unexpected Ubuntu package install command"

printf 'ID=ubuntu\nVERSION_ID="20.04"\n' > "$OS_RELEASE_FILE"
: > "$SUDO_LOG"
if run_install; then fail "Ubuntu before 22.04 was accepted"; fi
[[ ! -s "$SUDO_LOG" ]] || fail "unsupported Ubuntu reached package installation"

printf 'ID=debian\nVERSION_ID="12"\n' > "$OS_RELEASE_FILE"
if run_install; then fail "non-Ubuntu Linux was accepted"; fi

printf 'ID=ubuntu\nVERSION_ID="22.04"\n' > "$OS_RELEASE_FILE"
MOCK_UNAME_M=aarch64
if run_install; then fail "non-x86_64 Linux was accepted"; fi
MOCK_UNAME_M=x86_64

write_release <<'EOF'
{
  "assets": [
    {
      "browser_download_url": "https://attacker.example/Freaction/Aquilum/releases/download/v1.2.3/Aquilum_1.2.3_amd64.deb"
    }
  ]
}
EOF
: > "$SUDO_LOG"
if run_install; then fail "untrusted asset URL was accepted"; fi
[[ ! -s "$SUDO_LOG" ]] || fail "invalid release asset reached package installation"

write_release <<'EOF'
{
  "assets": [
    {
      "browser_download_url": "https://github.com/Freaction/Aquilum/releases/download/v1.2.3/Aquilum_1.2.3_arm64.deb"
    }
  ]
}
EOF
if run_install; then fail "non-amd64 release asset was accepted"; fi

MOCK_UNAME_S=Darwin
MOCK_UNAME_M=arm64
write_release <<'EOF'
{
  "assets": [
    {
      "browser_download_url": "https://github.com/Freaction/Aquilum/releases/download/v1.2.3/Aquilum_aarch64.dmg"
    }
  ]
}
EOF
MOCK_MOUNT="$temporary_root/mac-mount"
run_install || fail "Apple Silicon macOS installation failed"
[[ -d "$temporary_root/home/Applications/Aquilum.app" ]] || fail "macOS app was not copied"
assert_equal "$(tail -n 1 "$DOWNLOAD_LOG")" "https://github.com/Freaction/Aquilum/releases/download/v1.2.3/Aquilum_aarch64.dmg"

MOCK_UNAME_M=x86_64
write_release <<'EOF'
{
  "assets": [
    {
      "browser_download_url": "https://github.com/Freaction/Aquilum/releases/download/v1.2.3/Aquilum_x64.dmg"
    }
  ]
}
EOF
run_install || fail "Intel macOS installation failed"
assert_equal "$(tail -n 1 "$DOWNLOAD_LOG")" "https://github.com/Freaction/Aquilum/releases/download/v1.2.3/Aquilum_x64.dmg"

mkdir -p "$temporary_root/home/Notes"
printf 'keep\n' > "$temporary_root/home/Notes/Keep.md"
PATH="$mock_bin:/usr/bin:/bin" MOCK_UNAME_S=Darwin MOCK_UNAME_M=arm64 HOME="$temporary_root/home" bash "$script_dir/uninstall.sh" >/dev/null
[[ ! -e "$temporary_root/home/Applications/Aquilum.app" ]]
[[ "$(cat "$temporary_root/home/Notes/Keep.md")" == keep ]]
[[ "$(cat "$SUDO_LOG")" != *dpkg* ]] || fail "tests invoked dpkg"
