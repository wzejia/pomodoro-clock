# v9 失焦即收抽验（纯 CSS 改动碰不到此链路，抽验防意外——明卷 4）
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -Name FG9 -Namespace V9Fg -MemberDefinition '
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
    [V9Fg.FG9]::ForceForeground($script:PomoHwnd) | Out-Null
    Start-Sleep -Milliseconds 350
    if ([V9Fg.FG9]::GetForegroundWindow() -eq $script:PomoHwnd) { return $true }
  }
  return $false
}

Move-Pomo 1500 300
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 900
$st = App-State
if ($st.cls -notmatch "expanded") { Write-Output "展开失败 FAIL（态=$($st.cls)）"; exit 1 }
if (-not (Focus-Ours-Retry)) { Write-Output "抢前台失败 SKIP"; exit 2 }
$wt = Get-Process WindowsTerminal | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
[V9Fg.FG9]::ForceForeground($wt.MainWindowHandle) | Out-Null
$t0 = Get-Date
do { Start-Sleep -Milliseconds 100; $st = App-State } while (($st.cls -match "expanded") -and ((Get-Date) - $t0).TotalSeconds -lt 2)
$ms = [int]((Get-Date) - $t0).TotalMilliseconds
Start-Sleep -Milliseconds 600
$sz = Win-Size
$ok = ($st.cls -notmatch "expanded") -and ($sz -match "^220x76") -and ($ms -lt 1500)
Write-Output "失焦即收: 耗时~${ms}ms 态=$($st.cls) $sz $(if($ok){'PASS'}else{'FAIL'})"
