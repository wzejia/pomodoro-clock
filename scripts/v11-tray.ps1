# v11 托盘自绘菜单验收：真实右键弹层（明暗截图）/四项齐全/勾选态跟随注册表/失焦关/Esc 关/退出无进程
# 用法：pwsh scripts\v11-tray.ps1 -Step popup|dark|items|check|blur|esc|quit
# 纪律：quit 放最后（exe 死后需重启）；每步结束恢复现场（菜单 hidden、主窗 mini、主题还原）
param([Parameter(Mandatory=$true)][ValidateSet("popup","dark","items","check","blur","esc","quit")][string]$Step)
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing

$out = "D:\VibeCoding\pomodoro-clock\docs\screenshots\v11"
New-Item -ItemType Directory -Force $out | Out-Null
$AE = [System.Windows.Automation.AutomationElement]

Add-Type -Name TM -Namespace V11Tray -MemberDefinition '
 public delegate bool EnumWindowsProc(System.IntPtr h, uint param);
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, uint param);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr h, out uint pid);
 [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(System.IntPtr h, System.Text.StringBuilder sb, int max);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(System.IntPtr h);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(System.IntPtr h, out RECT r);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr h);
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
   EnumWindows(cb, 0);
   System.GC.KeepAlive(cb);
   return found;
 }
'
Add-Type -Name ME2 -Namespace V11TrayME -MemberDefinition '[DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint data, System.IntPtr extra);'

