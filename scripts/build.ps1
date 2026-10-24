# RemoteDesk Free - Windows Automated Build Script
$ErrorActionPreference = "Stop"

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  RemoteDesk Free - Building Standalone Windows Executables" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

$cargo_bin = "C:\Users\ETP\.cargo\bin"
if (-not (Test-Path "$cargo_bin\cargo.exe")) {
    $cargo_bin = "$env:USERPROFILE\.cargo\bin"
}

$llvm_bin_dir = Get-ChildItem -Path "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Filter "x86_64-w64-mingw32-gcc.exe" -Recurse -File -ErrorAction SilentlyContinue | Select-Object -First 1
if ($llvm_bin_dir) {
    $bin_path = $llvm_bin_dir.DirectoryName
    $env:PATH = "$bin_path;$cargo_bin;" + $env:PATH
    Write-Host "[Env] Using LLVM-MinGW toolchain: $bin_path" -ForegroundColor Green
    Copy-Item (Join-Path $bin_path "libunwind.dll") "libunwind.dll" -Force
} else {
    $env:PATH = "$cargo_bin;" + $env:PATH
}

Write-Host "[Build] Compiling optimized release binaries (target: x86_64-pc-windows-gnullvm)..." -ForegroundColor Yellow
cargo build --target x86_64-pc-windows-gnullvm --release

$app_bin = "target\x86_64-pc-windows-gnullvm\release\remotedesk-app.exe"
$server_bin = "target\x86_64-pc-windows-gnullvm\release\remotedesk-server.exe"

if ((Test-Path $app_bin) -and (Test-Path $server_bin)) {
    Copy-Item $app_bin "RemoteDesk-Free.exe" -Force
    Copy-Item $server_bin "RemoteDesk-Server.exe" -Force
    New-Item -ItemType Directory -Force -Path "dist" | Out-Null
    Copy-Item $app_bin "dist\RemoteDesk-Free.exe" -Force
    Copy-Item $server_bin "dist\RemoteDesk-Server.exe" -Force
    if (Test-Path "libunwind.dll") {
        Copy-Item "libunwind.dll" "dist\libunwind.dll" -Force
    }

    # Compile Windows Setup Installer if Inno Setup is available
    $iscc = Get-ChildItem -Path "C:\Program Files (x86)\Inno Setup*", "C:\Program Files\Inno Setup*", "$env:LOCALAPPDATA\Programs\Inno Setup*" -Filter "ISCC.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($iscc) {
        Write-Host "[Installer] Generating Windows Setup Installation EXE..." -ForegroundColor Yellow
        & $iscc.FullName "installer.iss" | Out-Null
        if (Test-Path "dist\RemoteDesk-Free-Setup-v1.1.0.1.exe") {
            Copy-Item "dist\RemoteDesk-Free-Setup-v1.1.0.1.exe" "RemoteDesk-Free-Setup-v1.1.0.1.exe" -Force
            Write-Host "  - Windows Setup Installer: RemoteDesk-Free-Setup-v1.1.0.1.exe" -ForegroundColor Green
        }
    }

    $app_size = [math]::Round((Get-Item $app_bin).Length / 1MB, 2)
    $srv_size = [math]::Round((Get-Item $server_bin).Length / 1MB, 2)
    Write-Host "`nBuild Successful!" -ForegroundColor Green
    Write-Host "  - Client Desktop App: RemoteDesk-Free.exe ($app_size MB)" -ForegroundColor White
    Write-Host "  - Signaling Server:   RemoteDesk-Server.exe ($srv_size MB)" -ForegroundColor White
} else {
    Write-Error "Build failed: output binaries not found."
}
