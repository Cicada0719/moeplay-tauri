# MoePlay Update Server installer. One-click: firewall rule + startup scheduled task + token.
# Run install-update-server.cmd (auto-elevates). ASCII-only strings for cmd-launcher safety.
param(
    [int]$Port = 8788,
    [switch]$KeepExistingToken
)
$ErrorActionPreference = "Stop"

function Test-Admin {
    return ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

$Src = Split-Path -Parent $MyInvocation.MyCommand.Path
$Dest = "C:\MoePlayUpdateServer"
$TaskName = "MoePlayUpdateServer"

# self-elevate
if (-not (Test-Admin)) {
    Write-Host "Requesting administrator privileges..."
    Start-Process powershell -Verb RunAs -ArgumentList @("-NoProfile","-ExecutionPolicy","Bypass","-File","`"$PSCommandPath`"","-Port",$Port)
    exit
}

New-Item -ItemType Directory -Force -Path $Dest | Out-Null
Copy-Item -Force (Join-Path $Src "server.ps1") $Dest
Copy-Item -Force (Join-Path $Src "index.html") $Dest
Copy-Item -Force (Join-Path $Src "uninstall.ps1") $Dest

$ConfigPath = Join-Path $Dest "server.config.json"
$Token = $null
if ($KeepExistingToken -and (Test-Path $ConfigPath)) {
    $Token = (Get-Content -Raw -Encoding UTF8 $ConfigPath | ConvertFrom-Json).token
}
if (-not $Token) { $Token = [guid]::NewGuid().ToString("N") + [guid]::NewGuid().ToString("N") }

$cfg = @{ port = $Port; token = $Token } | ConvertTo-Json
[System.IO.File]::WriteAllText($ConfigPath, $cfg, (New-Object System.Text.UTF8Encoding($true)))

# firewall (idempotent)
if (-not (Get-NetFirewallRule -DisplayName "MoePlayUpdateServer" -ErrorAction SilentlyContinue)) {
    New-NetFirewallRule -DisplayName "MoePlayUpdateServer" -Direction Inbound -Action Allow -Protocol TCP -LocalPort $Port | Out-Null
}

# urlacl reserved by HttpListener when running as admin task; no action needed.

# stop old instance
$action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$Dest\server.ps1`"" -WorkingDirectory $Dest
$trigger = New-ScheduledTaskTrigger -AtStartup
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1) -ExecutionTimeLimit (New-TimeSpan -Seconds 0)
$principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest
Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
# kill leftovers holding the port
Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" | Where-Object { $_.CommandLine -like "*$Dest*server.ps1*" } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Settings $settings -Principal $principal | Out-Null
Start-ScheduledTask -TaskName $TaskName

# LAN IP for user info
$ips = (Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -like "192.168.*" -or $_.IPAddress -like "10.*" -or $_.IPAddress -like "172.1*" }).IPAddress -join ", "

$PubUrl = "http://<server-ip>:$Port"
Write-Host ""
Write-Host "============================================================" -ForegroundColor Green
Write-Host " MoePlay update server installed."
Write-Host " Download page : $PubUrl  (LAN IPs on this box: $ips)"
Write-Host " Update endpoint: $PubUrl/latest.json"
Write-Host " Publish token  : $Token"
Write-Host " Token saved at : $Dest\server.config.json"
Write-Host ""
Write-Host " On your dev PC, put this token into moeplay-publish-config.json"
Write-Host " as ""publishToken"", then run: npm run publish:update"
Write-Host "============================================================" -ForegroundColor Green
Read-Host "Press Enter to close"
