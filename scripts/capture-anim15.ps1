# 任务1验收：新 morph 动画逐帧证据（卡片尺寸 rAF 采样 + PNG 帧）+ 连点 10 次 + 双主题
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\win.ps1"
$out = "D:\VibeCoding\pomodoro-clock\docs\review\anim15"
New-Item -ItemType Directory -Force $out | Out-Null

Add-Type -Name SWP4 -Namespace CapW4 -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetWindowPos(System.IntPtr h, System.IntPtr after, int x, int y, int cx, int cy, uint flags);'
$h = Get-PomoHandle
[CapW4.SWP4]::SetWindowPos($h, [System.IntPtr]::Zero, 240, 160, 0, 0, 0x0001) | Out-Null

function Grab($prefix, [int]$n, [int]$ms) {
  for ($i = 0; $i -lt $n; $i++) {
    Save-WindowShot "$out\$prefix-f$('{0:d2}' -f $i).png" -pad 60 | Out-Null
    Start-Sleep -Milliseconds $ms
  }
}

& bun "$PSScriptRoot\cdp.mjs" eval 'document.documentElement.dataset.theme="light"; "light"' | Out-Null

$doExpand = 'const p=document.getElementById("pill");p.dispatchEvent(new MouseEvent("mousedown",{bubbles:true}));p.dispatchEvent(new MouseEvent("mouseup",{bubbles:true}));"e"'
$doCollapse = 'document.getElementById("btn-collapse").click(); "c"'
$sampleJs = '(async()=>{const app=document.getElementById("app");return await new Promise((resolve)=>{const rows=[];const t0=performance.now();ACT const sample=()=>{const r=app.getBoundingClientRect();rows.push([Math.round(performance.now()-t0),Math.round(r.width*10)/10,Math.round(r.height*10)/10]);if(performance.now()-t0<700)requestAnimationFrame(sample);else resolve(rows.map(x=>x.join(":")).join(" "))};requestAnimationFrame(sample)})})()'

# ---- 展开：PNG 帧（动画进行中） ----
& bun "$PSScriptRoot\cdp.mjs" eval $doExpand | Out-Null
Grab "expand" 8 30
Start-Sleep -Milliseconds 700
# ---- 收起：PNG 帧 ----
& bun "$PSScriptRoot\cdp.mjs" eval $doCollapse | Out-Null
Grab "collapse" 8 30
Start-Sleep -Milliseconds 700
# ---- 展开#2：rAF 采样 ----
$expRows = & bun "$PSScriptRoot\cdp.mjs" eval ($sampleJs -replace 'ACT', $doExpand) 2>$null
Start-Sleep -Milliseconds 500
# ---- 收起#2：rAF 采样 ----
$colRows = & bun "$PSScriptRoot\cdp.mjs" eval ($sampleJs -replace 'ACT', $doCollapse) 2>$null
Start-Sleep -Milliseconds 500

"=== expand card rect (ms:w:h) ===" | Out-File "$out\sizes15.txt"
$expRows | Out-File "$out\sizes15.txt" -Append
"=== collapse card rect ===" | Out-File "$out\sizes15.txt" -Append
$colRows | Out-File "$out\sizes15.txt" -Append

# ---- 连点 10 次（间隔 120ms < 340ms 动画时长） ----
for ($k = 0; $k -lt 10; $k++) {
  if ($k % 2 -eq 0) { & bun "$PSScriptRoot\cdp.mjs" eval 'const p=document.getElementById("pill");p.dispatchEvent(new MouseEvent("mousedown",{bubbles:true}));p.dispatchEvent(new MouseEvent("mouseup",{bubbles:true}));"e"' | Out-Null }
  else { & bun "$PSScriptRoot\cdp.mjs" eval 'document.getElementById("btn-collapse").click(); "c"' | Out-Null }
  Start-Sleep -Milliseconds 120
}
Start-Sleep -Milliseconds 1000
$final = & bun "$PSScriptRoot\cdp.mjs" eval 'const a=document.getElementById("app");const r=a.getBoundingClientRect();a.className+" card="+Math.round(r.width)+"x"+Math.round(r.height)' 2>$null
"rapid10 final: $final" | Out-File "$out\sizes15.txt" -Append
Save-WindowShot "$out\rapid10-final.png" -pad 60 | Out-Null

# ---- 深色主题帧 ----
& bun "$PSScriptRoot\cdp.mjs" eval 'document.documentElement.dataset.theme="dark"; "dark"' | Out-Null
Start-Sleep -Milliseconds 300
& bun "$PSScriptRoot\cdp.mjs" eval 'const p=document.getElementById("pill");p.dispatchEvent(new MouseEvent("mousedown",{bubbles:true}));p.dispatchEvent(new MouseEvent("mouseup",{bubbles:true}));"e"' | Out-Null
Grab "expand-dark" 8 40
Start-Sleep -Milliseconds 500
& bun "$PSScriptRoot\cdp.mjs" eval 'document.getElementById("btn-collapse").click(); "c"' | Out-Null
Grab "collapse-dark" 8 40
Start-Sleep -Milliseconds 500
& bun "$PSScriptRoot\cdp.mjs" eval 'document.documentElement.dataset.theme="light"; "light"' | Out-Null
Write-Output "anim15 done; rapid10 final: $final"
