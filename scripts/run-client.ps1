# RemoteDesk Free - Start Desktop Client Application
param(
    [int]$UiPort = 9090,
    [switch]$Daemon,
    [string]$Connect = ""
)

$app_bin = "target\release\remotedesk-app.exe"
if (-not (Test-Path $app_bin)) {
    Write-Host "[Build] Compiling client binary first..." -ForegroundColor Yellow
    .\scripts\build.ps1
}

$argsList = @("--port", $UiPort)
if ($Daemon) {
    $argsList += "--daemon"
}
if ($Connect -ne "") {
    $argsList += @("--connect", $Connect)
}

Write-Host "[Client] Launching RemoteDesk Free Desktop Application..." -ForegroundColor Cyan
& $app_bin @argsList
