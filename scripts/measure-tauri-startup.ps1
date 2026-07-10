[CmdletBinding()]
param(
  [Parameter(Mandatory = $true, Position = 0)]
  [string]$ExePath,

  [Alias("Repeats")]
  [ValidateRange(1, 100)]
  [int]$Repeat = 10,

  [string[]]$Arguments = @(),

  [ValidateRange(1, 300)]
  [int]$TimeoutSeconds = 30,

  [ValidateRange(1, 120)]
  [int]$IdleSeconds = 5,

  [ValidateRange(25, 2000)]
  [int]$PollMilliseconds = 100,

  [ValidateRange(100, 60000)]
  [double]$MaxColdStartMs = 2000,

  [ValidateRange(100, 60000)]
  [double]$MaxFirstContentMs = 2500,

  [ValidateRange(32, 4096)]
  [double]$MaxIdleRssMb = 300,

  [string]$OutputDirectory = (Join-Path (Get-Location) "artifacts\performance"),

  [string]$JsonPath,

  [string]$MarkdownPath,

  [switch]$NoFail
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
  throw "measure-tauri-startup.ps1 requires Windows because it probes native windows and process working sets."
}

$ResolvedExe = (Resolve-Path -LiteralPath $ExePath).Path
$Exe = Get-Item -LiteralPath $ResolvedExe
if ($Exe.PSIsContainer -or $Exe.Extension -ne ".exe") {
  throw "ExePath must point to a Windows .exe file: $ResolvedExe"
}

$Timestamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
if (-not $JsonPath) { $JsonPath = Join-Path $OutputDirectory "tauri-startup-$Timestamp.json" }
if (-not $MarkdownPath) { $MarkdownPath = Join-Path $OutputDirectory "tauri-startup-$Timestamp.md" }
$JsonPath = [System.IO.Path]::GetFullPath($JsonPath)
$MarkdownPath = [System.IO.Path]::GetFullPath($MarkdownPath)

