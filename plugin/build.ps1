# build.ps1 — build the release binary and copy it into the plugin's bin/.
# Run from the crate root:  powershell ./plugin/build.ps1
#
# (Quit any running pet first — it locks the .exe and the build can't relink.)

$ErrorActionPreference = 'Stop'

$crateRoot = Split-Path -Parent $PSScriptRoot          # ...\clawd-pet
$pluginBin = Join-Path $PSScriptRoot 'bin'             # ...\clawd-pet\plugin\bin
$built     = Join-Path $crateRoot 'target/release/cc-petline.exe'

Write-Host "Building release binary..." -ForegroundColor Cyan
Push-Location $crateRoot
try {
    & cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}

if (-not (Test-Path $built)) { throw "expected binary not found: $built" }
if (-not (Test-Path $pluginBin)) { New-Item -ItemType Directory -Path $pluginBin | Out-Null }

Copy-Item $built (Join-Path $pluginBin 'cc-petline.exe') -Force
Write-Host "Copied cc-petline.exe -> plugin/bin/" -ForegroundColor Green
Write-Host "Next: /plugin -> install from local dir '$PSScriptRoot', then enable." -ForegroundColor Green
