# RemoteDesk Free - Start Signaling & Relay Server
param(
    [int]$Port = 8088
)

$server_bin = "target\release\remotedesk-server.exe"
if (-not (Test-Path $server_bin)) {
    Write-Host "[Build] Compiling server binary first..." -ForegroundColor Yellow
    .\scripts\build.ps1
}

$env:PORT = $Port
Write-Host "[Server] Launching RemoteDesk Free Server on port $Port..." -ForegroundColor Cyan
& $server_bin
