# 输入注入与窗口工具。用法: . scripts\win.ps1 然后调 Click-At / Drag-Window / Get-PomoRect / Save-WindowShot
Add-Type -AssemblyName System.Drawing
Add-Type -Name U32 -Namespace PomoWin -MemberDefinition '
 [DllImport("user32.dll")] public static extern bool GetWindowRect(System.IntPtr h, out RECT r);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr h);
 [DllImport("user32.dll")] public static extern uint SendInput(uint n, INPUT[] i, int sz);
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
 [DllImport("user32.dll")] public static extern uint GetDpiForSystem();
 public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
 [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public MOUSEINPUT mi; }
 [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public System.IntPtr dwExtraInfo; }
'

function Get-PomoHandle { (Get-Process pomodoro-clock | Select-Object -First 1).MainWindowHandle }

function Get-PomoRect {
  $h = Get-PomoHandle
  $r = New-Object PomoWin.U32+RECT
  [PomoWin.U32]::GetWindowRect($h, [ref]$r) | Out-Null
  return $r
}

function Send-Mouse($x, $y, [uint32]$flags) {
  # 进程 DPI-unaware：SendInput 绝对坐标在物理像素空间，需乘缩放
  $scale = [PomoWin.U32]::GetDpiForSystem() / 96.0
  $i = New-Object PomoWin.U32+INPUT
  $i.type = 0
  $i.mi.dx = [int]($x * $scale * 65535 / 1919)
  $i.mi.dy = [int]($y * $scale * 65535 / 1079)
  $i.mi.dwFlags = $flags
  $sz = [System.Runtime.InteropServices.Marshal]::SizeOf([type][PomoWin.U32+INPUT])
  [PomoWin.U32]::SendInput(1, @($i), $sz) | Out-Null
}

function Click-At($x, $y) {
  Send-Mouse $x $y 0x8001   # 先移动（hover）
  Start-Sleep -Milliseconds 150
  Send-Mouse $x $y 0x0002   # LEFTDOWN（原地）
  Start-Sleep -Milliseconds 90
  Send-Mouse $x $y 0x0004   # LEFTUP
}

Add-Type -Name GW -Namespace PomoWin2 -MemberDefinition '
 [DllImport("user32.dll")] public static extern System.IntPtr GetWindow(System.IntPtr h, uint cmd);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(System.IntPtr h, System.Text.StringBuilder s, int n);
 [DllImport("user32.dll")] public static extern System.IntPtr SendMessage(System.IntPtr h, uint m, System.IntPtr w, System.IntPtr l);
 [DllImport("user32.dll")] public static extern bool PostMessage(System.IntPtr h, uint m, System.IntPtr w, System.IntPtr l);
'

# 找到 WebView2 最深渲染子窗口（Chrome_WidgetWin_1）
function Get-WebViewChild {
  $h = Get-PomoHandle
  $wry = [PomoWin2.GW]::GetWindow($h, 5)          # WRY_WEBVIEW
  $w0  = [PomoWin2.GW]::GetWindow($wry, 5)        # Chrome_WidgetWin_0
  $w1  = [PomoWin2.GW]::GetWindow($w0, 5)         # Chrome_WidgetWin_1
  return $w1
}

# 完整拖拽模拟：JS 链(SendMessage 到 WebView) + OS 模态移动循环(PostMessage 到主窗口)
function Drag-PomoWindow([int]$fromX, [int]$fromY, [int]$dx, [int]$dy) {
  $w1 = Get-WebViewChild
  $main = Get-PomoHandle
  $mk = { param($x, $y) [System.IntPtr](($y -shl 16) -bor ($x -band 0xFFFF)) }
  # 1) JS 链：按下 + 移动超过阈值 → startDragging() → tao 进入模态移动循环
  [PomoWin2.GW]::SendMessage($w1, 0x0201, [System.IntPtr]::new(1), (& $mk $fromX $fromY)) | Out-Null
  Start-Sleep -Milliseconds 80
  [PomoWin2.GW]::SendMessage($w1, 0x0200, [System.IntPtr]::new(1), (& $mk ($fromX + 10) ($fromY + 8))) | Out-Null
  Start-Sleep -Milliseconds 80
  [PomoWin2.GW]::SendMessage($w1, 0x0200, [System.IntPtr]::new(1), (& $mk ($fromX + 20) ($fromY + 16))) | Out-Null
  Start-Sleep -Milliseconds 300
  # 2) OS 模态循环：投递鼠标移动 → 窗口跟随；最后抬起结束
  $steps = 12
  for ($i = 1; $i -le $steps; $i++) {
    $mx = $fromX + 20 + [int]($dx * $i / $steps)
    $my = $fromY + 16 + [int]($dy * $i / $steps)
    [PomoWin2.GW]::PostMessage($main, 0x0200, [System.IntPtr]::new(1), (& $mk $mx $my)) | Out-Null
    Start-Sleep -Milliseconds 30
  }
  [PomoWin2.GW]::PostMessage($main, 0x0202, [System.IntPtr]::Zero, (& $mk ($fromX + 20 + $dx) ($fromY + 16 + $dy))) | Out-Null
  Start-Sleep -Milliseconds 400
}
function WebView-Click([int]$x, [int]$y) {
  $w1 = Get-WebViewChild
  $lp = [System.IntPtr](($y -shl 16) -bor ($x -band 0xFFFF))
  [PomoWin2.GW]::SendMessage($w1, 0x0201, [System.IntPtr]::new(1), $lp) | Out-Null  # WM_LBUTTONDOWN
  Start-Sleep -Milliseconds 60
  [PomoWin2.GW]::SendMessage($w1, 0x0202, [System.IntPtr]::Zero, $lp) | Out-Null     # WM_LBUTTONUP
}

function Save-WindowShot($path, [int]$pad = 40) {
  $r = Get-PomoRect
  $w = $r.Right - $r.Left + 2 * $pad; $h = $r.Bottom - $r.Top + 2 * $pad
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.Left - $pad, $r.Top - $pad, 0, 0, $bmp.Size)
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  Write-Output "saved: $path ($($r.Left),$($r.Top) $($r.Right-$r.Left)x$($r.Bottom-$r.Top))"
}
