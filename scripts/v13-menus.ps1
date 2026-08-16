# v13 液态菜单区分度采集：应用内右键（明暗×两材质 4 张）+ 托盘弹出（明暗×两材质 4 张）
#   + 液态透字检验（ghost：菜单盖纯黑|纯白高对比内容，RGB 采样 + 对比度拉伸）
#   + 顺带采集几何数据（两菜单 rect/项高 × 四组合 → v13-geo.json，供任务 3 复测断言）
# 用法：pwsh scripts\v13-menus.ps1（一次跑全 → docs/screenshots/v13/）
# 纪律（v13.1 管线硬化）：染料三段（应用内/ghost/托盘）包 try/finally，finally 保底还原——
#       中途 throw 不再留悬空态（v13 次跑 mouse_event ArgException 中断后靠手动救回的教训）；
#       还原 material=开工【现读 config.json】的持久值（勿硬编码/勿信文档快照，外部会改 config）；
#       主题/材质只翻 DOM dataset + ui-style 广播，不动 config.json / stats.json / timer；
#       采集顺带产出 8 张 4x zoom 放大图（zoom/ 子目录），评审员提示词直接指图省自采。
# 坑位照单：cdp-seq eval 返对象才单层 JSON；.NET 构造 ::new()；eval 一律 IIFE；pwsh 7 运行。
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing

$out = "D:\VibeCoding\pomodoro-clock\docs\screenshots\v13"
New-Item -ItemType Directory -Force $out | Out-Null
$AE = [System.Windows.Automation.AutomationElement]

