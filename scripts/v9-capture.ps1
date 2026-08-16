# v9 悬停/按压帧采集驱动（修复前后通用）
# 用法：v9-capture.ps1 -Tag pre        —— 修复前基线（浅色，三按钮 hover+press）
#       v9-capture.ps1 -Tag r36 -Dark  —— 评审轮（明暗两套 + mini 暂停钮 + 收起把手 + 步进器按压）
# 注意：脚本结束恢复 mini 起始态（R33 教训：补拍脚本必须恢复窗口起始状态）
param([Parameter(Mandatory=$true)][string]$Tag, [switch]$Dark)
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -AssemblyName System.Drawing

$tmp = $script:ShotTmp
$mjs = "D:\VibeCoding\pomodoro-clock\scripts\v9-hover.mjs"
$out = "D:\VibeCoding\pomodoro-clock\docs\review"

function Hover([string]$name, [int]$x, [int]$y) { & bun $mjs hover "$tmp\v9-$Tag-$name" $x $y 2>$null | Out-Null }
function Press([string]$name, [int]$x, [int]$y) { & bun $mjs press "$tmp\v9-$Tag-$name" $x $y 2>$null | Out-Null }

# 以 (cx,cy) 为中心取 w×h，NearestNeighbor 放大 zoom 倍，贴到底色板上
function Crop-Zoom([string]$src, [int]$cx, [int]$cy, [int]$w, [int]$h, [int]$zoom, [string]$bg, [string]$dest) {
  $img = New-Object System.Drawing.Bitmap($src)
  $x0 = [Math]::Max(0, [Math]::Min($cx - [int]($w / 2), $img.Width - $w))
  $y0 = [Math]::Max(0, [Math]::Min($cy - [int]($h / 2), $img.Height - $h))
  $crop = $img.Clone([System.Drawing.Rectangle]::new($x0, $y0, $w, $h), $img.PixelFormat)
  $bmp = [System.Drawing.Bitmap]::new($w * $zoom, $h * $zoom)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.Clear([System.Drawing.ColorTranslator]::FromHtml($bg))
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
  # 注意：New-Object 的括号参数表是参数模式，`$w*$zoom` 不会被求值（会报 Object[]→UInt32），必须用 ::new()
  $g.DrawImage($crop, [System.Drawing.Rectangle]::new(0, 0, $w * $zoom, $h * $zoom), [System.Drawing.Rectangle]::new(0, 0, $w, $h), [System.Drawing.GraphicsUnit]::Pixel)
  $bmp.Save($dest, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose(); $crop.Dispose(); $img.Dispose()
}

# 取元素中心（cdp-seq eval 返回对象=单层 JSON；返回字符串才会双重编码）
function Centers([string]$ids) {
  $js = "(()=>{const o={};'$ids'.split(',').forEach(id=>{const b=document.getElementById(id);if(!b)return;const r=b.getBoundingClientRect();o[id]={x:Math.round(r.x+r.width/2),y:Math.round(r.y+r.height/2),w:Math.round(r.width),h:Math.round(r.height)}});return o})()"
  return ((Seq "eval $js") | ConvertFrom-Json)
}

function Set-Theme([string]$t) {
  Seq ("eval document.documentElement.dataset.theme='$t';document.querySelectorAll('.theme-opt').forEach(b=>b.classList.toggle('active',b.dataset.themeOpt==='$t'));'$t'") | Out-Null
  Start-Sleep -Milliseconds 400
}

function Capture-Set([string]$suffix, [string]$bg) {
  # 面板三按钮：hover 三帧 + press 两帧；裁切按钮中心区放大 4 倍
  $c = Centers "btn-primary,btn-reset,btn-switch,btn-collapse"
  foreach ($id in @("btn-primary", "btn-reset", "btn-switch")) {
    $b = $c.$id
    Hover "$id$suffix" $b.x $b.y
    Press "$id$suffix" $b.x $b.y
    foreach ($f in @("mid1", "mid2", "end", "press2")) {
      $src = "$tmp\v9-$Tag-$id$suffix-$f.png"
      if (Test-Path $src) { Crop-Zoom $src $b.x $b.y 100 44 4 $bg "$out\round-36-$Tag-$id$suffix-$f-x4.png" }
    }
  }
  # 收起把手 hover（不 press，press 也无 active 规则）
  $b = $c."btn-collapse"
  Hover "collapse$suffix" $b.x $b.y
  foreach ($f in @("mid1", "end")) {
    $src = "$tmp\v9-$Tag-collapse$suffix-$f.png"
    if (Test-Path $src) { Crop-Zoom $src $b.x $b.y 90 34 4 $bg "$out\round-36-$Tag-collapse$suffix-$f-x4.png" }
  }
}

Move-Pomo 1500 300
# 展开面板（宽容点击链路）
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 1400

Set-Theme light
Capture-Set "" "#E5E5EA"

if ($Dark) {
  Set-Theme dark
  Capture-Set "-dark" "#2C2C2E"
  # mini 暂停钮 hover：先收起（Esc 链路），timer 未启动时 mini 只有开始钮（icon-btn）
  Esc-Collapse; Start-Sleep -Milliseconds 1200
  $m = Centers "btn-toggle-run"
  $b = $m."btn-toggle-run"
  Hover "mini-btn-dark" $b.x $b.y
  foreach ($f in @("mid1", "end")) {
    $src = "$tmp\v9-$Tag-mini-btn-dark-$f.png"
    if (Test-Path $src) { Crop-Zoom $src $b.x $b.y 40 40 5 "#2C2C2E" "$out\round-36-$Tag-mini-btn-dark-$f-x5.png" }
  }
  # 步进器按压：重新展开 + 开抽屉
  Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 1400
  Seq "eval document.getElementById('app').classList.add('drawer-open');'open'" | Out-Null
  Start-Sleep -Milliseconds 700
  $s = Seq "eval (()=>{const b=document.querySelector('.step-btn');const r=b.getBoundingClientRect();return {x:Math.round(r.x+r.width/2),y:Math.round(r.y+r.height/2)}})()"
  $sb = $s | ConvertFrom-Json
  Press "step-btn-dark" $sb.x $sb.y
  $src = "$tmp\v9-$Tag-step-btn-dark-press2.png"
  if (Test-Path $src) { Crop-Zoom $src $sb.x $sb.y 46 34 5 "#2C2C2E" "$out\round-36-$Tag-step-btn-dark-press2-x5.png" }
  Seq "eval document.getElementById('app').classList.remove('drawer-open');'closed'" | Out-Null
  Start-Sleep -Milliseconds 400
}

# 恢复起始态：关抽屉 + Esc 收起 + 浅色（R33 教训）
Seq "eval document.getElementById('app').classList.remove('drawer-open');'closed'" | Out-Null
Esc-Collapse; Start-Sleep -Milliseconds 1200
Set-Theme light
Write-Output "v9-$Tag captured"