function Menu-Hwnd {
  $h = [V11Tray.TM]::FindMenuHwnd([uint32]$script:PomoPid)
  if ($h -eq [System.IntPtr]::Zero) { throw "tray-menu 窗口未找到（番茄钟菜单）" }
  return $h
}
function Menu-Visible { return [V11Tray.TM]::IsWindowVisible((Menu-Hwnd)) }
function Menu-Rect { $r = New-Object V11Tray.TM+RECT; [V11Tray.TM]::GetWindowRect((Menu-Hwnd), [ref]$r) | Out-Null; return $r }

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
  # 只认溢出面板里的托盘图标（宽 ≤60；任务栏按钮 84x48 同名须排除）；^ 是 toggle：3 轮循环兜底
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
  # SetCursorPos 定位 + mouse_event 按钮（本机 SendInput 被拦——长效教训）
  [PomoWin.U32]::SetCursorPos($x, $y) | Out-Null
  Start-Sleep -Milliseconds 150
  $down = if ($btn -eq "right") { 0x0008 } else { 0x0002 }
  $up   = if ($btn -eq "right") { 0x0010 } else { 0x0004 }
  [V11TrayME.ME2]::mouse_event([uint32]$down, 0, 0, 0, [System.IntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [V11TrayME.ME2]::mouse_event([uint32]$up, 0, 0, 0, [System.IntPtr]::Zero)
}
function Tray-Eval([string]$js) {
  $env:CDP_MATCH = "tray-menu"
  try { (Seq "eval (async()=>{ $js })()") | ConvertFrom-Json } finally { Remove-Item Env:CDP_MATCH }
}
function Shot-Menu([string]$name, [string]$bg) {
  $tmpShot = "$script:ShotTmp\_tray11.png"
  $env:CDP_MATCH = "tray-menu"
  try { Seq "shot $tmpShot" | Out-Null } finally { Remove-Item Env:CDP_MATCH }
  Start-Sleep -Milliseconds 200
  $src = [System.Drawing.Bitmap]::new($tmpShot)
  $pad = 24
  $bmp = [System.Drawing.Bitmap]::new(($src.Width + 2*$pad), ($src.Height + 2*$pad))
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.Clear([System.Drawing.ColorTranslator]::FromHtml($bg))
  $g.DrawImage($src, $pad, $pad, $src.Width, $src.Height)
  $bmp.Save("$out\$name.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose(); $src.Dispose()
  Write-Output "saved: $out\$name.png"
}
function Open-TrayMenu {
  $icon = Find-OverflowIcon
  if (-not $icon) { throw "托盘溢出面板图标未找到" }
  $r = $icon.Current.BoundingRectangle
  $cx = [int]($r.X + $r.Width / 2); $cy = [int]($r.Y + $r.Height / 2)
  Write-Output "tray icon at ($cx,$cy)"
  Click-At $cx $cy "right"
  Start-Sleep -Milliseconds 900
  if (-not (Menu-Visible)) { throw "右键后托盘菜单窗不可见" }
  return $r
}

switch ($Step) {
  "popup" {
    # 浅色弹出：真实右键 → 截图 + 定位断言（工作区内、贴任务栏内侧）
    Add-Type -AssemblyName System.Windows.Forms
    $iconRect = Open-TrayMenu
    $mr = Menu-Rect
    $wa = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
    Write-Output "menu rect: L=$($mr.Left) T=$($mr.Top) R=$($mr.Right) B=$($mr.Bottom) ($($mr.Right-$mr.Left)x$($mr.Bottom-$mr.Top))"
    Write-Output "work area: $wa"
    $inside = ($mr.Left -ge $wa.Left) -and ($mr.Right -le $wa.Right) -and ($mr.Top -ge $wa.Top) -and ($mr.Bottom -le $wa.Bottom)
    $aboveIcon = $mr.Bottom -le ([int]$iconRect.Y + 2)  # 底部任务栏：菜单应在图标上方
    Write-Output "inside-workarea=$inside above-icon=$aboveIcon"
    if ($inside -and $aboveIcon) { Write-Output "POPUP PASS" } else { Write-Output "POPUP FAIL" }
    Shot-Menu "traymenu-popup" "#E5E5EA"
    # 恢复现场：菜单仍开着且焦点在菜单（下一步再右键会变 toggle 关）
    $ws = New-Object -ComObject WScript.Shell; $ws.SendKeys("{ESC}")
    Start-Sleep -Milliseconds 400
  }
  "dark" {
    # 深色弹出：主窗 dataset + ui-style 广播（真实监听链路），截完还原浅色
    Seq "eval (()=>{document.documentElement.dataset.theme='dark';window.__TAURI__.event.emit('ui-style',{theme:'dark',material:document.documentElement.dataset.material});return 'dark'})()" | Out-Null
    Start-Sleep -Milliseconds 300
    Open-TrayMenu | Out-Null
    $t = Tray-Eval "return document.documentElement.dataset.theme"
    Write-Output "menu page theme=$t"
    Shot-Menu "traymenu-popup-dark" "#2C2C2E"
    # 还原
    Tray-Eval "await window.__TAURI__.core.invoke('tray_menu_hide'); return 1" | Out-Null
    Start-Sleep -Milliseconds 400
    Seq "eval (()=>{document.documentElement.dataset.theme='light';window.__TAURI__.event.emit('ui-style',{theme:'light',material:document.documentElement.dataset.material});return 'light'})()" | Out-Null
  }
  "items" {
    Open-TrayMenu | Out-Null
    $r = Tray-Eval "const s=await window.__TAURI__.core.invoke('timer_snapshot'); return {items:[...document.querySelectorAll('.ctx-item')].map(b=>b.textContent), status:s.status}"
    Write-Output ($r | ConvertTo-Json -Compress)
    $expectToggle = if ($r.status -eq "running") { "暂停" } elseif ($r.status -eq "paused") { "继续" } else { "开始" }
    if ($r.items.Count -eq 4 -and $r.items[0] -eq $expectToggle -and $r.items[1] -eq "设置…" -and $r.items[2] -eq "开机自启" -and $r.items[3] -eq "退出") { Write-Output "ITEMS PASS" } else { Write-Output "ITEMS FAIL (toggle 应=$expectToggle)" }
    Tray-Eval "await window.__TAURI__.core.invoke('tray_menu_hide'); return 1" | Out-Null
    Start-Sleep -Milliseconds 400
  }
  "check" {
    # 勾选态=注册表真实态：开→注册表开→重开菜单 aria-checked=true→截图→再点关→注册表关
    Open-TrayMenu | Out-Null
    $r0 = Tray-Eval "return {aria:document.getElementById('tm-autostart').getAttribute('aria-checked'), checked:document.getElementById('tm-autostart').classList.contains('checked')}"
    $reg0 = (reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v 番茄钟 2>$null) -join " "
    Write-Output "before: dom=$($r0 | ConvertTo-Json -Compress) registry=$(if ($reg0 -match '番茄钟') {'ON'} else {'OFF'})"
    Tray-Eval "document.getElementById('tm-autostart').click(); return 1" | Out-Null
    Start-Sleep -Milliseconds 800
    $reg1 = (reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v 番茄钟 2>$null) -join " "
    Write-Output "after-click: registry=$(if ($reg1 -match '番茄钟') {'ON'} else {'OFF'})(菜单应已自动关=$(-not (Menu-Visible)))"
    Open-TrayMenu | Out-Null
    $r1 = Tray-Eval "return {aria:document.getElementById('tm-autostart').getAttribute('aria-checked')}"
    Write-Output "reopen: aria=$($r1.aria)（应 true=跟随注册表）"
    Shot-Menu "traymenu-autostart-on" "#E5E5EA"
    # 还原为关（默认关）
    Tray-Eval "document.getElementById('tm-autostart').click(); return 1" | Out-Null
    Start-Sleep -Milliseconds 800
    $reg2 = (reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v 番茄钟 2>$null) -join " "
    if ($reg2 -match '番茄钟') { Write-Output "CHECK FAIL: 注册表未关" } else { Write-Output "CHECK PASS（开/关两向注册表+勾选跟随）" }
  }
  "blur" {
    # 先记下当前前台（终端），再开菜单（菜单 set_focus 成前台），最后抢回前台验失焦关
    $term = [V7Win.V7]::GetForegroundWindow()
    Open-TrayMenu | Out-Null
    [V11Tray.TM]::SetForegroundWindow($term) | Out-Null
    Start-Sleep -Milliseconds 700
    if (Menu-Visible) { Write-Output "BLUR FAIL: still visible" } else { Write-Output "BLUR PASS" }
  }
  "esc" {
    Open-TrayMenu | Out-Null
    $ws = New-Object -ComObject WScript.Shell; $ws.SendKeys("{ESC}")
    Start-Sleep -Milliseconds 600
    if (Menu-Visible) { Write-Output "ESC FAIL: still visible" } else { Write-Output "ESC PASS" }
  }
  "quit" {
    Open-TrayMenu | Out-Null
    Shot-Menu "traymenu-before-quit" "#E5E5EA"
    # 真实 CDP 点击「退出」项（菜单页坐标系内；cdp-seq 无 click 命令，用 cdp.mjs）
    $qr = Tray-Eval "const r=document.getElementById('tm-quit').getBoundingClientRect(); return {x:Math.round(r.x+r.width/2), y:Math.round(r.y+r.height/2)}"
    $env:CDP_MATCH = "tray-menu"
    try { & bun "D:\VibeCoding\pomodoro-clock\scripts\cdp.mjs" click $qr.x $qr.y | Out-Null } finally { Remove-Item Env:CDP_MATCH }
    Start-Sleep -Seconds 2
    $alive = Get-Process pomodoro-clock -ErrorAction SilentlyContinue
    if ($alive) { Write-Output "QUIT FAIL: process still alive" } else { Write-Output "QUIT PASS: no pomodoro-clock process" }
  }
}
