param(
  [string]$ProcessName = "moeplay",
  [string]$ExePath = "",
  [string]$TitleRegex = "",
  [string]$OutPath = "..\screenshots\window.png",
  [int]$WaitSeconds = 25
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not [System.IO.Path]::IsPathRooted($OutPath)) {
  $OutPath = Join-Path $scriptDir $OutPath
}
$OutPath = [System.IO.Path]::GetFullPath($OutPath)
New-Item -ItemType Directory -Force (Split-Path -Parent $OutPath) | Out-Null

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class Win32WindowCapture {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }

  [DllImport("user32.dll")]
  public static extern bool SetForegroundWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("dwmapi.dll")]
  public static extern int DwmGetWindowAttribute(IntPtr hwnd, int dwAttribute, out RECT pvAttribute, int cbAttribute);
}
"@

function Get-MatchingWindow {
  $deadline = (Get-Date).AddSeconds($WaitSeconds)
  do {
    $matches = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
      Where-Object { $_.MainWindowHandle -ne 0 }

    if ($TitleRegex) {
      $matches = $matches | Where-Object { $_.MainWindowTitle -match $TitleRegex }
    }

    $window = $matches | Sort-Object StartTime -Descending | Select-Object -First 1
    if ($window) { return $window }
    Start-Sleep -Milliseconds 500
  } while ((Get-Date) -lt $deadline)

  throw "No visible window found for process '$ProcessName' within $WaitSeconds seconds."
}

if ($ExePath) {
  $resolvedExe = Resolve-Path -LiteralPath $ExePath
  $existing = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne 0 } |
    Select-Object -First 1

  if (-not $existing) {
    Start-Process -FilePath $resolvedExe
  }
}

$process = Get-MatchingWindow
$handle = [IntPtr]$process.MainWindowHandle

[Win32WindowCapture]::ShowWindow($handle, 9) | Out-Null
[Win32WindowCapture]::SetForegroundWindow($handle) | Out-Null
Start-Sleep -Milliseconds 900

$rect = New-Object Win32WindowCapture+RECT
$dwmResult = [Win32WindowCapture]::DwmGetWindowAttribute($handle, 9, [ref]$rect, [System.Runtime.InteropServices.Marshal]::SizeOf($rect))
if ($dwmResult -ne 0 -or $rect.Right -le $rect.Left -or $rect.Bottom -le $rect.Top) {
  [Win32WindowCapture]::GetWindowRect($handle, [ref]$rect) | Out-Null
}

$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
if ($width -le 0 -or $height -le 0) {
  throw "Invalid capture bounds: left=$($rect.Left), top=$($rect.Top), right=$($rect.Right), bottom=$($rect.Bottom)"
}

$bitmap = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
  $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
  $bitmap.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
  $graphics.Dispose()
  $bitmap.Dispose()
}

[PSCustomObject]@{
  OutPath = $OutPath
  ProcessId = $process.Id
  ProcessName = $process.ProcessName
  Title = $process.MainWindowTitle
  Bounds = "$($rect.Left),$($rect.Top),$width,$height"
}
