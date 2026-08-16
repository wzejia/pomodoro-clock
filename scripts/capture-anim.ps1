# 动画逐帧取证: pwsh -File scripts\capture-anim.ps1
# 产出 docs/review/anim/expand-fN.png(≥6帧) + sizes.txt(尺寸序列,验单调) + rapid-final.png(连点10次终态)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\win.ps1"
$out = "D:\VibeCoding\pomodoro-clock\docs\review\anim"
New-Item -ItemType Directory -Force $out | Out-Null

# 归位到干净背景并确保迷你态
Add-Type -Name SWP3 -Namespace CapW3 -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetWindowPos(System.IntPtr h, System.IntPtr after, int x, int y, int cx, int cy, uint flags);'
$h = Get-PomoHandle
[CapW3.SWP3]::SetWindowPos($h, [System.IntPtr]::Zero, 240, 160, 0, 0, 0x0001) | Out-Null
& bun "$PSScriptRoot\cdp.mjs" eval 'document.getElementById("app").classList.contains("expanded") && document.getElementById("btn-collapse").click(); "ok"' | Out-Null
Start-Sleep -Milliseconds 800

# --- 展开动画逐帧：先触发，立刻进入采样循环 ---
& bun "$PSScriptRoot\cdp.mjs" click 100 38 | Out-Null
$series = @()
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$i = 0
while ($sw.ElapsedMilliseconds -lt 700) {
  $r = Get-PomoRect
  $series += [PSCustomObject]@{ ms = $sw.ElapsedMilliseconds; w = ($r.Right - $r.Left); h = ($r.Bottom - $r.Top) }
  if ($i % 2 -eq 0) { Save-WindowShot "$out\expand-f$($i/2).png" -pad 40 | Out-Null }
  $i++
  Start-Sleep -Milliseconds 45
}
$series | ForEach-Object { "{0,4}ms  {1}x{2}" -f $_.ms, $_.w, $_.h } | Out-File -Encoding utf8 "$out\sizes.txt"
$series | ForEach-Object { "{0,4}ms  {1}x{2}" -f $_.ms, $_.w, $_.h }

# --- 快速连点 10 次（展开/收起交替）后查终态 ---
for ($k = 0; $k -lt 5; $k++) {
  & bun "$PSScriptRoot\cdp.mjs" click 100 38 | Out-Null
  Start-Sleep -Milliseconds 110
  & bun "$PSScriptRoot\cdp.mjs" eval 'document.getElementById("btn-collapse").click(); "c"' | Out-Null
  Start-Sleep -Milliseconds 110
}
Start-Sleep -Milliseconds 900
$r = Get-PomoRect
"after 10 rapid toggles: $($r.Right-$r.Left)x$($r.Bottom-$r.Top) (expect 220x76)"
Save-WindowShot "$out\rapid-final.png" -pad 40 | Out-Null

# 再展开查展开终态
& bun "$PSScriptRoot\cdp.mjs" click 100 38 | Out-Null
Start-Sleep -Milliseconds 900
$r = Get-PomoRect
"expanded final: $($r.Right-$r.Left)x$($r.Bottom-$r.Top) (expect 340x441)"
Save-WindowShot "$out\expanded-final.png" -pad 40 | Out-Null
