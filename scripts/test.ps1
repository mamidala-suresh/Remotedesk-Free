# RemoteDesk Free - Test Execution Script
$ErrorActionPreference = "Stop"

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  RemoteDesk Free - Automated Verification Test Suites" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

$cargo_bin = "C:\Users\ETP\.cargo\bin"
if (-not (Test-Path "$cargo_bin\cargo.exe")) {
    $cargo_bin = "$env:USERPROFILE\.cargo\bin"
}

$llvm_pkg = Get-ChildItem -Path "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Filter "*LLVM-MinGW*" -Recurse -Directory -ErrorAction SilentlyContinue | Select-Object -First 1
if ($llvm_pkg) {
    $bin_path = Join-Path $llvm_pkg.FullName "bin"
    $env:PATH = "$bin_path;$cargo_bin;" + $env:PATH
} else {
    $env:PATH = "$cargo_bin;" + $env:PATH
}

Write-Host "[1/3] Running remotedesk-core Unit Tests..." -ForegroundColor Yellow
cargo test -p remotedesk-core

Write-Host "`n[2/3] Running remotedesk-capture Screen & Input Tests..." -ForegroundColor Yellow
cargo test -p remotedesk-capture

Write-Host "`n[3/3] Running End-to-End Integration & Security Tests..." -ForegroundColor Yellow
cargo test -p remotedesk-tests

Write-Host "`nAll Test Suites Passed Successfully!" -ForegroundColor Green
