
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  public const uint LEFTDOWN = 0x0002, LEFTUP = 0x0004;
  public static void Drag(int x1, int y1, int x2, int y2) {
    SetCursorPos(x1, y1);
    System.Threading.Thread.Sleep(150);
    mouse_event(LEFTDOWN, 0, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(150);
    for (int i = 1; i <= 12; i++) {
      SetCursorPos(x1 + (x2 - x1) * i / 12, y1 + (y2 - y1) * i / 12);
      System.Threading.Thread.Sleep(35);
    }
    System.Threading.Thread.Sleep(150);
    mouse_event(LEFTUP, 0, 0, 0, IntPtr.Zero);
  }
}
"@
[Win]::SetProcessDPIAware() | Out-Null

$dir = "$env:TEMP\miyin-drag"
Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $dir -Force | Out-Null
Copy-Item 'D:\Code\Rust\HSCL\dist\miyin-launcher\miyin-launcher.exe' "$dir\miyin-launcher.exe"
$p = Start-Process -FilePath "$dir\miyin-launcher.exe" -WorkingDirectory $dir -PassThru
Start-Sleep -Seconds 7

$h = $p.MainWindowHandle
if ($h -eq [IntPtr]::Zero) { "NO WINDOW HANDLE"; Stop-Process -Id $p.Id -Force; exit 1 }

$before = New-Object Win+RECT
[Win]::GetWindowRect($h, [ref]$before) | Out-Null
"before: L=$($before.Left) T=$($before.Top) W=$($before.Right-$before.Left) H=$($before.Bottom-$before.Top)"

[Win]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 600

# Try a few grab points along the top bar, from right to left
$points = @(
  @{ dx = 300; dy = 28; label = "right-mid" },
  @{ dx = 520; dy = 28; label = "center" },
  @{ dx = 160; dy = 28; label = "left (brand)" }
)

foreach ($pt in $points) {
  $sx = $before.Right - $pt.dx
  $sy = $before.Top + $pt.dy
  "grab at $($pt.label): ($sx, $sy)"

  [Win]::Drag($sx, $sy, $sx - 150, $sy + 100)
  Start-Sleep -Milliseconds 800

  $now = New-Object Win+RECT
  [Win]::GetWindowRect($h, [ref]$now) | Out-Null
  $dx = $now.Left - $before.Left
  $dy = $now.Top - $before.Top
  "  moved dx=$dx dy=$dy"

  if ([Math]::Abs($dx) -gt 20 -or [Math]::Abs($dy) -gt 20) {
    "RESULT: OK -- top bar can drag the window ($($pt.label))"
    Stop-Process -Id $p.Id -Force
    Start-Sleep -Seconds 1
    Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
    exit 0
  }
}

"RESULT: FAIL -- window did not move at any grab point"
Stop-Process -Id $p.Id -Force
Start-Sleep -Seconds 1
Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