if (-not ("MoePlayWindowProbe" -as [type])) {
Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public sealed class MoePlayWindowSample
{
    public bool Captured { get; set; }
    public int Width { get; set; }
    public int Height { get; set; }
    public int UniqueColors { get; set; }
    public double LuminanceRange { get; set; }
    public double ChangedPixelRatio { get; set; }
    public uint Dpi { get; set; }
    public string Error { get; set; }
}

public static class MoePlayWindowProbe
{
    private delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [StructLayout(LayoutKind.Sequential)]
    private struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);
    [DllImport("user32.dll")]
    private static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")]
    private static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")]
    private static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint flags);
    [DllImport("user32.dll")]
    private static extern uint GetDpiForWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    private static extern IntPtr GetDC(IntPtr hWnd);
    [DllImport("user32.dll")]
    private static extern int ReleaseDC(IntPtr hWnd, IntPtr hDC);
    [DllImport("gdi32.dll")]
    private static extern IntPtr CreateCompatibleDC(IntPtr hdc);
    [DllImport("gdi32.dll")]
    private static extern bool DeleteDC(IntPtr hdc);
    [DllImport("gdi32.dll")]
    private static extern IntPtr CreateCompatibleBitmap(IntPtr hdc, int width, int height);
    [DllImport("gdi32.dll")]
    private static extern IntPtr SelectObject(IntPtr hdc, IntPtr obj);
    [DllImport("gdi32.dll")]
    private static extern bool DeleteObject(IntPtr obj);
    [DllImport("gdi32.dll")]
    private static extern uint GetPixel(IntPtr hdc, int x, int y);

    public static IntPtr FindLargestVisibleWindow(int processId)
    {
        IntPtr best = IntPtr.Zero;
        long bestArea = 0;
        EnumWindows((hWnd, _) => {
            if (!IsWindowVisible(hWnd)) return true;
            uint owner;
            GetWindowThreadProcessId(hWnd, out owner);
            if (owner != (uint)processId) return true;
            RECT rect;
            if (!GetWindowRect(hWnd, out rect)) return true;
            long width = Math.Max(0, rect.Right - rect.Left);
            long height = Math.Max(0, rect.Bottom - rect.Top);
            long area = width * height;
            if (area > bestArea) { bestArea = area; best = hWnd; }
            return true;
        }, IntPtr.Zero);
        return best;
    }

    private static int Red(uint color) { return (int)(color & 0xff); }
    private static int Green(uint color) { return (int)((color >> 8) & 0xff); }
    private static int Blue(uint color) { return (int)((color >> 16) & 0xff); }

    public static MoePlayWindowSample Capture(IntPtr hWnd)
    {
        var result = new MoePlayWindowSample();
        IntPtr screenDc = IntPtr.Zero;
        IntPtr memoryDc = IntPtr.Zero;
        IntPtr bitmap = IntPtr.Zero;
        IntPtr previous = IntPtr.Zero;
        try
        {
            RECT rect;
            if (!GetWindowRect(hWnd, out rect)) { result.Error = "GetWindowRect failed"; return result; }
            int width = Math.Max(1, rect.Right - rect.Left);
            int height = Math.Max(1, rect.Bottom - rect.Top);
            result.Width = width;
            result.Height = height;
            result.Dpi = GetDpiForWindow(hWnd);

            screenDc = GetDC(IntPtr.Zero);
            memoryDc = CreateCompatibleDC(screenDc);
            bitmap = CreateCompatibleBitmap(screenDc, width, height);
            if (screenDc == IntPtr.Zero || memoryDc == IntPtr.Zero || bitmap == IntPtr.Zero) {
                result.Error = "GDI allocation failed";
                return result;
            }
            previous = SelectObject(memoryDc, bitmap);
            if (!PrintWindow(hWnd, memoryDc, 2)) { result.Error = "PrintWindow failed"; return result; }

            var colors = new HashSet<uint>();
            double minLum = 255;
            double maxLum = 0;
            int changed = 0;
            int total = 0;
            uint reference = GetPixel(memoryDc, Math.Min(width - 1, 2), Math.Min(height - 1, 2));
            const int columns = 24;
            const int rows = 16;
            for (int yIndex = 0; yIndex < rows; yIndex++)
            {
                int y = Math.Min(height - 1, (int)Math.Round((height - 1) * (yIndex + 0.5) / rows));
                for (int xIndex = 0; xIndex < columns; xIndex++)
                {
                    int x = Math.Min(width - 1, (int)Math.Round((width - 1) * (xIndex + 0.5) / columns));
                    uint color = GetPixel(memoryDc, x, y);
                    colors.Add(color);
                    int red = Red(color), green = Green(color), blue = Blue(color);
                    double lum = 0.2126 * red + 0.7152 * green + 0.0722 * blue;
                    minLum = Math.Min(minLum, lum);
                    maxLum = Math.Max(maxLum, lum);
                    int distance = Math.Abs(red - Red(reference)) + Math.Abs(green - Green(reference)) + Math.Abs(blue - Blue(reference));
                    if (distance >= 24) changed++;
                    total++;
                }
            }
            result.UniqueColors = colors.Count;
            result.LuminanceRange = Math.Round(maxLum - minLum, 2);
            result.ChangedPixelRatio = total == 0 ? 0 : Math.Round((double)changed / total, 4);
            result.Captured = true;
            return result;
        }
        catch (Exception ex)
        {
            result.Error = ex.GetType().Name + ": " + ex.Message;
            return result;
        }
        finally
        {
            if (previous != IntPtr.Zero && memoryDc != IntPtr.Zero) SelectObject(memoryDc, previous);
            if (bitmap != IntPtr.Zero) DeleteObject(bitmap);
            if (memoryDc != IntPtr.Zero) DeleteDC(memoryDc);
            if (screenDc != IntPtr.Zero) ReleaseDC(IntPtr.Zero, screenDc);
        }
    }
}
"@
}

function Get-ExactExecutableProcesses {
  param([string]$Path)
  $Normalized = [System.IO.Path]::GetFullPath($Path)
  return @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object {
    $_.ExecutablePath -and [string]::Equals([System.IO.Path]::GetFullPath($_.ExecutablePath), $Normalized, [System.StringComparison]::OrdinalIgnoreCase)
  })
}

$Existing = @(Get-ExactExecutableProcesses -Path $ResolvedExe)
if ($Existing.Count -gt 0) {
  $Pids = ($Existing | ForEach-Object ProcessId) -join ", "
  throw "Refusing to measure while the same executable is already running (PID: $Pids). Close it first so timings and cleanup remain attributable to script-created processes only."
}

