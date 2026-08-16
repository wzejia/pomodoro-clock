# 评审轮截图采集: powershell -File scripts\capture-round.ps1 -Round 1
# 产出 docs/review/round-N.png(面板浅色主图) + round-N-mini.png + round-N-dark.png + round-N-mini-dark.png
param([Parameter(Mandatory=$true)][int]$Round)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\win.ps1"
$cdp = { param($a) & bun "$PSScriptRoot\cdp.mjs" $a 2>$null }
$cdpe = { param($expr) & bun "$PSScriptRoot\cdp.mjs" eval $expr 2>$null }

$out = "D:\VibeCoding\pomodoro-clock\docs\review"
New-Item -ItemType Directory -Force $out | Out-Null

# 窗口挪到干净位置,启动计时让画面是运行态
Add-Type -Name SWP2 -Namespace CapW -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetWindowPos(System.IntPtr h, System.IntPtr after, int x, int y, int cx, int cy, uint flags);'
$h = Get-PomoHandle
[CapW.SWP2]::SetWindowPos($h, [System.IntPtr]::Zero, 240, 160, 0, 0, 0x0001) | Out-Null
& bun "$PSScriptRoot\cdp.mjs" eval 'document.documentElement.dataset.theme="light"; window.__TAURI__.core.invoke("timer_start").then(()=>"ok")' | Out-Null
Start-Sleep -Seconds 3

# 1) 迷你态 浅色(运行中)
Save-WindowShot "$out\round-$Round-mini.png" -pad 60

# 2) 展开 浅色(运行中+统计)
& bun "$PSScriptRoot\cdp.mjs" click 100 38 | Out-Null
Start-Sleep -Milliseconds 1400
Save-WindowShot "$out\round-$Round.png" -pad 60

# 3) 展开 深色
& bun "$PSScriptRoot\cdp.mjs" eval 'document.documentElement.dataset.theme="dark"; document.querySelectorAll(".theme-opt").forEach(b=>b.classList.toggle("active",b.dataset.themeOpt==="dark")); "dark"' | Out-Null
Start-Sleep -Milliseconds 600
Save-WindowShot "$out\round-$Round-dark.png" -pad 60

# 4) 迷你态 深色
& bun "$PSScriptRoot\cdp.mjs" eval 'document.getElementById("btn-collapse").getBoundingClientRect().toJSON()' | Out-Null
Start-Sleep -Milliseconds 200
& bun "$PSScriptRoot\cdp.mjs" eval 'document.getElementById("btn-collapse").click(); "collapsed"' | Out-Null
Start-Sleep -Milliseconds 1200
Save-WindowShot "$out\round-$Round-mini-dark.png" -pad 60

# 回到浅色,供下一轮起点
& bun "$PSScriptRoot\cdp.mjs" eval 'document.documentElement.dataset.theme="light"; document.querySelectorAll(".theme-opt").forEach(b=>b.classList.toggle("active",b.dataset.themeOpt==="light")); "light"' | Out-Null
Write-Output "round-$Round captured: mini/panel/dark/mini-dark"
