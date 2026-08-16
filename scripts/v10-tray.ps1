# v10 托盘验收：UIA 定位托盘图标 → 图标截图 / 左键聚焦 / 右键菜单截图（动态文案）/ 点退出验进程消失
# 用法：pwsh scripts\v10-tray.ps1 -Step icon|focus|menu|quit
# 纪律：menu 步骤不调退出；quit 放最后（exe 死后需重启 dev）
param([Parameter(Mandatory=$true)][ValidateSet("icon","focus","menu","menurun","quit","quitkeys")][string]$Step)
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing

$out = "D:\VibeCoding\pomodoro-clock\docs\screenshots\v10"
New-Item -ItemType Directory -Force $out | Out-Null
$AE = [System.Windows.Automation.AutomationElement]

function Find-OverflowIcon {
  # 只认溢出面板里的托盘图标（宽 ≤60；任务栏按钮 84x48 同名须排除）。
  # ^ 是 toggle：找不到小图标就点 ^ 再查，循环 3 轮覆盖「面板已开被点关」的情形
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
function Open-Overflow {
  # 先点开 ^「显示隐藏的图标」溢出面板，面板里的托盘图标才点得到
  $cond = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
    [System.Windows.Automation.ControlType]::Button)
  $all = $AE::RootElement.FindAll([System.Windows.Automation.TreeScope]::Subtree, $cond)
  foreach ($b in $all) {
    try {
      if ($b.Current.Name -match "隐藏的图标|Show hidden icons") {
        $r = $b.Current.BoundingRectangle
        Write-Output "chevron at ($([int]($r.X + $r.Width / 2)),$([int]($r.Y + $r.Height / 2))) clicked"
        Click-At ([int]($r.X + $r.Width / 2)) ([int]($r.Y + $r.Height / 2)) "left"
        Start-Sleep -Milliseconds 700
        return
      }
    } catch {}
  }
}
function Shot-Rect([int]$x, [int]$y, [int]$w, [int]$h, [string]$dest) {
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
  $bmp.Save($dest, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  Write-Output "saved: $dest"
}
Add-Type -Name ME -Namespace V10Tray -MemberDefinition '[DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint data, System.IntPtr extra);'
function Click-At([int]$x, [int]$y, [string]$btn) {
  # SetCursorPos 定位 + mouse_event 按钮（本机 SendInput 键盘/鼠标注入均被拦——长效教训）
  [PomoWin.U32]::SetCursorPos($x, $y) | Out-Null
  Start-Sleep -Milliseconds 150
  $down = if ($btn -eq "right") { 0x0008 } else { 0x0002 }   # RIGHTDOWN / LEFTDOWN
  $up   = if ($btn -eq "right") { 0x0010 } else { 0x0004 }   # RIGHTUP / LEFTUP
  [V10Tray.ME]::mouse_event([uint32]$down, 0, 0, 0, [System.IntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [V10Tray.ME]::mouse_event([uint32]$up, 0, 0, 0, [System.IntPtr]::Zero)
}

$icon = Find-OverflowIcon
if (-not $icon) { Write-Error "托盘溢出面板图标未找到（UIA 搜不到 ≤60 宽的『番茄钟』按钮）"; exit 1 }
$r = $icon.Current.BoundingRectangle
$cx = [int]($r.X + $r.Width / 2); $cy = [int]($r.Y + $r.Height / 2)
Write-Output "tray icon at ($cx,$cy) size $($r.Width)x$($r.Height)"

switch ($Step) {
  "icon" {
    Shot-Rect ($cx - 40) ($cy - 24) 96 48 "$out\tray-icon.png"
  }
  "focus" {
    # 先把前台让给本终端（模拟「窗口不在前台」），再左键托盘图标
    $term = [V7Win.V7]::GetForegroundWindow()
    Write-Output "before: fg=$term ours=$script:PomoHwnd"
    Click-At $cx $cy "left"
    Start-Sleep -Milliseconds 600
    $fg = [V7Win.V7]::GetForegroundWindow()
    Write-Output "after: fg=$fg ours=$script:PomoHwnd"
    if ($fg -eq $script:PomoHwnd) { Write-Output "FOCUS PASS" } else { Write-Output "FOCUS FAIL" }
  }
  "menu" {
    # 当前计时 running → 菜单首项应为「暂停」（初始文案「开始」，见即动态生效）
    Click-At $cx $cy "right"
    Start-Sleep -Milliseconds 700
    # 菜单弹在图标上方：截图标上方 260x140 区域
    Shot-Rect ($cx - 200) ($cy - 150) 260 140 "$out\tray-menu.png"
    # Esc 关菜单（SendKeys 通道，注入纪律）
    $ws = New-Object -ComObject WScript.Shell; $ws.SendKeys("{ESC}")
    Start-Sleep -Milliseconds 300
  }
  "menurun" {
    # running 态补拍：菜单首项应为「暂停」；只截图不动键盘（先验证再决定下一步）
    Click-At $cx $cy "right"
    Start-Sleep -Milliseconds 700
    Shot-Rect ($cx + 20) ($cy - 170) 230 170 "$out\tray-menu-running.png"
    $ws = New-Object -ComObject WScript.Shell; $ws.SendKeys("{ESC}")
    Start-Sleep -Milliseconds 300
  }
  "quitkeys" {
    # UIA 枚举不到 Tauri 托盘菜单项 → 键盘赛道：{UP} 从首项环绕到末项「退出」+{ENTER}
    # 安全依据：Find-OverflowIcon 确证面板开着且右键点必中 40x40 图标 → 弹出的必是我们的三项菜单；
    # 按键前先截图存证（tray-menu-before-quit.png），{DOWN}{ENTER}=「开始」此前已实证过同一链路
    Click-At $cx $cy "right"
    Start-Sleep -Milliseconds 700
    Shot-Rect ($cx + 20) ($cy - 170) 230 170 "$out\tray-menu-before-quit.png"
    $ws = New-Object -ComObject WScript.Shell
    $ws.SendKeys("{UP}")
    Start-Sleep -Milliseconds 250
    $ws.SendKeys("{ENTER}")
    Start-Sleep -Seconds 2
    $alive = Get-Process pomodoro-clock -ErrorAction SilentlyContinue
    if ($alive) { Write-Output "QUIT FAIL: process still alive" } else { Write-Output "QUIT PASS: no pomodoro-clock process" }
  }
  "quit" {
    Click-At $cx $cy "right"
    Start-Sleep -Milliseconds 700
    # UIA 找弹出菜单（class #32768）里的「退出」项
    Start-Sleep -Milliseconds 300
    $menuCond = New-Object System.Windows.Automation.PropertyCondition(
      [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
      [System.Windows.Automation.ControlType]::MenuItem)
    $items = $AE::RootElement.FindAll([System.Windows.Automation.TreeScope]::Subtree, $menuCond)
    $quit = $null
    foreach ($it in $items) { try { if ($it.Current.Name -eq "退出") { $quit = $it; break } } catch {} }
    if (-not $quit) { Write-Output "QUIT ITEM NOT FOUND"; $ws = New-Object -ComObject WScript.Shell; $ws.SendKeys("{ESC}"); exit 1 }
    $qr = $quit.Current.BoundingRectangle
    Shot-Rect ($cx - 200) ($cy - 150) 260 140 "$out\tray-menu-before-quit.png"
    Click-At ([int]($qr.X + $qr.Width / 2)) ([int]($qr.Y + $qr.Height / 2)) "left"
    Start-Sleep -Seconds 2
    $alive = Get-Process pomodoro-clock -ErrorAction SilentlyContinue
    if ($alive) { Write-Output "QUIT FAIL: process still alive" } else { Write-Output "QUIT PASS: no pomodoro-clock process" }
  }
}
