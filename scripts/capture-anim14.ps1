# 任务0复现证据：展开/收起逐帧抓屏 + 连点10次终态
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\win.ps1"
$out = "D:\VibeCoding\pomodoro-clock\docs\review\anim14"
New-Item -ItemType Directory -Force $out | Out-Null
$cdpe = { param($expr) & bun "$PSScriptRoot\cdp.mjs" eval $expr 2>$null }

# 挪到干净位置
Add-Type -Name SWP3 -Namespace CapW3 -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetWindowPos(System.IntPtr h, System.IntPtr after, int x, int y, int cx, int cy, uint flags);'
$h = Get-PomoHandle
[CapW3.SWP3]::SetWindowPos($h, [System.IntPtr]::Zero, 240, 160, 0, 0, 0x0001) | Out-Null

function Grab($prefix, [int]$n, [int]$ms) {
  for ($i = 0; $i -lt $n; $i++) {
    Save-WindowShot "$out\$prefix-f$('{0:d2}' -f $i).png" -pad 60 | Out-Null
    Start-Sleep -Milliseconds $ms
  }
}

# 确保当前迷你 → 触发展开，抓帧
& bun "$PSScriptRoot\cdp.mjs" eval 'document.documentElement.dataset.theme="light"; document.getElementById("app").classList.contains("mini") ? "mini" : "expanded"' | Out-Null
& bun "$PSScriptRoot\cdp.mjs" click 100 38 | Out-Null
Grab "expand" 12 30
Start-Sleep -Milliseconds 800

# 触发收起，抓帧（重点）
& bun "$PSScriptRoot\cdp.mjs" eval 'document.getElementById("btn-collapse").click(); "c"' | Out-Null
Grab "collapse" 12 30
Start-Sleep -Milliseconds 800

# 连点 10 次（展开/收起交替，间隔 120ms < 动画时长 340ms，考验打断）
for ($k = 0; $k -lt 10; $k++) {
  if ($k % 2 -eq 0) { & bun "$PSScriptRoot\cdp.mjs" click 100 38 | Out-Null }
  else { & bun "$PSScriptRoot\cdp.mjs" eval 'document.getElementById("btn-collapse").click(); "c"' | Out-Null }
  Start-Sleep -Milliseconds 120
}
Start-Sleep -Milliseconds 1000
# 终态应为迷你（偶数次展开、奇数次收起，10 次后收起）
& bun "$PSScriptRoot\cdp.mjs" eval 'const r = document.getElementById("app").getBoundingClientRect(); document.getElementById("app").className + " " + Math.round(r.width) + "x" + Math.round(r.height)'
Save-WindowShot "$out\rapid10-final.png" -pad 60 | Out-Null
Write-Output "anim14 frames done"
