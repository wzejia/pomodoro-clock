# v10 任务5 多屏/DPI 模拟实测：窗口移出主屏边缘后 morph 钳制回工作区 + 拖拽跟随不冻结
# 用法：pwsh scripts\v10-multimon.ps1
# 判据：半出右缘/左缘负坐标/完全出屏 三态展开后 rect 全在工作区 (0,0,1920,1032) 内且 CDP 应答；
#       边缘拖拽 rect 跟随真实光标、up 后停跟。证据 JSON 落 %TEMP%\pomo-shots\v10-multimon.json
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1

$WA = @{ Left = 0; Top = 0; Right = 1920; Bottom = 1032 }   # 主屏工作区（1080 - 任务栏48）
$ev = [ordered]@{}
$pass = $true

function Rect-Obj { $r = Get-PomoRect; [ordered]@{ L = $r.Left; T = $r.Top; R = $r.Right; B = $r.Bottom } }
function In-WorkArea($r) { ($r.L -ge $WA.Left) -and ($r.T -ge $WA.Top) -and ($r.R -le $WA.Right) -and ($r.B -le $WA.Bottom) }
function Cdp-Alive { try { (Seq "state") | Out-Null; return $true } catch { return $false } }

# DPR（CSS→物理换算，拖拽光标定位用）
$dpr = [double]((Seq "eval window.devicePixelRatio") | ConvertFrom-Json)
$ev.dpr = $dpr

# ---------- A：半出右缘 → 展开 morph 钳回 ----------
Move-Pomo 1860 400
$r0 = Rect-Obj; $ev.A_before = $r0
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 1400
$r1 = Rect-Obj; $ev.A_after = $r1; $ev.A_clamped = (In-WorkArea $r1); $ev.A_alive = (Cdp-Alive)
if (-not ($ev.A_clamped -and $ev.A_alive)) { $pass = $false }
Esc-Collapse; Start-Sleep -Milliseconds 1000

# ---------- B1：左缘负坐标（模拟左屏坐标符号）→ 展开钳回 ----------
Move-Pomo -120 300
$ev.B1_before = Rect-Obj
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 1400
$r = Rect-Obj; $ev.B1_after = $r; $ev.B1_clamped = (In-WorkArea $r); $ev.B1_alive = (Cdp-Alive)
if (-not ($ev.B1_clamped -and $ev.B1_alive)) { $pass = $false }
Esc-Collapse; Start-Sleep -Milliseconds 1000

# ---------- B2：完全出屏（虚拟屏无显示器处）→ DEFAULTTONEAREST 钳回最近显示器 ----------
Move-Pomo 3000 400
$ev.B2_before = Rect-Obj
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 1400
$r = Rect-Obj; $ev.B2_after = $r; $ev.B2_clamped = (In-WorkArea $r); $ev.B2_alive = (Cdp-Alive)
if (-not ($ev.B2_clamped -and $ev.B2_alive)) { $pass = $false }
Esc-Collapse; Start-Sleep -Milliseconds 1000

# ---------- C：右缘处拖拽跟随 + 停跟 ----------
# 前端阈值拖拽：合成 move 位移须 >12px 才 drag_begin（v10 诊断实证）；
# 窗口贴右缘时 SetCursorPos 目标超出 1920 会被 OS 钳到 1919——跟随量按钳后光标断言
Move-Pomo 1700 500
$r0 = Rect-Obj
# 真实光标落到「触发 move 事件」的抓取点（140,60 CSS，dpr=1 同物理），零跳动起手
[PomoWin.U32]::SetCursorPos($r0.L + [int](140 * $dpr), $r0.T + [int](60 * $dpr)) | Out-Null
Start-Sleep -Milliseconds 250
Seq "downmove 100 38 140 60" | Out-Null
Start-Sleep -Milliseconds 200
$expectCursorX = [math]::Min($r0.L + [int](140 * $dpr) + 150, 1919)   # OS 边缘钳制
[PomoWin.U32]::SetCursorPos($r0.L + [int](140 * $dpr) + 150, $r0.T + [int](60 * $dpr)) | Out-Null
Start-Sleep -Milliseconds 350
$r1 = Rect-Obj
Seq "up 140 60" | Out-Null
Start-Sleep -Milliseconds 250
$r2 = Rect-Obj
# up 后再挪光标，窗口不得再跟
[PomoWin.U32]::SetCursorPos($expectCursorX + 200 - 2000, $r0.T + 300) | Out-Null
Start-Sleep -Milliseconds 400
$r3 = Rect-Obj
$expectL = $expectCursorX - [int](140 * $dpr)
$ev.C = [ordered]@{ before = $r0; dragged = $r1; afterUp = $r2; cursorAway = $r3;
  followed = ($r1.L - $r0.L); expectedL = $expectL; stoppedAfterUp = ($r3.L -eq $r2.L); alive = (Cdp-Alive) }
# 跟到光标（钳后）位置 ±3px；up 后完全停跟；CDP 应答
if (-not ([math]::Abs($r1.L - $expectL) -le 3 -and $ev.C.stoppedAfterUp -and $ev.C.alive)) { $pass = $false }

# 复原：回常规位 + 确保收起
Move-Pomo 1500 300
Esc-Collapse; Start-Sleep -Milliseconds 600

$ev.PASS = $pass
$ev | ConvertTo-Json -Depth 4 | Out-File -Encoding utf8 "$script:ShotTmp\v10-multimon.json"
$ev | ConvertTo-Json -Depth 4
Write-Output ($(if ($pass) { "MULTIMON PASS" } else { "MULTIMON FAIL" }))
