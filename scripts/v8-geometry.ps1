# v8 P0 验收：两档材质下控件 getBoundingClientRect 全等 + 抽屉高度余量 + 面板底部留白
. D:\VibeCoding\pomodoro-clock\scripts\v7-env.ps1

$collect = @'
(()=>{
  const pick = {};
  const sels = ['.pill-phase','.pill-time','#btn-toggle-run',
    '.panel-phase','.panel-time','.progress-track','.cycle-dots',
    '#btn-primary','#btn-reset','#btn-switch','.stat-card','.week-chart','.collapse-btn',
    '.drawer-head','.drawer .section-caption','.drawer .group-card','.drawer .setting-row',
    '.drawer .stepper','.drawer .toggle','.drawer .theme-switch','.ctx-menu'];
  sels.forEach(s=>{
    document.querySelectorAll(s).forEach((el,i)=>{
      const r = el.getBoundingClientRect();
      pick[s+'#'+i] = [r.x,r.y,r.width,r.height].map(v=>+v.toFixed(2));
    });
  });
  const hr = [...document.querySelectorAll('.drawer .group-card')].map(c=>{
    const last = c.querySelector('.setting-row:last-child');
    return +(c.getBoundingClientRect().bottom - last.getBoundingClientRect().bottom).toFixed(2);
  });
  const foot = document.querySelector('.collapse-btn').getBoundingClientRect();
  return JSON.stringify({rects:pick, drawerHeadroom:hr, collapseBottom:+foot.bottom.toFixed(2)});
})()
'@

function Collect([string]$label) {
  $raw = Seq ("eval " + $collect)
  $raw | Out-File -Encoding utf8 "$script:ShotTmp\v8-geo-$label.json"
  Write-Output "collected $label"
}
function Set-Material([string]$m) {
  Seq ("eval (()=>{document.documentElement.dataset.material='$m';document.querySelectorAll('.material-opt').forEach(b=>b.classList.toggle('active',b.dataset.materialOpt==='$m'));return '$m'})()") | Out-Null
  Start-Sleep -Milliseconds 500
}
function Drawer([bool]$open) {
  $v = if ($open) { "true" } else { "false" }
  Seq ("eval document.getElementById('app').classList.toggle('drawer-open',$v);'$v'") | Out-Null
  Start-Sleep -Milliseconds 700
}

# 确保展开态（宽容点击链路：CDP dispatch 由 cdp-seq 的 jitter 模拟）
Seq "jitter 100 38 2 2" | Out-Null; Start-Sleep -Milliseconds 1400

# --- 面板态（抽屉关）：liquid → classic ---
Drawer $false
Set-Material liquid_glass; Collect "panel-liquid"
Set-Material classic;     Collect "panel-classic"

# --- 抽屉态 ---
Drawer $true
Collect "drawer-classic"
Set-Material liquid_glass; Collect "drawer-liquid"
Drawer $false

# --- 右键菜单态 ---
Seq "eval (()=>{const m=document.getElementById('ctx-menu');m.style.left='180px';m.style.top='200px';m.classList.add('open');return 'open'})()" | Out-Null
Start-Sleep -Milliseconds 400
Collect "ctx-liquid"
Set-Material classic; Collect "ctx-classic"
Seq "eval document.getElementById('ctx-menu').classList.remove('open');'closed'" | Out-Null
Set-Material liquid_glass

# --- 收起回 mini，测胶囊 ---
Esc-Collapse; Start-Sleep -Milliseconds 1200
Collect "mini-liquid"
Set-Material classic; Collect "mini-classic"
Set-Material liquid_glass
Write-Output "geometry collection done"
