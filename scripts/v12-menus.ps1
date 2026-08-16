# v12 评审轮菜单截图采集：展开态右键菜单（明暗×两材质 4 张）+ 托盘菜单弹出（明暗 2 张）
# 用法：pwsh scripts\v12-menus.ps1（无参，一次跑全 6 张 → docs/screenshots/v12/）
# 纪律：起始/结束均为 mini 收起、菜单关、浅色、classic（领导配置）；不动 timer、不动 config.json；
#       主题/材质只翻 DOM dataset（托盘深色走真实 ui-style 广播链路）；UIA/mouse_event 段抄自 v11-tray.ps1。
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing

$out = "D:\VibeCoding\pomodoro-clock\docs\screenshots\v12"
New-Item -ItemType Directory -Force $out | Out-Null
$AE = [System.Windows.Automation.AutomationElement]

Add-Type -Name TM -Namespace V12Tray -MemberDefinition '
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
Add-Type -Name ME2 -Namespace V12TrayME -MemberDefinition '[DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint data, System.IntPtr extra);'

function Menu-Visible { return [V12Tray.TM]::IsWindowVisible([V12Tray.TM]::FindMenuHwnd([uint32]$script:PomoPid)) }
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
function Click-At([int]$x, [int]$y, [string]$btn) {
  [PomoWin.U32]::SetCursorPos($x, $y) | Out-Null
  Start-Sleep -Milliseconds 150
  $down = if ($btn -eq "right") { 0x0008 } else { 0x0002 }
  $up   = if ($btn -eq "right") { 0x0010 } else { 0x0004 }
  [V12TrayME.ME2]::mouse_event([uint32]$down, 0, 0, 0, [System.IntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [V12TrayME.ME2]::mouse_event([uint32]$up, 0, 0, 0, [System.IntPtr]::Zero)
}
function Open-TrayMenu {
  $icon = Find-OverflowIcon
  if (-not $icon) { throw "托盘溢出面板图标未找到" }
  $r = $icon.Current.BoundingRectangle
  Click-At ([int]($r.X + $r.Width / 2)) ([int]($r.Y + $r.Height / 2)) "right"
  Start-Sleep -Milliseconds 900
  if (-not (Menu-Visible)) { throw "右键后托盘菜单窗不可见" }
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
  $tmp = "$script:ShotTmp\_v12main.png"
  Seq "shot $tmp" | Out-Null; Start-Sleep -Milliseconds 200
  Composite $tmp $bg "$out\$name.png" 60
}
function Shot-Tray([string]$name, [string]$bg) {
  $tmp = "$script:ShotTmp\_v12tray.png"
  $env:CDP_MATCH = "tray-menu"
  try { Seq "shot $tmp" | Out-Null } finally { Remove-Item Env:CDP_MATCH }
  Start-Sleep -Milliseconds 200
  Composite $tmp $bg "$out\$name.png" 24
}
function Tray-Eval([string]$js) {
  $env:CDP_MATCH = "tray-menu"
  try { (Seq "eval (async()=>{ $js })()") | ConvertFrom-Json } finally { Remove-Item Env:CDP_MATCH }
}

function Main-Style {
  # cdp-seq eval 返回对象才是单层 JSON（字符串会被双重编码，ConvertFrom-Json 只剩裸串）
  $r = (Seq "eval (()=>{return {theme:document.documentElement.dataset.theme,material:document.documentElement.dataset.material,iw:innerWidth,ih:innerHeight}})()") | ConvertFrom-Json
  return $r
}

# --- 起始态校验：mini 收起、浅色、classic ---
$st = App-State
$st2 = Main-Style
Write-Output "start state: $($st.cls) menu=$($st.menu) $($st2.iw)x$($st2.ih) theme=$($st2.theme) material=$($st2.material)"

# --- 应用内右键菜单：4 组合（明暗 × 两材质），真实 contextmenu 派发定位 ---
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 1400
$combos = @(
  @("light", "classic", "v12-ctx-light-classic", "#E5E5EA"),
  @("light", "liquid_glass", "v12-ctx-light-liquid", "#E5E5EA"),
  @("dark", "classic", "v12-ctx-dark-classic", "#2C2C2E"),
  @("dark", "liquid_glass", "v12-ctx-dark-liquid", "#2C2C2E")
)
foreach ($c in $combos) {
  Seq ("eval (()=>{document.documentElement.dataset.theme='$($c[0])';document.documentElement.dataset.material='$($c[1])';return 'ok'})()") | Out-Null
  Start-Sleep -Milliseconds 300
  Seq "eval (()=>{const app=document.querySelector('.app');app.dispatchEvent(new MouseEvent('contextmenu',{clientX:170,clientY:200,bubbles:true,cancelable:true}));return 'open'})()" | Out-Null
  Start-Sleep -Milliseconds 500
  Shot-Main $c[2] $c[3]
  # 真实外点关菜单（pointerdown 捕获监听 → hideCtxMenu，aria 归位）
  Seq "eval document.dispatchEvent(new PointerEvent('pointerdown',{bubbles:true}));'closed'" | Out-Null
  Start-Sleep -Milliseconds 300
}
# 还原主窗浅色 classic + Esc 整收回 mini
Seq "eval (()=>{document.documentElement.dataset.theme='light';document.documentElement.dataset.material='classic';return 'ok'})()" | Out-Null
Esc-Collapse; Start-Sleep -Milliseconds 1200

# --- 托盘菜单：浅色（主窗 light/classic 经 query-ui-style 对表） ---
Open-TrayMenu
Shot-Tray "v12-tray-light" "#E5E5EA"
$ws = New-Object -ComObject WScript.Shell; $ws.SendKeys("{ESC}")
Start-Sleep -Milliseconds 500

# --- 托盘菜单：深色（真实 ui-style 广播链路） ---
Seq "eval (()=>{document.documentElement.dataset.theme='dark';window.__TAURI__.event.emit('ui-style',{theme:'dark',material:document.documentElement.dataset.material});return 'dark'})()" | Out-Null
Start-Sleep -Milliseconds 300
Open-TrayMenu
$t = Tray-Eval "return document.documentElement.dataset.theme"
Write-Output "tray page theme=$t"
Shot-Tray "v12-tray-dark" "#2C2C2E"
Tray-Eval "await window.__TAURI__.core.invoke('tray_menu_hide'); return 1" | Out-Null
Start-Sleep -Milliseconds 400
Seq "eval (()=>{document.documentElement.dataset.theme='light';window.__TAURI__.event.emit('ui-style',{theme:'light',material:document.documentElement.dataset.material});return 'light'})()" | Out-Null
Start-Sleep -Milliseconds 300

# --- 终态校验：mini、菜单关、浅色、classic、托盘菜单 hidden ---
$fin = App-State
$fin2 = Main-Style
$trayVisible = Menu-Visible
Write-Output "final state: $($fin.cls) menu=$($fin.menu) $($fin2.iw)x$($fin2.ih) theme=$($fin2.theme) material=$($fin2.material) trayVisible=$trayVisible"
if ($fin.cls -eq "app mini" -and -not $fin.menu -and $fin2.theme -eq "light" -and $fin2.material -eq "classic" -and -not $trayVisible) {
  Write-Output "V12-CAPTURE PASS (6 shots, state restored)"
} else {
  Write-Output "V12-CAPTURE WARN: 状态未完全还原，请检查"
}
