# Build DuckTrack installers for Windows (.exe built by NSIS, .msi by WiX).
# Run on Windows. Output: src-tauri/target/release/bundle/{nsis,msi}/
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

Write-Output "==> Installing tauri-cli (if missing)"
if (-not (Get-Command cargo-tauri -ErrorAction SilentlyContinue)) {
    cargo install tauri-cli --version "^2" --locked
}

Write-Output "==> Bundling (nsis, msi)"
cargo tauri build --bundles nsis,msi

Write-Output ""
Write-Output "==> Artifacts"
Get-ChildItem -Recurse "src-tauri/target/release/bundle" -Include *.exe,*.msi | ForEach-Object { $_.FullName }