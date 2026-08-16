# 任务2/3验收：真实右键菜单截图（桌面背景入镜）+ 三键纯文字 + 抽屉流程
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\win.ps1"
$out = "D:\VibeCoding\pomodoro-clock\docs\review\ctx"
New-Item -ItemType Directory -Force $out | Out-Null

Add-Type -Namespace CapC -Name IN -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, System.IntPtr e); [DllImport("user32.dll")] public static extern bool SetWindowPos(System.IntPtr h, System.IntPtr after, int x, int y, int cx, int cy, uint flags);'

$h = Get-PomoHandle
[CapC.IN]::SetWindowPos($h, [System.IntPtr]::Zero, 240, 160, 0, 0, 0x0001) | Out-Null
& bun "$PSScriptRoot\cdp.mjs" eval 'document.documentElement.dataset.theme="light"; document.getElementById("app").classList.contains("mini")?"mini":"exp"' | Out-Null
Start-Sleep -Milliseconds 400

function Right-Click-At([int]$sx, [int]$sy) {
  [CapC.IN]::SetCursorPos($sx, $sy) | Out-Null
  Start-Sleep -Milliseconds 120
  [CapC.IN]::mouse_event(0x0008, 0, 0, 0, [System.IntPtr]::Zero)  # RIGHTDOWN
  Start-Sleep -Milliseconds 60
  [CapC.IN]::mouse_event(0x0010, 0, 0, 0, [System.IntPtr]::Zero)  # RIGHTUP
}

# 1) 迷你态真实右键 → 菜单
$r = Get-PomoRect
Right-Click-At ($r.Left + 110) ($r.Top + 38)
Start-Sleep -Milliseconds 500
Save-WindowShot "$out\ctx-mini-menu.png" -pad 80 | Out-Null

# 2) 点「设置…」→ 自动展开 + 抽屉滑入（点菜单项屏幕坐标）
& bun "$PSScriptRoot\cdp.mjs" eval 'JSON.stringify(document.getElementById("ctx-settings").getBoundingClientRect().toJSON())'
$mi = & bun "$PSScriptRoot\cdp.mjs" eval 'JSON.stringify(document.getElementById("ctx-settings").getBoundingClientRect().toJSON())' 2>$null | ConvertFrom-Json
$r2 = Get-PomoRect
[CapC.IN]::SetCursorPos(($r2.Left + [int]($mi.x + $mi.width/2)), ($r2.Top + [int]($mi.y + $mi.height/2))) | Out-Null
Start-Sleep -Milliseconds 120
[CapC.IN]::mouse_event(0x0002, 0, 0, 0, [System.IntPtr]::Zero)  # LEFTDOWN
Start-Sleep -Milliseconds 60
[CapC.IN]::mouse_event(0x0004, 0, 0, 0, [System.IntPtr]::Zero)  # LEFTUP
Start-Sleep -Milliseconds 1200
Save-WindowShot "$out\ctx-to-drawer.png" -pad 60 | Out-Null

# 3) 抽屉返回，展开态右键 → 菜单
& bun "$PSScriptRoot\cdp.mjs" eval 'document.getElementById("btn-drawer-back").click(); "back"' | Out-Null
Start-Sleep -Milliseconds 600
$r3 = Get-PomoRect
Right-Click-At ($r3.Left + 170) ($r3.Top + 220)
Start-Sleep -Milliseconds 500
Save-WindowShot "$out\ctx-panel-menu.png" -pad 60 | Out-Null
[CapC.IN]::SetCursorPos(($r3.Left + 60), ($r3.Top + 400)) | Out-Null
[CapC.IN]::mouse_event(0x0002, 0, 0, 0, [System.IntPtr]::Zero)
[CapC.IN]::mouse_event(0x0004, 0, 0, 0, [System.IntPtr]::Zero)
Start-Sleep -Milliseconds 400

# 4) 三键纯文字特写（展开面板按钮行）
Save-WindowShot "$out\buttons-plain.png" -pad 60 | Out-Null
Write-Output "ctx captures done"