function Add-OwnedProcessIdentity {
  param(
    [System.Collections.Generic.Dictionary[int, long]]$Owned,
    [int]$ProcessId
  )
  if ($ProcessId -le 0 -or $ProcessId -eq $PID -or $Owned.ContainsKey($ProcessId)) { return }
  try {
    $Process = Get-Process -Id $ProcessId -ErrorAction Stop
    $Owned[$ProcessId] = $Process.StartTime.ToUniversalTime().Ticks
  }
  catch { }
}

function Update-OwnedProcessTree {
  param(
    [System.Collections.Generic.Dictionary[int, long]]$Owned
  )
  $Rows = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Select-Object ProcessId, ParentProcessId)
  $Changed = $true
  while ($Changed) {
    $Changed = $false
    foreach ($Row in $Rows) {
      $ParentId = [int]$Row.ParentProcessId
      $ChildId = [int]$Row.ProcessId
      if ($Owned.ContainsKey($ParentId) -and -not $Owned.ContainsKey($ChildId)) {
        Add-OwnedProcessIdentity -Owned $Owned -ProcessId $ChildId
        if ($Owned.ContainsKey($ChildId)) { $Changed = $true }
      }
    }
  }
}

function Test-OwnedProcessIdentity {
  param(
    [System.Collections.Generic.Dictionary[int, long]]$Owned,
    [int]$ProcessId
  )
  if (-not $Owned.ContainsKey($ProcessId)) { return $false }
  try {
    $Process = Get-Process -Id $ProcessId -ErrorAction Stop
    return $Process.StartTime.ToUniversalTime().Ticks -eq $Owned[$ProcessId]
  }
  catch { return $false }
}

function Get-OwnedWorkingSetBytes {
  param(
    [System.Collections.Generic.Dictionary[int, long]]$Owned
  )
  Update-OwnedProcessTree -Owned $Owned
  [int64]$Total = 0
  foreach ($ProcessId in @($Owned.Keys)) {
    if (-not (Test-OwnedProcessIdentity -Owned $Owned -ProcessId $ProcessId)) { continue }
    try { $Total += (Get-Process -Id $ProcessId -ErrorAction Stop).WorkingSet64 } catch { }
  }
  return $Total
}

function Stop-OwnedProcessTree {
  param(
    [System.Collections.Generic.Dictionary[int, long]]$Owned,
    [int]$RootProcessId
  )
  Update-OwnedProcessTree -Owned $Owned
  if (Test-OwnedProcessIdentity -Owned $Owned -ProcessId $RootProcessId) {
    Stop-Process -Id $RootProcessId -ErrorAction SilentlyContinue
  }
  Start-Sleep -Milliseconds 500
  Update-OwnedProcessTree -Owned $Owned
  foreach ($ProcessId in @($Owned.Keys | Sort-Object -Descending)) {
    if (Test-OwnedProcessIdentity -Owned $Owned -ProcessId $ProcessId) {
      Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    }
  }
}

function Get-ElapsedMilliseconds {
  param([long]$StartedTimestamp)
  return [Math]::Round(([System.Diagnostics.Stopwatch]::GetTimestamp() - $StartedTimestamp) * 1000.0 / [System.Diagnostics.Stopwatch]::Frequency, 2)
}

function Get-Percentile {
  param(
    [double[]]$Values,
    [ValidateRange(0, 1)][double]$Percentile
  )
  $Sorted = @($Values | Sort-Object)
  if ($Sorted.Count -eq 0) { return $null }
  $Index = [Math]::Max(0, [Math]::Ceiling($Percentile * $Sorted.Count) - 1)
  return [Math]::Round([double]$Sorted[$Index], 2)
}

function Get-Median {
  param([double[]]$Values)
  return Get-Percentile -Values $Values -Percentile 0.5
}

function Escape-MarkdownCell {
  param([object]$Value)
  if ($null -eq $Value) { return "n/a" }
  return ([string]$Value).Replace("|", "\|").Replace("`r", " ").Replace("`n", " ")
}

