#!/usr/bin/env bash
# Build DuckTrack installers for Linux (.deb, .rpm, .AppImage).
# Output: src-tauri/target/release/bundle/{deb,rpm,appimage}/
set -euo pipefail
cd "$(dirname "$0")/.."

export PATH="$HOME/.cargo/bin:$PATH"

echo "==> Rust toolchain"
rustc --version

echo "==> Installing tauri-cli (if missing)"
command -v cargo-tauri >/dev/null 2>&1 || cargo install tauri-cli --version "^2" --locked

# Skips stripping in linuxdeploy: the strip binary bundled with linuxdeploy
# cannot handle the DT_RELR (.relr.dyn) sections produced by glibc >= 2.41,
# which makes the AppImage build fail on current distros. Safe on all distros.
export NO_STRIP=1

echo "==> Bundling (deb, rpm, AppImage)"
cargo tauri build --bundles deb,rpm,appimage

echo
echo "==> Artifacts"
find src-tauri/target/release/bundle -type f \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \) -exec ls -lh {} \;