# v7 验收环境：统一 P/Invoke + 辅助函数（每个脚本顶部 . 引入）
$ErrorActionPreference = "Stop"
cd D:\VibeCoding\pomodoro-clock
. scripts\win.ps1
Add-Type -Name V7 -Namespace V7Win -MemberDefinition '
 [DllImport("user32.dll")] public static extern bool SetWindowPos(System.IntPtr h, System.IntPtr after, int x, int y, int cx, int cy, uint flags);
 [DllImport("user32.dll")] public static extern System.IntPtr WindowFromPoint(int x, int y);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr h, out uint pid);
 [DllImport("user32.dll")] public static extern System.IntPtr GetForegroundWindow();
'

$script:Seq = "D:\VibeCoding\pomodoro-clock\scripts\cdp-seq.mjs"
# 中间帧/JSON 产物统一放 TEMP（原 job tmp 会随会话清理，2026-08-14 收编进 scripts/）
$script:ShotTmp = "$env:TEMP\pomo-shots"
New-Item -ItemType Directory -Force $script:ShotTmp | Out-Null
$script:PomoPid = (Get-Process pomodoro-clock | Select-Object -First 1).Id
$script:PomoHwnd = (Get-Process -Id $script:PomoPid).MainWindowHandle

function Seq([string]$cmdline) { & bun $script:Seq ($cmdline -split ' ') 2>$null }

function App-State { (Seq "state") | ConvertFrom-Json }
function Esc-Collapse { Seq "esc" | Out-Null }

function Move-Pomo([int]$x, [int]$y) {
  [V7Win.V7]::SetWindowPos($script:PomoHwnd, [System.IntPtr]::Zero, $x, $y, 0, 0, 0x0001) | Out-Null  # SWP_NOSIZE
  Start-Sleep -Milliseconds 300
}

function Win-Size { $r = Get-PomoRect; return "$(($r.Right-$r.Left))x$(($r.Bottom-$r.Top))@($($r.Left),$($r.Top))" }

# pill 文本点（客户端坐标，避开右侧按钮）
$script:PillCX = 100; $script:PillCY = 38
function Pill-Screen { $r = Get-PomoRect; return @(($r.Left + $script:PillCX), ($r.Top + $script:PillCY)) }

# 把真实光标挪到 pill 抓取点（供 Rust 拖拽线程跟随），不点击、不抢焦
function Cursor-To-Pill([int]$dx = 16) {
  $p = Pill-Screen
  [PomoWin.U32]::SetCursorPos($p[0] + $dx, $p[1]) | Out-Null
}