$Runs = @()
for ($Iteration = 1; $Iteration -le $Repeat; $Iteration++) {
  Write-Host "[$Iteration/$Repeat] Starting $($Exe.Name)..."
  $Owned = [System.Collections.Generic.Dictionary[int, long]]::new()
  $Process = $null
  $RootPid = 0
  $Run = [ordered]@{
    iteration = $Iteration
    passed = $false
    processId = $null
    coldStartMs = $null
    firstContentMs = $null
    idleRssMb = $null
    idleRssMaxMb = $null
    dpi = $null
    scalePercent = $null
    windowWidth = $null
    windowHeight = $null
    contentProbe = $null
    error = $null
  }

  try {
    $StartInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $StartInfo.FileName = $ResolvedExe
    $StartInfo.WorkingDirectory = $Exe.DirectoryName
    $StartInfo.UseShellExecute = $false
    foreach ($Argument in $Arguments) { $null = $StartInfo.ArgumentList.Add($Argument) }

    $Process = [System.Diagnostics.Process]::new()
    $Process.StartInfo = $StartInfo
    $StartedTimestamp = [System.Diagnostics.Stopwatch]::GetTimestamp()
    if (-not $Process.Start()) { throw "Process.Start returned false" }
    $RootPid = $Process.Id
    $Run.processId = $RootPid
    Add-OwnedProcessIdentity -Owned $Owned -ProcessId $RootPid

    $Deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $WindowHandle = [IntPtr]::Zero
    while ([DateTime]::UtcNow -lt $Deadline) {
      Update-OwnedProcessTree -Owned $Owned
      if ($Process.HasExited) { throw "Process exited before a visible main window appeared (exit code $($Process.ExitCode))" }
      $WindowHandle = [MoePlayWindowProbe]::FindLargestVisibleWindow($RootPid)
      if ($WindowHandle -ne [IntPtr]::Zero) {
        if ($null -eq $Run.coldStartMs) { $Run.coldStartMs = Get-ElapsedMilliseconds -StartedTimestamp $StartedTimestamp }
        $Sample = [MoePlayWindowProbe]::Capture($WindowHandle)
        $Run.contentProbe = [ordered]@{
          captured = $Sample.Captured
          uniqueColors = $Sample.UniqueColors
          luminanceRange = $Sample.LuminanceRange
          changedPixelRatio = $Sample.ChangedPixelRatio
          error = $Sample.Error
        }
        if ($Sample.Dpi -gt 0) {
          $Run.dpi = $Sample.Dpi
          $Run.scalePercent = [Math]::Round($Sample.Dpi / 96.0 * 100, 0)
        }
        $Run.windowWidth = $Sample.Width
        $Run.windowHeight = $Sample.Height
        if ($Sample.Captured -and $Sample.UniqueColors -ge 6 -and $Sample.LuminanceRange -ge 20 -and $Sample.ChangedPixelRatio -ge 0.02) {
          $Run.firstContentMs = Get-ElapsedMilliseconds -StartedTimestamp $StartedTimestamp
          break
        }
      }
      Start-Sleep -Milliseconds $PollMilliseconds
    }

    if ($null -eq $Run.coldStartMs) { throw "Timed out after ${TimeoutSeconds}s waiting for a visible Tauri window" }
    if ($null -eq $Run.firstContentMs) {
      $ProbeError = if ($Run.contentProbe -and $Run.contentProbe.error) { $Run.contentProbe.error } else { "content thresholds were not reached" }
      throw "Timed out after ${TimeoutSeconds}s waiting for first-content ($ProbeError)"
    }

    $IdleSamples = [System.Collections.Generic.List[double]]::new()
    $IdleStarted = [DateTime]::UtcNow
    $IdleDeadline = $IdleStarted.AddSeconds($IdleSeconds)
    $IdleSampleDelay = [Math]::Max(250, $PollMilliseconds)
    while ([DateTime]::UtcNow -lt $IdleDeadline) {
      if ($Process.HasExited) { throw "Process exited during idle RSS sampling (exit code $($Process.ExitCode))" }
      $ElapsedIdle = ([DateTime]::UtcNow - $IdleStarted).TotalSeconds
      $WorkingSetBytes = Get-OwnedWorkingSetBytes -Owned $Owned
      if ($ElapsedIdle -ge ($IdleSeconds / 2.0)) {
        $IdleSamples.Add([Math]::Round($WorkingSetBytes / 1MB, 2))
      }
      Start-Sleep -Milliseconds $IdleSampleDelay
    }
    if ($IdleSamples.Count -eq 0) { throw "No idle RSS samples were collected" }
    $Run.idleRssMb = Get-Median -Values $IdleSamples.ToArray()
    $Run.idleRssMaxMb = [Math]::Round(($IdleSamples | Measure-Object -Maximum).Maximum, 2)
    $Run.passed = $Run.coldStartMs -le $MaxColdStartMs -and $Run.firstContentMs -le $MaxFirstContentMs -and $Run.idleRssMb -le $MaxIdleRssMb
  }
  catch {
    $Run.error = $_.Exception.Message
  }
  finally {
    if ($RootPid -gt 0) { Stop-OwnedProcessTree -Owned $Owned -RootProcessId $RootPid }
    if ($Process) { $Process.Dispose() }
  }

  $Runs += [pscustomobject]$Run
  $Status = if ($Run.passed) { "PASS" } else { "FAIL" }
  Write-Host "[$Iteration/$Repeat] $Status cold=$($Run.coldStartMs)ms content=$($Run.firstContentMs)ms idleRSS=$($Run.idleRssMb)MiB scale=$($Run.scalePercent)% $($Run.error)"
  Start-Sleep -Milliseconds 750
}

