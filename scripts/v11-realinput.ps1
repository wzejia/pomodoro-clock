# v11 托盘菜单真实输入验收（修复子智能体遗留疑问的实证）：
# 真实 OS 级输入（mouse_event 相对微移 + 点击）能否驱动菜单页 :hover 与项点击？
# 路径：UIA 右键弹层 → SetCursorPos 到「设置…」+ mouse_event 相对 ±1px 微移 → 查 :hover
#      → mouse_event 左键点「设置…」→ 主窗应展开+开抽屉（open-settings 链路）、菜单自关
# 判定：hover 真 + 主窗展开开抽屉 = 真实输入可达；点穿桌面（菜单失焦自关、主窗不动）= 不可达
# 用法：pwsh scripts\v11-realinput.ps1
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$AE = [System.Windows.Automation.AutomationElement]

Add-Type -Name RI -Namespace V11RI -MemberDefinition '
 public delegate bool EnumWindowsProc(System.IntPtr h, uint param);
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, uint param);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr h, out uint pid);
 [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(System.IntPtr h, System.Text.StringBuilder sb, int max);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(System.IntPtr h);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(System.IntPtr h, out RECT r);
 [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint data, System.IntPtr extra);
 public struct RECT { public int Left, Top, Right, Bottom; }
 public static System.IntPtr FindMenuHwnd(uint pid) {
   System.IntPtr found = System.IntPtr.Zero;
   EnumWindowsProc cb = (h, p) => {
     uint wp; GetWindowThreadProcessId(h, out wp);
     if (wp != pid) return true;
     var sb = new System.Text.StringBuilder(64); GetWindowTextW(h, sb, 64);
     if (sb.ToString() == "番茄钟菜单") { found = h; return false; }
     return true;
   };
   EnumWindows(cb, 0); System.GC.KeepAlive(cb);
   return found;
 }
'

function Click-At([int]$x, [int]$y, [string]$btn) {
  [PomoWin.U32]::SetCursorPos($x, $y) | Out-Null
  Start-Sleep -Milliseconds 150
  $down = if ($btn -eq "right") { 0x0008 } else { 0x0002 }
  $up   = if ($btn -eq "right") { 0x0010 } else { 0x0004 }
  [V11RI.RI]::mouse_event([uint32]$down, 0, 0, 0, [System.IntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [V11RI.RI]::mouse_event([uint32]$up, 0, 0, 0, [System.IntPtr]::Zero)
}
function Open-Overflow {
  $cond = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button)
  foreach ($b in $AE::RootElement.FindAll([System.Windows.Automation.TreeScope]::Subtree, $cond)) {
    try {
      if ($b.Current.Name -match "隐藏的图标|Show hidden icons") {
        $r = $b.Current.BoundingRectangle
        Click-At ([int]($r.X + $r.Width / 2)) ([int]($r.Y + $r.Height / 2)) "left"
        Start-Sleep -Milliseconds 700; return
      }
    } catch {}
  }
}
function Find-OverflowIcon {
  for ($i = 0; $i -lt 3; $i++) {
    $cond = New-Object System.Windows.Automation.PropertyCondition(
      [System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button)
    foreach ($b in $AE::RootElement.FindAll([System.Windows.Automation.TreeScope]::Subtree, $cond)) {
      try {
        if ($b.Current.Name -like "*番茄钟*" -and $b.Current.BoundingRectangle.Width -le 60) { return $b }
      } catch {}
    }
    Open-Overflow
  }
  return $null
}
function Tray-Eval([string]$js) {
  $env:CDP_MATCH = "tray-menu"
  try { (Seq "eval (async()=>{ $js })()") | ConvertFrom-Json } finally { Remove-Item Env:CDP_MATCH }
}

# 1) 右键弹层
$icon = Find-OverflowIcon
if (-not $icon) { throw "托盘图标未找到" }
$ir = $icon.Current.BoundingRectangle
Click-At ([int]($ir.X + $ir.Width / 2)) ([int]($ir.Y + $ir.Height / 2)) "right"
Start-Sleep -Milliseconds 900
$mh = [V11RI.RI]::FindMenuHwnd([uint32]$script:PomoPid)
if ($mh -eq [System.IntPtr]::Zero -or -not [V11RI.RI]::IsWindowVisible($mh)) { throw "菜单未弹出" }
$mr = New-Object V11RI.RI+RECT
[V11RI.RI]::GetWindowRect($mh, [ref]$mr) | Out-Null
Write-Output "menu at ($($mr.Left),$($mr.Top))-($($mr.Right),$($mr.Bottom))"

# 2) 「设置…」项的屏幕坐标（页内 rect + 窗口客户区原点）
$item = Tray-Eval "const r=document.getElementById('tm-settings').getBoundingClientRect(); return {x:r.x,y:r.y,w:r.width,h:r.height}"
$sx = $mr.Left + [int]($item.x + $item.w / 2); $sy = $mr.Top + [int]($item.y + $item.h / 2)
Write-Output "tm-settings screen=($sx,$sy)"

# 3) SetCursorPos 落点 + mouse_event 相对微移 ±1px 造真实 WM_MOUSEMOVE（SetCursorPos 本身不产生 hover）
[PomoWin.U32]::SetCursorPos($sx, $sy) | Out-Null
Start-Sleep -Milliseconds 200
[V11RI.RI]::mouse_event(0x0001, 1, 0, 0, [System.IntPtr]::Zero)
Start-Sleep -Milliseconds 120
[V11RI.RI]::mouse_event(0x0001, -1, 0, 0, [System.IntPtr]::Zero)
Start-Sleep -Milliseconds 400
$hover = Tray-Eval "return {hover:document.getElementById('tm-settings').matches(':hover'), anyHover:[...document.querySelectorAll('.ctx-item')].map(b=>b.matches(':hover'))}"
Write-Output "hover state: $($hover | ConvertTo-Json -Compress)"
$hoverOk = $hover.hover -eq $true

# 4) 真实点击「设置…」（左键 down/up，v10 实证通道）
[V11RI.RI]::mouse_event(0x0002, 0, 0, 0, [System.IntPtr]::Zero)
Start-Sleep -Milliseconds 60
[V11RI.RI]::mouse_event(0x0004, 0, 0, 0, [System.IntPtr]::Zero)
Start-Sleep -Milliseconds 1200
$main = Seq "eval (()=>{const a=document.getElementById('app');return {cls:a.className, expanded:a.classList.contains('expanded'), drawer:a.classList.contains('drawer-open')}})()" | ConvertFrom-Json
$menuGone = -not [V11RI.RI]::IsWindowVisible($mh)
Write-Output "after click: main=$($main | ConvertTo-Json -Compress) menuGone=$menuGone"
$clickOk = $main.expanded -and $main.drawer -and $menuGone

# 5) 恢复现场：主窗整收回 mini（Esc 链路）
Seq "esc" | Out-Null
Start-Sleep -Milliseconds 1200
$final = Seq "state"
Write-Output "restored: $final"
[PomoWin.U32]::SetCursorPos(1, 1) | Out-Null

Write-Output $(if ($hoverOk -and $clickOk) { "REAL-INPUT PASS（hover 真 + 点击开设置）" } else { "REAL-INPUT FAIL: hover=$hoverOk click=$clickOk" })
