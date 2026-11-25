$ErrorActionPreference = 'Continue'
$out = "C:\dev\CRU\cru-modern\target\ui_click_result.txt"
$log = Join-Path $env:TEMP "cru-modern-debug.log"
$last = Join-Path $env:TEMP "cru-modern-last-restart.txt"
$lines = @()
function Say($t) { $script:lines += $t; $t | Out-File -FilePath $out -Append -Encoding utf8 }

if (Test-Path $out) { Remove-Item $out -Force }
if (Test-Path $log) { Remove-Item $log -Force }
if (Test-Path $last) { Remove-Item $last -Force }

$exe = "C:\dev\CRU\cru-modern\target12\debug\cru-modern.exe"
Get-Process cru-modern -ErrorAction SilentlyContinue | ForEach-Object { taskkill /PID $_.Id /F 2>&1 | Out-Null }
Start-Sleep -Seconds 3
Start-Process -FilePath $exe | Out-Null
Start-Sleep -Seconds 8

$p = Get-Process cru-modern -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { Say "APP NOT RUNNING"; exit 1 }
Say "pid=$($p.Id)"

$sig = @'
using System;
using System.Runtime.InteropServices;
public class W7 {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr extra);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
'@
Add-Type -TypeDefinition $sig

$hwnd = $p.MainWindowHandle
[W7]::ShowWindow($hwnd, 9) | Out-Null
[W7]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
[W7]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
[W7]::BringWindowToTop($hwnd) | Out-Null
[W7]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 900
Say "foreground matches app: $([W7]::GetForegroundWindow() -eq $hwnd)"

$r = New-Object W7+RECT
[W7]::GetWindowRect($hwnd, [ref]$r) | Out-Null
$x = $r.Right - 231
$y = $r.Top + 58
Say "click $x,$y (window $($r.Left),$($r.Top))"
[W7]::SetCursorPos($x, $y) | Out-Null
Start-Sleep -Milliseconds 300
[W7]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 120
[W7]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$exitAt = $null
$relaunchAt = $null
$events = @()
while ($sw.Elapsed.TotalSeconds -lt 45) {
    Start-Sleep -Milliseconds 300
    $procs = @(Get-Process cru-modern -ErrorAction SilentlyContinue)
    $t = [math]::Round($sw.Elapsed.TotalSeconds, 1)
    $old = $procs | Where-Object { $_.Id -eq $p.Id }
    if (-not $old -and -not $exitAt) { $exitAt = $t; $events += "${t}s GUI exited" }
    $newOnes = $procs | Where-Object { $_.Id -ne $p.Id -and $_.MainWindowHandle -ne 0 }
    if ($newOnes -and -not $relaunchAt) { $relaunchAt = $t; $events += "${t}s GUI relaunched pid=$($newOnes[0].Id)" }
    if ($exitAt -and $relaunchAt -and $t -gt ($relaunchAt + 6)) { break }
}
Say ($events -join " | ")
$now = @(Get-Process cru-modern -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 })
if ($now) { Say "final: pid=$($now[0].Id) responding=$($now[0].Responding) title='$($now[0].MainWindowTitle)'" } else { Say "final: NO GUI" }
Say "last-restart file left behind: $(Test-Path $last)"
Say "--- debug log ---"
if (Test-Path $log) { Get-Content $log | ForEach-Object { Say $_ } } else { Say "(no log file)" }
