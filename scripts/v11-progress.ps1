# v11 任务4 胶囊进度条三态证据：专注中段 / 休息中段 / 暂停冻结（全图 + 底部 4x 放大裁切）
# 临时把时长改 1 分钟制造中段进度，验完恢复原 config/stats（先备份 .bak-v11）
# 用法：pwsh scripts\v11-progress.ps1
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -AssemblyName System.Drawing

$out = "D:\VibeCoding\pomodoro-clock\docs\screenshots\v11"
New-Item -ItemType Directory -Force $out | Out-Null
$exeDir = "D:\VibeCoding\pomodoro-clock\src-tauri\target\debug"
Copy-Item "$exeDir\config.json" "$exeDir\config.json.bak-v11" -Force
Copy-Item "$exeDir\stats.json" "$exeDir\stats.json.bak-v11" -Force

$tmpShot = "$script:ShotTmp\_shot11.png"
function Shot { Seq "shot $tmpShot" | Out-Null; Start-Sleep -Milliseconds 200 }

# CDP 原图带透明底：先合成底色，再裁底部条带 4x 最近邻放大（2px 条肉眼可评）
function Save-State([string]$bg, [string]$name) {
  Shot
  $src = [System.Drawing.Bitmap]::new($tmpShot)
  $pad = 60
  $bmp = [System.Drawing.Bitmap]::new(($src.Width + 2*$pad), ($src.Height + 2*$pad))
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.Clear([System.Drawing.ColorTranslator]::FromHtml($bg))
  $g.DrawImage($src, $pad, $pad, $src.Width, $src.Height)
  $bmp.Save("$out\$name.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  # 底部 30% 条带 4x 放大（含计时数字下缘作上下文）
  $ch = [int]($src.Height * 0.30)
  $strip = [System.Drawing.Bitmap]::new($src.Width, $ch)
  $gs = [System.Drawing.Graphics]::FromImage($strip)
  $gs.Clear([System.Drawing.ColorTranslator]::FromHtml($bg))
  $gs.DrawImage($src, (New-Object System.Drawing.Rectangle(0, 0, $src.Width, $ch)), (New-Object System.Drawing.Rectangle(0, ($src.Height - $ch), $src.Width, $ch)), [System.Drawing.GraphicsUnit]::Pixel)
  $gs.Dispose()
  $zoom = [System.Drawing.Bitmap]::new(($strip.Width * 4), ($strip.Height * 4))
  $gz = [System.Drawing.Graphics]::FromImage($zoom)
  $gz.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
  $gz.DrawImage($strip, 0, 0, $zoom.Width, $zoom.Height)
  $gz.Dispose()
  $zoom.Save("$out\$name-zoom.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $zoom.Dispose(); $strip.Dispose(); $src.Dispose()
  Write-Output "saved: $name (+zoom)"
}

# IIFE 必须 async（await 在普通箭头函数里是语法错误，本脚本首跑翻车即此因）；
# 返回对象由 CDP returnByValue 带出，ConvertFrom-Json 一次即得
function Pomo-Eval([string]$js) { (Seq "eval (async()=>{ $js })()") | ConvertFrom-Json }

# 读原配置（恢复用），临时改 1/1/1 分钟（set_config 会停表回 idle，但 phase 保持原样）
$orig = Pomo-Eval "return await window.__TAURI__.core.invoke('get_config')"
Write-Output "orig: work=$($orig.work_ms) short=$($orig.short_break_ms) long=$($orig.long_break_ms) every=$($orig.long_break_every) auto=$($orig.auto_start_next)"
Pomo-Eval "await window.__TAURI__.core.invoke('set_config',{workMin:1,shortBreakMin:1,longBreakMin:1,longBreakEvery:4,autoStartNext:false}); return 1" | Out-Null

# 相位归一：set_config 不改 phase，若接管时不在 work（如残留计时刚自然完成翻了相位），skip 回 work
for ($i=0; $i -lt 3; $i++) {
  $ph = Pomo-Eval "const s=await window.__TAURI__.core.invoke('timer_snapshot'); return s.phase"
  if ($ph -eq 'work') { break }
  Pomo-Eval "await window.__TAURI__.core.invoke('timer_skip'); return 1" | Out-Null
}

# —— 态1：专注中段（1 分钟工作跑 ~22s ≈ 37%）——
Pomo-Eval "await window.__TAURI__.core.invoke('timer_start'); return 1" | Out-Null
Start-Sleep -Seconds 22
$s1 = Pomo-Eval "const s=await window.__TAURI__.core.invoke('timer_snapshot'); return {phase:s.phase,status:s.status,done:+(1-s.remaining_ms/s.total_ms).toFixed(3),bar:document.getElementById('pill-progress-bar').style.width}"
if ($s1.phase -ne 'work' -or $s1.status -ne 'running') { throw "态1相位断言失败: $($s1 | ConvertTo-Json -Compress)" }
Write-Output "work-mid: phase=$($s1.phase) status=$($s1.status) done=$($s1.done) bar=$($s1.bar)"
Save-State "#E5E5EA" "progress-work-mid"

# —— 态2：休息中段（skip 进短休，跑 ~16s ≈ 27%）——
Pomo-Eval "await window.__TAURI__.core.invoke('timer_skip'); await window.__TAURI__.core.invoke('timer_start'); return 1" | Out-Null
Start-Sleep -Seconds 16
$s2 = Pomo-Eval "const s=await window.__TAURI__.core.invoke('timer_snapshot'); return {phase:s.phase,status:s.status,done:+(1-s.remaining_ms/s.total_ms).toFixed(3),bar:document.getElementById('pill-progress-bar').style.width}"
if ($s2.phase -ne 'short_break' -or $s2.status -ne 'running') { throw "态2相位断言失败: $($s2 | ConvertTo-Json -Compress)" }
Write-Output "break-mid: phase=$($s2.phase) status=$($s2.status) done=$($s2.done) bar=$($s2.bar)"
Save-State "#E5E5EA" "progress-break-mid"

# —— 态3：暂停冻结（回工作跑 12s 后暂停，两次采样宽度必须不变）——
Pomo-Eval "await window.__TAURI__.core.invoke('timer_skip'); await window.__TAURI__.core.invoke('timer_start'); return 1" | Out-Null
Start-Sleep -Seconds 12
Pomo-Eval "await window.__TAURI__.core.invoke('timer_pause'); return 1" | Out-Null
Start-Sleep -Milliseconds 600
$p1 = Pomo-Eval "return document.getElementById('pill-progress-bar').style.width"
Start-Sleep -Milliseconds 900
$p2 = Pomo-Eval "const s=await window.__TAURI__.core.invoke('timer_snapshot'); return {status:s.status,remaining:s.remaining_ms,bar:document.getElementById('pill-progress-bar').style.width}"
Write-Output "paused: w1=$p1 status=$($p2.status) remaining=$($p2.remaining) w2=$($p2.bar)"
Save-State "#E5E5EA" "progress-paused"

# —— 恢复：停表 + 原配置写回 + 校验落盘 ——
Pomo-Eval "await window.__TAURI__.core.invoke('timer_reset'); return 1" | Out-Null
Pomo-Eval "await window.__TAURI__.core.invoke('set_config',{workMin:$([int]($orig.work_ms/60000)),shortBreakMin:$([int]($orig.short_break_ms/60000)),longBreakMin:$([int]($orig.long_break_ms/60000)),longBreakEvery:$($orig.long_break_every),autoStartNext:$($orig.auto_start_next.ToString().ToLower())}); return 1" | Out-Null
$now = Get-Content "$exeDir\config.json" -Raw
Write-Output "config.json after restore: $now"
# stats 不应有污染（各阶段均未跑完），保险起见比对哈希，变了就回滚
$h1 = (Get-FileHash "$exeDir\stats.json").Hash
$h2 = (Get-FileHash "$exeDir\stats.json.bak-v11").Hash
if ($h1 -ne $h2) { Copy-Item "$exeDir\stats.json.bak-v11" "$exeDir\stats.json" -Force; Write-Output "stats.json restored from bak" } else { Write-Output "stats.json untouched" }
Write-Output "v11 progress capture done"
