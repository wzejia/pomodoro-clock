# v6.1 终验 · v7 交互回归四项（失焦即收/Esc 整收/宽容点击/快速拖拽不冻）
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -Name FG61 -Namespace V61Fg -MemberDefinition '
 [DllImport("user32.dll")] public static extern System.IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr h, out uint pid);
 [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint from, uint to, bool attach);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr h);
 [DllImport("user32.dll")] public static extern bool BringWindowToTop(System.IntPtr h);
 [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
 public static bool ForceForeground(System.IntPtr h) {
   var fg = GetForegroundWindow();
   if (fg == h) return true;
   uint dummy;
   uint fgTid = GetWindowThreadProcessId(fg, out dummy);
   uint myTid = GetCurrentThreadId();
   AttachThreadInput(myTid, fgTid, true);
   BringWindowToTop(h);
   bool ok = SetForegroundWindow(h);
   AttachThreadInput(myTid, fgTid, false);
   return ok;
 }
'
function Focus-Ours-Retry {
  for ($t = 0; $t -lt 5; $t++) {
    [V61Fg.FG61]::ForceForeground($script:PomoHwnd) | Out-Null
    Start-Sleep -Milliseconds 350
    if ([V61Fg.FG61]::GetForegroundWindow() -eq $script:PomoHwnd) { return $true }
  }
  return $false
}
function Focus-Away {
  $wt = Get-Process WindowsTerminal | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
  [V61Fg.FG61]::ForceForeground($wt.MainWindowHandle) | Out-Null
  Start-Sleep -Milliseconds 400
}

Move-Pomo 1500 300
$wshell = New-Object -ComObject WScript.Shell

# 1) 宽容点击：8px 手抖 → 展开
Seq "jitter 100 38 8 3" | Out-Null; Start-Sleep -Milliseconds 900
$st = App-State
Write-Output "1) 宽容点击 8px 抖动展开: 态=$($st.cls) $(if($st.cls -match 'expanded'){'PASS'}else{'FAIL'})"

# 2) Esc 整收（抽屉开着也整收）
Seq "eval document.getElementById('app').classList.add('drawer-open');'d'" | Out-Null; Start-Sleep -Milliseconds 500
$st = App-State; $wasDrawer = $st.drawer
if (-not (Focus-Ours-Retry)) { Write-Output "2) 抢前台失败 SKIP" } else {
  $wshell.SendKeys("{ESC}")
  Start-Sleep -Milliseconds 900
  $st = App-State; $sz = Win-Size
  $ok = $wasDrawer -and ($st.cls -notmatch "expanded") -and (-not $st.drawer) -and ($sz -match "^220x76")
  Write-Output "2) 抽屉开态真实 Esc 整收: 抽屉曾开=$wasDrawer 态=$($st.cls) drawer=$($st.drawer) $sz $(if($ok){'PASS'}else{'FAIL'})"
}

# 3) 失焦即收：展开 → 抢焦给别的窗口 → 1s 内收起
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 800
if (-not (Focus-Ours-Retry)) { Write-Output "3) 抢前台失败 SKIP" } else {
  Focus-Away
  $t0 = Get-Date
  do { Start-Sleep -Milliseconds 100; $st = App-State } while (($st.cls -match "expanded") -and ((Get-Date) - $t0).TotalSeconds -lt 2)
  $ms = [int]((Get-Date) - $t0).TotalMilliseconds
  Start-Sleep -Milliseconds 600
  $sz = Win-Size
  $ok = ($st.cls -notmatch "expanded") -and ($sz -match "^220x76") -and ($ms -lt 1500)
  Write-Output "3) 失焦即收: 耗时~${ms}ms 态=$($st.cls) $sz $(if($ok){'PASS'}else{'FAIL'})"
}

# 4) 快速拖拽不冻：leave 后续拖仍跟随（v7 冻结 bug 回归）
Move-Pomo 1500 300
Cursor-To-Pill
# 终点必须在胶囊命中区内（y=38 中线）；透明窗圆角外像素会穿透 → mouseleave 杀拖拽（脚本教训）
Seq "downmove 100 38 124 38" | Out-Null   # 24px > 12 阈值 → 启动原生拖拽
Start-Sleep -Milliseconds 200
$r0 = Get-PomoRect
# 物理光标右移 120px（窗口应跟随）
$p = Pill-Screen
for ($i = 1; $i -le 6; $i++) { [PomoWin.U32]::SetCursorPos($p[0] + 16 + $i * 20, $p[1]) | Out-Null; Start-Sleep -Milliseconds 30 }
Start-Sleep -Milliseconds 200
$r1 = Get-PomoRect
Seq "leave" | Out-Null                  # 鼠标跑出窗口（旧 bug 在此冻结）
for ($i = 1; $i -le 5; $i++) { [PomoWin.U32]::SetCursorPos($p[0] + 136 + $i * 20, $p[1]) | Out-Null; Start-Sleep -Milliseconds 30 }
Start-Sleep -Milliseconds 200
$r2 = Get-PomoRect
Seq "up 124 38" | Out-Null
$d1 = $r1.Left - $r0.Left; $d2 = $r2.Left - $r1.Left
$ok = ($d1 -ge 100) -and ($d2 -ge 80)
Write-Output "4) 快速拖拽不冻: leave前跟 ${d1}px / leave后续跟 ${d2}px $(if($ok){'PASS'}else{'FAIL'})"
Move-Pomo 1500 300
Write-Output "回归完成"