Add-Type -Name TM -Namespace V13Tray -MemberDefinition '
 public delegate bool EnumWindowsProc(System.IntPtr h, uint param);
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, uint param);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr h, out uint pid);
 [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(System.IntPtr h, System.Text.StringBuilder sb, int max);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(System.IntPtr h);
 public static System.IntPtr FindMenuHwnd(uint pid) {
   System.IntPtr found = System.IntPtr.Zero;
   EnumWindowsProc cb = (h, p) => {
     uint wp; GetWindowThreadProcessId(h, out wp);
     if (wp != pid) return true;
     var sb = new System.Text.StringBuilder(64); GetWindowTextW(h, sb, 64);
     if (sb.ToString() == "番茄钟菜单") { found = h; return false; }
     return true;
   };
   EnumWindows(cb, 0);
   System.GC.KeepAlive(cb);
   return found;
 }
'
Add-Type -Name ME2 -Namespace V13TrayME -MemberDefinition '[DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint data, System.IntPtr extra);'

function Menu-Visible { return [V13Tray.TM]::IsWindowVisible([V13Tray.TM]::FindMenuHwnd([uint32]$script:PomoPid)) }
function Click-At([int]$x, [int]$y, [string]$btn) {
  [PomoWin.U32]::SetCursorPos($x, $y) | Out-Null
  Start-Sleep -Milliseconds 150
  $down = if ($btn -eq "right") { 0x0008 } else { 0x0002 }
  $up   = if ($btn -eq "right") { 0x0010 } else { 0x0004 }
  [V13TrayME.ME2]::mouse_event([uint32]$down, 0, 0, 0, [System.IntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [V13TrayME.ME2]::mouse_event([uint32]$up, 0, 0, 0, [System.IntPtr]::Zero)
}
function Open-Overflow {
  $cond = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
    [System.Windows.Automation.ControlType]::Button)
  $all = $AE::RootElement.FindAll([System.Windows.Automation.TreeScope]::Subtree, $cond)
  foreach ($b in $all) {
    try {
      if ($b.Current.Name -match "隐藏的图标|Show hidden icons") {
        $r = $b.Current.BoundingRectangle
        Click-At ([int]($r.X + $r.Width / 2)) ([int]($r.Y + $r.Height / 2)) "left"
        Start-Sleep -Milliseconds 700
        return
      }
    } catch {}
  }
}
function Find-OverflowIcon {
  for ($i = 0; $i -lt 3; $i++) {
    $cond = New-Object System.Windows.Automation.PropertyCondition(
      [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
      [System.Windows.Automation.ControlType]::Button)
    $all = $AE::RootElement.FindAll([System.Windows.Automation.TreeScope]::Subtree, $cond)
    foreach ($b in $all) {
      try {
        if ($b.Current.Name -like "*番茄钟*" -and $b.Current.BoundingRectangle.Width -le 60) { return $b }
      } catch {}
    }
    Open-Overflow
  }
  return $null
}
function Open-TrayMenu {
  $icon = Find-OverflowIcon
  if (-not $icon) { throw "托盘溢出面板图标未找到" }
  $r = $icon.Current.BoundingRectangle
  Click-At ([int]($r.X + $r.Width / 2)) ([int]($r.Y + $r.Height / 2)) "right"
  Start-Sleep -Milliseconds 900
  if (-not (Menu-Visible)) { throw "右键后托盘菜单窗不可见" }
  # 真实微移生成 WM_MOUSEMOVE 重命中：光标就在图标上（菜单窗外），清掉任何残留 hover
  # （SetCursorPos 无鼠标消息不可靠——v13 首跑四张托盘图「开机自启」被钉 hover 蓝底事故；
  #  反例 v12 同流程不归位零残留。单次 +1px 足够触发重命中；dx 是 DWORD，负相对位移要传 0xFFFFFFFF）
  [V13TrayME.ME2]::mouse_event(0x0001, 1, 0, 0, [System.IntPtr]::Zero)
  Start-Sleep -Milliseconds 350
}
function Composite([string]$tmp, [string]$bg, [string]$dest, [int]$pad) {
  $src = [System.Drawing.Bitmap]::new($tmp)
  $bmp = [System.Drawing.Bitmap]::new(($src.Width + 2*$pad), ($src.Height + 2*$pad))
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.Clear([System.Drawing.ColorTranslator]::FromHtml($bg))
  $g.DrawImage($src, $pad, $pad, $src.Width, $src.Height)
  $bmp.Save($dest, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose(); $src.Dispose()
  Write-Output "saved: $dest"
}
function Shot-Main([string]$name, [string]$bg) {
  $tmp = "$script:ShotTmp\_v13main.png"
  Seq "shot $tmp" | Out-Null; Start-Sleep -Milliseconds 200
  Copy-Item $tmp "$script:ShotTmp\v13-raw-$name.png" -Force
  Composite $tmp $bg "$out\$name.png" 60
}
function Shot-Tray([string]$name, [string]$bg) {
  $tmp = "$script:ShotTmp\_v13tray.png"
  $env:CDP_MATCH = "tray-menu"
  try { Seq "shot $tmp" | Out-Null } finally { Remove-Item Env:CDP_MATCH }
  Start-Sleep -Milliseconds 200
  Copy-Item $tmp "$script:ShotTmp\v13-raw-$name.png" -Force
  Composite $tmp $bg "$out\$name.png" 24
}
function Tray-Eval([string]$js) {
  $env:CDP_MATCH = "tray-menu"
  try { (Seq "eval (async()=>{ $js })()") | ConvertFrom-Json } finally { Remove-Item Env:CDP_MATCH }
}
function Main-Eval([string]$js) { (Seq "eval (()=>{ $js })()") | ConvertFrom-Json }
function Set-Style([string]$theme, [string]$material, [bool]$broadcast) {
  $js = "document.documentElement.dataset.theme='$theme';document.documentElement.dataset.material='$material';"
  if ($broadcast) { $js += "window.__TAURI__.event.emit('ui-style',{theme:'$theme',material:'$material'});" }
  Main-Eval "$js return 'ok'" | Out-Null
  Start-Sleep -Milliseconds 300
}
# 几何采集：菜单 rect + 各项高（IIFE，返对象单层）
$GEO_JS = "const m=document.querySelector('.ctx-menu');const r=m.getBoundingClientRect();const its=[...m.querySelectorAll('.ctx-item')].map(i=>i.offsetHeight);return {x:r.x,y:r.y,w:r.width,h:r.height,ow:m.offsetWidth,items:its,iw:innerWidth,ih:innerHeight}"
$script:geo = @{}

# 并排对比图：两张同尺寸 crop 2x nearest 左右拼（左 classic 右 liquid）
function Pair([string]$rawA, [string]$rawB, [int]$cx, [int]$cy, [int]$cw, [int]$ch, [string]$bg, [string]$dest) {
  $zoom = 2; $pad = 12
  $sw = $cw * $zoom; $sh = $ch * $zoom
  $bmp = [System.Drawing.Bitmap]::new(($sw*2 + $pad*3), ($sh + $pad*2))
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
  $g.Clear([System.Drawing.ColorTranslator]::FromHtml($bg))
  $sa = [System.Drawing.Bitmap]::new($rawA); $sb = [System.Drawing.Bitmap]::new($rawB)
  $da = [System.Drawing.Rectangle]::new($pad, $pad, $sw, $sh)
  $db = [System.Drawing.Rectangle]::new(($pad*2 + $sw), $pad, $sw, $sh)
  $g.DrawImage($sa, $da, $cx, $cy, $cw, $ch, [System.Drawing.GraphicsUnit]::Pixel)
  $g.DrawImage($sb, $db, $cx, $cy, $cw, $ch, [System.Drawing.GraphicsUnit]::Pixel)
  $bmp.Save($dest, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose(); $sa.Dispose(); $sb.Dispose()
  Write-Output "saved: $dest"
}

# --- 起始态记录（还原目标=开工现读：material 取 config.json 持久值，theme 取开工实测值） ---
$cfg = Get-Content D:\VibeCoding\pomodoro-clock\src-tauri\target\debug\config.json -Raw | ConvertFrom-Json
$cfgMaterial = $cfg.material
$st = App-State
$st2 = Main-Eval "return {theme:document.documentElement.dataset.theme,material:document.documentElement.dataset.material,iw:innerWidth,ih:innerHeight}"
$startTheme = $st2.theme
Write-Output "start state: $($st.cls) menu=$($st.menu) $($st2.iw)x$($st2.ih) theme=$startTheme material=$($st2.material)（config 现读持久 material=$cfgMaterial）"
[PomoWin.U32]::SetCursorPos(400, 400) | Out-Null
$zoomOut = "$out\zoom"
New-Item -ItemType Directory -Force $zoomOut | Out-Null

# Crop-Zoom：raw 图指定区 nearest 放大（评审证据预置，免评审员自采构图不一致）
function Crop-Zoom([string]$raw, [int]$cx, [int]$cy, [int]$cw, [int]$ch, [int]$zoom, [string]$dest) {
  $src = [System.Drawing.Bitmap]::new($raw)
  $bmp = [System.Drawing.Bitmap]::new(($cw * $zoom), ($ch * $zoom))
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
  $dst = [System.Drawing.Rectangle]::new(0, 0, ($cw * $zoom), ($ch * $zoom))
  $g.DrawImage($src, $dst, $cx, $cy, $cw, $ch, [System.Drawing.GraphicsUnit]::Pixel)
  $bmp.Save($dest, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose(); $src.Dispose()
}

# --- A. 应用内右键菜单：4 组合（明暗 × 两材质） ---
# 染料三段（A/B/C）整体包 try/finally：finally 保底还原，中途 throw 不留悬空态
try {
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 1400
$combos = @(
  @("light", "classic", "v13-ctx-light-classic", "#E5E5EA"),
  @("light", "liquid_glass", "v13-ctx-light-liquid", "#E5E5EA"),
  @("dark", "classic", "v13-ctx-dark-classic", "#2C2C2E"),
  @("dark", "liquid_glass", "v13-ctx-dark-liquid", "#2C2C2E")
)
foreach ($c in $combos) {
  Set-Style $c[0] $c[1] $false
  Main-Eval "const app=document.querySelector('.app');app.dispatchEvent(new MouseEvent('contextmenu',{clientX:170,clientY:200,bubbles:true,cancelable:true}));return 'open'" | Out-Null
  Start-Sleep -Milliseconds 500
  $script:geo["ctx-$($c[0])-$($c[1])"] = Main-Eval $GEO_JS
  Shot-Main $c[2] $c[3]
  Crop-Zoom "$script:ShotTmp\v13-raw-$($c[2]).png" 170 200 104 89 4 "$zoomOut\v13-zoom-$($c[2]).png"; Write-Output "zoom: v13-zoom-$($c[2]).png"
  # 真实外点关菜单（pointerdown 捕获监听 → hideCtxMenu）
  Seq "eval document.dispatchEvent(new PointerEvent('pointerdown',{bubbles:true}));'closed'" | Out-Null
  Start-Sleep -Milliseconds 300
}
# 应用内并排对比（裁菜单区：菜单开于 (170,200) 104×89，外扩 16）
Pair "$script:ShotTmp\v13-raw-v13-ctx-light-classic.png" "$script:ShotTmp\v13-raw-v13-ctx-light-liquid.png" 154 184 136 121 "#E5E5EA" "$out\v13-pair-ctx-light.png"
Pair "$script:ShotTmp\v13-raw-v13-ctx-dark-classic.png" "$script:ShotTmp\v13-raw-v13-ctx-dark-liquid.png" 154 184 136 121 "#2C2C2E" "$out\v13-pair-ctx-dark.png"

# --- B. 透字检验（液态档盖纯黑|纯白高对比内容；菜单仍开 (170,200)） ---
Set-Style "dark" "liquid_glass" $false
Main-Eval "const p=document.createElement('div');p.id='v13ghost';p.style.cssText='position:fixed;left:140px;top:170px;width:180px;height:140px;z-index:1;display:flex;';p.innerHTML='<div style=""flex:1;background:#000;color:#fff;font:700 26px sans-serif;display:grid;place-items:center;"">番茄</div><div style=""flex:1;background:#fff;color:#000;font:700 26px sans-serif;display:grid;place-items:center;"">GHOST</div>';document.body.appendChild(p);return 'pattern'" | Out-Null
Start-Sleep -Milliseconds 200
foreach ($g in @(@("dark", "#2C2C2E"), @("light", "#E5E5EA"))) {
  Set-Style $g[0] "liquid_glass" $false
  Main-Eval "const app=document.querySelector('.app');app.dispatchEvent(new MouseEvent('contextmenu',{clientX:170,clientY:200,bubbles:true,cancelable:true}));return 'open'" | Out-Null
  Start-Sleep -Milliseconds 500
  $tmp = "$script:ShotTmp\_v13ghost.png"
  Seq "shot $tmp" | Out-Null; Start-Sleep -Milliseconds 200
  Copy-Item $tmp "$script:ShotTmp\v13-raw-ghost-$($g[0]).png" -Force
  Composite $tmp $g[1] "$out\v13-ghost-$($g[0])-raw.png" 60
  # RGB 采样：菜单内避开字形的两条横带（顶 padding y=203、底 padding y=287），黑半区 vs 白半区
  $bmp = [System.Drawing.Bitmap]::new($tmp)
  $pxA = $bmp.GetPixel(190, 203); $pxB = $bmp.GetPixel(250, 203)
  $pxC = $bmp.GetPixel(190, 287); $pxD = $bmp.GetPixel(250, 287)
  $bmp.Dispose()
  $dTop = [Math]::Abs([int]$pxA.R - [int]$pxB.R); $dBot = [Math]::Abs([int]$pxC.R - [int]$pxD.R)
  Write-Output "ghost-$($g[0]) RGB: black@top=($($pxA.R),$($pxA.G),$($pxA.B)) white@top=($($pxB.R),$($pxB.G),$($pxB.B)) black@bottom=($($pxC.R),$($pxC.G),$($pxC.B)) white@bottom=($($pxD.R),$($pxD.G),$($pxD.B)) | bleed-delta top=$dTop bottom=$dBot"
  # 对比度拉伸裁切（菜单 rect 170,200,104,89 → 3x nearest）
  $src = [System.Drawing.Bitmap]::new($tmp)
  $cw = 104; $chh = 89; $min = 255; $max = 0
  for ($y = 200; $y -lt 200 + $chh; $y++) { for ($x = 170; $x -lt 170 + $cw; $x++) {
    $px = $src.GetPixel($x, $y); $l = [int]($px.R) + $px.G + $px.B
    if ($l -lt $min) { $min = $l }; if ($l -gt $max) { $max = $l }
  } }
  $range = [Math]::Max(1, $max - $min)
  $zoom = 3
  $out2 = [System.Drawing.Bitmap]::new(($cw * $zoom), ($chh * $zoom))
  for ($y = 0; $y -lt $chh; $y++) { for ($x = 0; $x -lt $cw; $x++) {
    $px = $src.GetPixel(170 + $x, 200 + $y)
    $f = { param($v) [Math]::Min(255, [Math]::Max(0, [int]((([int]$v * 3 - $min) * 255 / $range)))) }
    $c = [System.Drawing.Color]::FromArgb((& $f $px.R), (& $f $px.G), (& $f $px.B))
    for ($dy = 0; $dy -lt $zoom; $dy++) { for ($dx = 0; $dx -lt $zoom; $dx++) { $out2.SetPixel($x * $zoom + $dx, $y * $zoom + $dy, $c) } }
  } }
  $out2.Save("$out\v13-ghost-$($g[0])-stretch.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $out2.Dispose(); $src.Dispose()
  Write-Output "saved: $out\v13-ghost-$($g[0])-stretch.png (levels min=$min max=$max)"
  Seq "eval document.dispatchEvent(new PointerEvent('pointerdown',{bubbles:true}));'closed'" | Out-Null
  Start-Sleep -Milliseconds 300
}
Main-Eval "const p=document.getElementById('v13ghost');if(p)p.remove();return 'clean'" | Out-Null
Esc-Collapse; Start-Sleep -Milliseconds 1200

# --- C. 托盘菜单：明暗 × 两材质（真实 ui-style 广播链路 + 真实右键） ---
$trayCombos = @(
  @("light", "classic", "v13-tray-light-classic", "#E5E5EA"),
  @("light", "liquid_glass", "v13-tray-light-liquid", "#E5E5EA"),
  @("dark", "classic", "v13-tray-dark-classic", "#2C2C2E"),
  @("dark", "liquid_glass", "v13-tray-dark-liquid", "#2C2C2E")
)
$ti = 0
foreach ($c in $trayCombos) {
  Set-Style $c[0] $c[1] $true
  Open-TrayMenu
  $script:geo["tray-$($c[0])-$($c[1])"] = Tray-Eval "const m=document.querySelector('.ctx-menu');const r=m.getBoundingClientRect();const its=[...m.querySelectorAll('.ctx-item')].map(i=>i.offsetHeight);return {x:r.x,y:r.y,w:r.width,h:r.height,ow:m.offsetWidth,items:its,theme:document.documentElement.dataset.theme,material:document.documentElement.dataset.material}"
  Shot-Tray $c[2] $c[3]
  Crop-Zoom "$script:ShotTmp\v13-raw-$($c[2]).png" 0 0 104 116 4 "$zoomOut\v13-zoom-$($c[2]).png"; Write-Output "zoom: v13-zoom-$($c[2]).png"
  $ti++
  if ($ti -lt $trayCombos.Count) { $ws = New-Object -ComObject WScript.Shell; $ws.SendKeys("{ESC}") } else { Tray-Eval "await window.__TAURI__.core.invoke('tray_menu_hide'); return 1" | Out-Null }
  Start-Sleep -Milliseconds 500
}
# 托盘并排对比（raw=菜单整窗 104×116）
Pair "$script:ShotTmp\v13-raw-v13-tray-light-classic.png" "$script:ShotTmp\v13-raw-v13-tray-light-liquid.png" 0 0 104 116 "#E5E5EA" "$out\v13-pair-tray-light.png"
Pair "$script:ShotTmp\v13-raw-v13-tray-dark-classic.png" "$script:ShotTmp\v13-raw-v13-tray-dark-liquid.png" 0 0 104 116 "#2C2C2E" "$out\v13-pair-tray-dark.png"

# 几何 JSON 落盘（任务 3 断言用）
$script:geo | ConvertTo-Json -Depth 4 | Set-Content "$out\v13-geo.json" -Encoding UTF8
Write-Output "saved: $out\v13-geo.json"

} finally {
  # --- 保底还原（无论中途是否 throw）：幂等动作逐步执行，单步失败不拦后续、不盖原异常 ---
  foreach ($step in @(
    @{ n = "ghost-pattern-clean"; j = "const p=document.getElementById('v13ghost');if(p)p.remove();return {ok:1}" ; tray = $false },
    @{ n = "esc-collapse";        j = $null;                                                                  tray = $false },
    @{ n = "style-restore";       j = "document.documentElement.dataset.theme='$startTheme';document.documentElement.dataset.material='$cfgMaterial';window.__TAURI__.event.emit('ui-style',{theme:'$startTheme',material:'$cfgMaterial'});return {ok:1}" ; tray = $false },
    @{ n = "tray-hide";           j = "await window.__TAURI__.core.invoke('tray_menu_hide'); return 1"                                       ; tray = $true }
  )) {
    try {
      if ($step.n -eq "esc-collapse") { Esc-Collapse }
      elseif ($step.tray) { Tray-Eval $step.j | Out-Null }
      else { Main-Eval "(()=>{ $($step.j) })()" | Out-Null }
    } catch { Write-Output "restore WARN [$($step.n)]: $($_.Exception.Message)" }
  }
  Start-Sleep -Milliseconds 600

  # --- 终态校验：mini、菜单关、material=开工现读 config 持久值、theme=开工实测、托盘 hidden ---
  $fin = App-State
  $fin2 = Main-Eval "return {theme:document.documentElement.dataset.theme,material:document.documentElement.dataset.material,iw:innerWidth,ih:innerHeight}"
  $trayVisible = Menu-Visible
  Write-Output "final state: $($fin.cls) menu=$($fin.menu) $($fin2.iw)x$($fin2.ih) theme=$($fin2.theme) material=$($fin2.material) trayVisible=$trayVisible"
  if ($fin.cls -eq "app mini" -and -not $fin.menu -and $fin2.theme -eq $startTheme -and $fin2.material -eq $cfgMaterial -and -not $trayVisible) {
    Write-Output "V13-CAPTURE PASS (8 shots + 4 pairs + 2 ghost + 8 zoom, state restored)"
  } else {
    Write-Output "V13-CAPTURE WARN: 状态未完全还原，请检查"
  }
}