$SuccessfulRuns = @($Runs | Where-Object { $null -ne $_.firstContentMs -and $null -ne $_.idleRssMb })
$ColdValues = [double[]]@($SuccessfulRuns | ForEach-Object coldStartMs)
$ContentValues = [double[]]@($SuccessfulRuns | ForEach-Object firstContentMs)
$RssValues = [double[]]@($SuccessfulRuns | ForEach-Object idleRssMb)
$Scales = @($SuccessfulRuns | Where-Object { $null -ne $_.scalePercent } | Select-Object -ExpandProperty scalePercent -Unique | Sort-Object)

$Summary = [ordered]@{
  successfulRuns = $SuccessfulRuns.Count
  failedRuns = $Repeat - $SuccessfulRuns.Count
  coldStartMs = [ordered]@{ p50 = (Get-Percentile -Values $ColdValues -Percentile 0.5); p95 = (Get-Percentile -Values $ColdValues -Percentile 0.95) }
  firstContentMs = [ordered]@{ p50 = (Get-Percentile -Values $ContentValues -Percentile 0.5); p95 = (Get-Percentile -Values $ContentValues -Percentile 0.95) }
  idleRssMb = [ordered]@{ p50 = (Get-Percentile -Values $RssValues -Percentile 0.5); p95 = (Get-Percentile -Values $RssValues -Percentile 0.95) }
  observedScalePercents = $Scales
}

$Checks = @(
  [pscustomobject]@{ name = "All repeats produced first-content and RSS samples"; actual = $SuccessfulRuns.Count; limit = $Repeat; unit = "runs"; passed = $SuccessfulRuns.Count -eq $Repeat },
  [pscustomobject]@{ name = "Cold start P95"; actual = $Summary.coldStartMs.p95; limit = $MaxColdStartMs; unit = "ms"; passed = $null -ne $Summary.coldStartMs.p95 -and $Summary.coldStartMs.p95 -le $MaxColdStartMs },
  [pscustomobject]@{ name = "First-content P95"; actual = $Summary.firstContentMs.p95; limit = $MaxFirstContentMs; unit = "ms"; passed = $null -ne $Summary.firstContentMs.p95 -and $Summary.firstContentMs.p95 -le $MaxFirstContentMs },
  [pscustomobject]@{ name = "Idle RSS P95"; actual = $Summary.idleRssMb.p95; limit = $MaxIdleRssMb; unit = "MiB"; passed = $null -ne $Summary.idleRssMb.p95 -and $Summary.idleRssMb.p95 -le $MaxIdleRssMb }
)
$Passed = @($Checks | Where-Object { -not $_.passed }).Count -eq 0

$Report = [ordered]@{
  schemaVersion = 1
  capturedAt = (Get-Date).ToUniversalTime().ToString("o")
  passed = $Passed
  executable = [ordered]@{
    path = $ResolvedExe
    bytes = $Exe.Length
    lastWriteTime = $Exe.LastWriteTimeUtc.ToString("o")
    fileVersion = $Exe.VersionInfo.FileVersion
    productVersion = $Exe.VersionInfo.ProductVersion
  }
  environment = [ordered]@{
    computerName = $env:COMPUTERNAME
    os = [System.Environment]::OSVersion.VersionString
    powershell = $PSVersionTable.PSVersion.ToString()
    processorCount = [System.Environment]::ProcessorCount
    observedScalePercents = $Scales
  }
  definitions = [ordered]@{
    coldStart = "Fresh script-created process to first visible top-level window. OS filesystem/WebView caches are not flushed."
    firstContent = "Fresh process to first PrintWindow sample with at least 6 sampled colors, luminance range >= 20, and changed-pixel ratio >= 0.02."
    idleRss = "Median working set of the script-created Tauri/WebView process tree during the final half of the idle window."
  }
  options = [ordered]@{
    repeat = $Repeat
    timeoutSeconds = $TimeoutSeconds
    idleSeconds = $IdleSeconds
    pollMilliseconds = $PollMilliseconds
    maxColdStartMs = $MaxColdStartMs
    maxFirstContentMs = $MaxFirstContentMs
    maxIdleRssMb = $MaxIdleRssMb
  }
  summary = $Summary
  checks = $Checks
  runs = $Runs
  scaleEvidenceCaveat = "The script records current monitor DPI but does not change Windows scale. Separate real interactive runs and visual/manual sign-off at 125% and 150% are still required."
}

