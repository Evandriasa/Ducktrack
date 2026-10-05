# Build DuckTrack installers for macOS (.app, .dmg).
# Run on macOS. Output: src-tauri/target/release/bundle/{macos,dmg}/
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

Write-Output "==> Installing tauri-cli (if missing)"
if (-not (Get-Command cargo-tauri -ErrorAction SilentlyContinue)) {
    cargo install tauri-cli --version "^2" --locked
}

Write-Output "==> Bundling (app, dmg)"
cargo tauri build --bundles app,dmg

Write-Output ""
Write-Output "==> Artifacts"
Get-ChildItem -Recurse "src-tauri/target/release/bundle" -Include *.app,*.dmg | ForEach-Object { $_.FullName }