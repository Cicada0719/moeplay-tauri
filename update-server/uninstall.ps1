# MoePlay Update Server uninstaller. Keeps www data; removes task/firewall/process.
$ErrorActionPreference = "SilentlyContinue"
Unregister-ScheduledTask -TaskName "MoePlayUpdateServer" -Confirm:$false
Get-NetFirewallRule -DisplayName "MoePlayUpdateServer" | Remove-NetFirewallRule
Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" | Where-Object { $_.CommandLine -like "*MoePlayUpdateServer*server.ps1*" } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
Write-Host "MoePlayUpdateServer removed. Data kept at C:\MoePlayUpdateServer\www (delete manually if unwanted)."
