# v6.1 评审轮截图采集：四界面 × 明暗 × 当前材质（CDP 离屏 shot + 底色合成）
# 用法：pwsh scripts\v61-capture.ps1 -Round 27 [-Material liquid_glass|classic]
param([Parameter(Mandatory=$true)][int]$Round, [string]$Material = "liquid_glass")
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -AssemblyName System.Drawing

$out = "D:\VibeCoding\pomodoro-clock\docs\review"
$tmpShot = "$script:ShotTmp\_shot61.png"
$tag = if ($Material -eq "classic") { "classic-" } else { "" }

function Composite([string]$bg, [string]$dest) {
  $src = New-Object System.Drawing.Bitmap($tmpShot)
  $pad = 60
  $bmp = New-Object System.Drawing.Bitmap ($src.Width + 2*$pad), ($src.Height + 2*$pad)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.Clear([System.Drawing.ColorTranslator]::FromHtml($bg))
  $g.DrawImage($src, $pad, $pad, $src.Width, $src.Height)
  $bmp.Save($dest, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose(); $src.Dispose()
  Write-Output "saved: $dest"
}
function Shot { Seq "shot $tmpShot" | Out-Null; Start-Sleep -Milliseconds 200 }
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
    # IIFE：顶层 const 会在页面全局词法环境残留，二次调用报重声明（R27 P1 采集事故）
    Seq "eval (()=>{const m=document.getElementById('ctx-menu');m.style.left='180px';m.style.top='200px';m.classList.add('open');return 'open'})()" | Out-Null
  } else {
    Seq "eval document.getElementById('ctx-menu').classList.remove('open');'closed'" | Out-Null
  }
  Start-Sleep -Milliseconds 400
}

Move-Pomo 1500 300
Seq "eval window.__TAURI__.core.invoke('timer_start').then(()=>'ok')" | Out-Null
Start-Sleep -Seconds 2
Set-Material $Material

Set-Theme light
# 1) mini 浅色
Shot; Composite "#E5E5EA" "$out\round-$Round-$($tag)mini.png"
# 2) 展开 浅色
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 1400
Shot; Composite "#E5E5EA" "$out\round-$Round-$($tag)panel.png"
# 3) 抽屉 浅色
Drawer $true
Shot; Composite "#E5E5EA" "$out\round-$Round-$($tag)drawer.png"
Drawer $false
# 4) 右键菜单 浅色
Ctx $true
Shot; Composite "#E5E5EA" "$out\round-$Round-$($tag)ctx.png"
Ctx $false

Set-Theme dark
# 5) 展开 深色
Shot; Composite "#2C2C2E" "$out\round-$Round-$($tag)panel-dark.png"
# 6) 抽屉 深色
Drawer $true
Shot; Composite "#2C2C2E" "$out\round-$Round-$($tag)drawer-dark.png"
Drawer $false
# 7) 右键菜单 深色
Ctx $true
Shot; Composite "#2C2C2E" "$out\round-$Round-$($tag)ctx-dark.png"
Ctx $false
# 8) mini 深色（Esc 收起走 v7 链路）
Esc-Collapse; Start-Sleep -Milliseconds 1200
Shot; Composite "#2C2C2E" "$out\round-$Round-$($tag)mini-dark.png"

Set-Theme light
Write-Output "round-$Round ($Material) captured: 8 shots"
