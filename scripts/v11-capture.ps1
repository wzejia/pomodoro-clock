# v11 评审轮截图采集：八态（迷你/面板/抽屉/菜单 × 明暗，液态玻璃）+ 经典迷你对照
# 与 v61 差异：临时改 1 分钟时长让胶囊进度条有可见填充（评审关注点）；采集全程 <46s 防工作阶段完成翻相位
# 用法：pwsh scripts\v11-capture.ps1 -Round 40
param([Parameter(Mandatory=$true)][int]$Round)
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -AssemblyName System.Drawing

$out = "D:\VibeCoding\pomodoro-clock\docs\review"
$tmpShot = "$script:ShotTmp\_shot11r.png"
$exeDir = "D:\VibeCoding\pomodoro-clock\src-tauri\target\debug"
Copy-Item "$exeDir\config.json" "$exeDir\config.json.bak-v11r" -Force
Copy-Item "$exeDir\stats.json" "$exeDir\stats.json.bak-v11r" -Force

function Composite([string]$bg, [string]$dest) {
  $src = [System.Drawing.Bitmap]::new($tmpShot)
  $pad = 60
  $bmp = [System.Drawing.Bitmap]::new(($src.Width + 2*$pad), ($src.Height + 2*$pad))
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.Clear([System.Drawing.ColorTranslator]::FromHtml($bg))
  $g.DrawImage($src, $pad, $pad, $src.Width, $src.Height)
  $bmp.Save($dest, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose(); $src.Dispose()
  Write-Output "saved: $dest"
}
function Shot { Seq "shot $tmpShot" | Out-Null; Start-Sleep -Milliseconds 200 }
function Pomo-Eval([string]$js) { (Seq "eval (async()=>{ $js })()") | ConvertFrom-Json }
function Set-Theme([string]$t) {
  Seq ("eval document.documentElement.dataset.theme='$t';document.querySelectorAll('.theme-opt').forEach(b=>b.classList.toggle('active',b.dataset.themeOpt==='$t'));'$t'") | Out-Null
  Start-Sleep -Milliseconds 400
}
function Set-Material([string]$m) {
  Seq ("eval document.documentElement.dataset.material='$m';document.querySelectorAll('.material-opt').forEach(b=>b.classList.toggle('active',b.dataset.materialOpt==='$m'));'$m'") | Out-Null
  Start-Sleep -Milliseconds 400
}
function Drawer([bool]$open) {
  $v = if ($open) { "true" } else { "false" }
  Seq ("eval document.getElementById('app').classList.toggle('drawer-open',$v);'$v'") | Out-Null
  Start-Sleep -Milliseconds 600
}
function Ctx([bool]$open) {
  if ($open) {
    Seq "eval (()=>{const m=document.getElementById('ctx-menu');m.style.left='180px';m.style.top='200px';m.classList.add('open');return 'open'})()" | Out-Null
  } else {
    Seq "eval document.getElementById('ctx-menu').classList.remove('open');'closed'" | Out-Null
  }
  Start-Sleep -Milliseconds 400
}

# 现场记录（采集完恢复 DOM 主题/材质——并行干预纪律：不替用户做选择，只借不改变）
$preTheme = Pomo-Eval "return document.documentElement.dataset.theme"
$preMaterial = Pomo-Eval "return document.documentElement.dataset.material"
Write-Output "pre: theme=$preTheme material=$preMaterial"

# 临时 1/1/1 分钟 + 相位归一到 work + 起跑（进度条 ~15s 后约 25% 起可见）
$orig = Pomo-Eval "return await window.__TAURI__.core.invoke('get_config')"
Pomo-Eval "await window.__TAURI__.core.invoke('set_config',{workMin:1,shortBreakMin:1,longBreakMin:1,longBreakEvery:4,autoStartNext:false}); return 1" | Out-Null
for ($i=0; $i -lt 3; $i++) {
  $ph = Pomo-Eval "const s=await window.__TAURI__.core.invoke('timer_snapshot'); return s.phase"
  if ($ph -eq 'work') { break }
  Pomo-Eval "await window.__TAURI__.core.invoke('timer_skip'); return 1" | Out-Null
}
Pomo-Eval "await window.__TAURI__.core.invoke('timer_start'); return 1" | Out-Null

Move-Pomo 1500 300
Set-Material liquid_glass
Set-Theme light
Start-Sleep -Seconds 15  # 进度条走到 ~25%

# 1) mini 浅色（进度条可见）
Shot; Composite "#E5E5EA" "$out\round-$Round-mini.png"
# 2) 展开 浅色
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 1400
Shot; Composite "#E5E5EA" "$out\round-$Round-panel.png"
# 3) 抽屉 浅色（快捷键新行在列）
Drawer $true
Shot; Composite "#E5E5EA" "$out\round-$Round-drawer.png"
Drawer $false
# 4) 右键菜单 浅色
Ctx $true
Shot; Composite "#E5E5EA" "$out\round-$Round-ctx.png"
Ctx $false

Set-Theme dark
# 5) 展开 深色
Shot; Composite "#2C2C2E" "$out\round-$Round-panel-dark.png"
# 6) 抽屉 深色
Drawer $true
Shot; Composite "#2C2C2E" "$out\round-$Round-drawer-dark.png"
Drawer $false
# 7) 右键菜单 深色
Ctx $true
Shot; Composite "#2C2C2E" "$out\round-$Round-ctx-dark.png"
Ctx $false
# 8) mini 深色（Esc 收起走 v7 链路）
Esc-Collapse; Start-Sleep -Milliseconds 1200
Shot; Composite "#2C2C2E" "$out\round-$Round-mini-dark.png"

# 9) 经典材质迷你对照（进度条两档都生效的证据；timer 仍在跑，相位同一）
Set-Theme light
Set-Material classic
Shot; Composite "#E5E5EA" "$out\round-$Round-classic-mini.png"
Set-Material liquid_glass

# —— 恢复：停表 + 原配置 + 原主题/材质 + stats 防污染比对 ——
Pomo-Eval "await window.__TAURI__.core.invoke('timer_reset'); return 1" | Out-Null
Pomo-Eval "await window.__TAURI__.core.invoke('set_config',{workMin:$([int]($orig.work_ms/60000)),shortBreakMin:$([int]($orig.short_break_ms/60000)),longBreakMin:$([int]($orig.long_break_ms/60000)),longBreakEvery:$($orig.long_break_every),autoStartNext:$($orig.auto_start_next.ToString().ToLower())}); return 1" | Out-Null
Set-Material $preMaterial
Set-Theme $preTheme
$h1 = (Get-FileHash "$exeDir\stats.json").Hash
$h2 = (Get-FileHash "$exeDir\stats.json.bak-v11r").Hash
if ($h1 -ne $h2) { Copy-Item "$exeDir\stats.json.bak-v11r" "$exeDir\stats.json" -Force; Write-Output "stats.json restored from bak" } else { Write-Output "stats.json untouched" }
Write-Output "round-$Round captured: 9 shots (liquid x8 + classic mini)"