$JsonDirectory = Split-Path -Parent $JsonPath
$MarkdownDirectory = Split-Path -Parent $MarkdownPath
New-Item -ItemType Directory -Force -Path $JsonDirectory | Out-Null
New-Item -ItemType Directory -Force -Path $MarkdownDirectory | Out-Null
$Report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $JsonPath -Encoding UTF8

$Markdown = [System.Text.StringBuilder]::new()
$null = $Markdown.AppendLine("# Real Tauri startup evidence")
$null = $Markdown.AppendLine()
$null = $Markdown.AppendLine("- Captured: $($Report.capturedAt)")
$null = $Markdown.AppendLine("- Executable: ``$ResolvedExe`` ($($Exe.VersionInfo.ProductVersion))")
$null = $Markdown.AppendLine("- Repeats: $Repeat")
$null = $Markdown.AppendLine("- Result: **$(if ($Passed) { 'PASS' } else { 'FAIL' })**")
$null = $Markdown.AppendLine("- Observed Windows scale: $(if ($Scales.Count) { (($Scales | ForEach-Object { "$_%" }) -join ', ') } else { 'unavailable' })")
$null = $Markdown.AppendLine()
$null = $Markdown.AppendLine("## Thresholds")
$null = $Markdown.AppendLine()
$null = $Markdown.AppendLine("| Check | Actual | Limit | Result |")
$null = $Markdown.AppendLine("|---|---:|---:|---|")
foreach ($Check in $Checks) {
  $Actual = Escape-MarkdownCell $Check.actual
  $null = $Markdown.AppendLine("| $($Check.name) | $Actual $($Check.unit) | $($Check.limit) $($Check.unit) | $(if ($Check.passed) { 'PASS' } else { 'FAIL' }) |")
}
$null = $Markdown.AppendLine()
$null = $Markdown.AppendLine("## Runs")
$null = $Markdown.AppendLine()
$null = $Markdown.AppendLine("| Run | Cold window ms | First content ms | Idle RSS MiB | Scale | Result | Error |")
$null = $Markdown.AppendLine("|---:|---:|---:|---:|---:|---|---|")
foreach ($Run in $Runs) {
  $null = $Markdown.AppendLine("| $($Run.iteration) | $(Escape-MarkdownCell $Run.coldStartMs) | $(Escape-MarkdownCell $Run.firstContentMs) | $(Escape-MarkdownCell $Run.idleRssMb) | $(Escape-MarkdownCell $Run.scalePercent)% | $(if ($Run.passed) { 'PASS' } else { 'FAIL' }) | $(Escape-MarkdownCell $Run.error) |")
}
$null = $Markdown.AppendLine()
$null = $Markdown.AppendLine("## Measurement definitions and caveats")
$null = $Markdown.AppendLine()
$null = $Markdown.AppendLine("- Cold start is process-cold: a new script-owned process is created for every repeat. The script does **not** flush Windows filesystem, GPU, or WebView2 caches.")
$null = $Markdown.AppendLine("- First-content is detected from the real top-level window via ``PrintWindow`` pixel diversity; a timeout is reported as missing evidence rather than substituted with process/window timing.")
$null = $Markdown.AppendLine("- Idle RSS is the Tauri root plus only descendants discovered from that script-created PID. Cleanup never kills by executable name.")
$null = $Markdown.AppendLine("- **125% and 150% Windows scale still require separate real interactive runs and visual/manual sign-off.** This script records the current DPI only; it does not emulate or claim those environments.")
$Markdown.ToString() | Set-Content -LiteralPath $MarkdownPath -Encoding UTF8

Write-Output "JSON: $JsonPath"
Write-Output "Markdown: $MarkdownPath"

if (-not $Passed -and -not $NoFail) { exit 1 }



