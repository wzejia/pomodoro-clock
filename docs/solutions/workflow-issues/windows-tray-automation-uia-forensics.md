---
type: solution
title: Windows 托盘自动化取证三坑：UIA 同名歧义、chevron toggle 陷阱、键盘赛道兜底
date: 2026-08-14
category: workflow-issues
module: tray-automation
problem_type: workflow_issue
component: development_workflow
severity: medium
root_cause: missing_workflow_step
resolution_type: workflow_improvement
applies_when:
  - 需要自动化操作或验收 Windows 系统托盘图标与右键菜单（Tauri/Electron/原生桌面应用）
  - UIA 按 Name 搜到多个同名托盘元素，需要区分任务栏按钮与溢出面板真图标
  - UIA 枚举不到应用托盘弹出菜单的 MenuItem，需要键盘赛道驱动菜单项
  - 本机 SendInput 注入被拦、或 PowerShell 工具每次调用新进程导致 Add-Type 类型不跨调用存活
symptoms:
  - UIA 按 Name 搜托盘图标同名命中任务栏按钮（84x48），右键弹出 Windows 跳转列表而非应用托盘菜单
  - 盲点「^ 显示隐藏的图标」chevron 把已开的溢出面板关掉，随后图标枚举失败或退化到任务栏按钮
  - UIA 枚举不到 Tauri 托盘弹出菜单的 MenuItem，菜单项自动化无路
  - 盲注链右键落桌面误执行桌面菜单项（事故起源）
related_components:
  - scripts/v10-tray.ps1（icon/focus/menu/menurun/quit/quitkeys 六步）
  - scripts/v7-env.ps1（统一 P/Invoke 环境）
  - docs/screenshots/v10/tray-*.png（验收证据）
tags: [windows, tray, uia, automation, powershell, input-injection, acceptance-testing, tauri]
---

# Windows 托盘/任务栏自动化取证方法论（UIA + 物理注入）

## Context

番茄钟 v10（Tauri 2）任务 1 的验收要求对系统托盘做全链路取证：截图证明托盘图标存在、左键点击能把悬浮窗聚焦、右键弹出三项菜单（「开始/暂停」动态文案 +「设置…」+「退出」）、点「退出」后进程真实消失。这台机器没有现成 GUI 自动化框架可用——验收链路是 PowerShell 7 + UIAutomation（UIA）+ CDP + P/Invoke 物理注入（SetCursorPos / mouse_event / WScript SendKeys）从零拼出来的。最终硬化产物是 `scripts/v10-tray.ps1`（六个 Step：icon / focus / menu / menurun / quit / quitkeys），公共 P/Invoke 环境在 `scripts/v7-env.ps1`。过程中踩了三个结构性的坑，外加一起真实事故催生的「先验证再按键」纪律。这些坑不挑项目——任何在 Windows 上要对托盘/任务栏做自动化取证或验收的场景都会遇到。

这套链路并非从零起步：v7 时代（2026-08-13）为验收「宽容点击/拖拽/失焦即收」已从零搭过物理注入环境并沉淀为 `v7-env.ps1` 枢纽，当时的分工格局是「OS 级交互走物理注入、页内证据走 CDP 合成事件」，v10 托盘取证直接继承了这个格局 (session history)。v7 的起步摩擦（首个注入脚本连续报错、被中断三次才跑通）与「注入需关沙箱」的事后口径时间线形态一致 (session history)。

## Guidance

### 坑 1：UIA 同名干扰——按 Name 搜会同时命中任务栏按钮和托盘图标

`FindAll(ControlType.Button)` 按 Name 含「番茄钟」过滤，会同时命中两个元素：任务栏窗口按钮（84x48）和溢出面板里的托盘图标（40x40）（坐标亦不同，本机 2026-08-14 实测分别约 y≈1056 与 y≈915，换机须重测）。FindAll 先返回谁不确定。后果不是找不到，而是**找错**——右键点在任务栏按钮上，弹出的是 Windows 跳转列表（番茄钟/固定到任务栏/关闭窗口），我们曾把这个跳转列表误当托盘菜单截图存证，险些拿它交差。

解法：收集全部匹配后按几何尺寸过滤，取 `BoundingRectangle.Width <= 60` 者（托盘图标 40x40，任务栏按钮 84x48，宽度是稳定区分特征）。见 `scripts/v10-tray.ps1` 的 `Find-OverflowIcon`：

```powershell
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
```

教训的一般形态：**UIA 按 Name 搜索不是唯一标识，桌面 shell 里同名元素极常见**（应用窗口、任务栏按钮、托盘图标、跳转列表项常常共享产品名）。必须叠加几何（BoundingRectangle 尺寸/坐标区间）或父链特征做二次确认。

### 坑 2：溢出区「^」chevron 是 toggle，盲点会让面板关上

托盘图标在溢出面板里，点「^ 显示隐藏的图标 / Show hidden icons」chevron 才能看到。但 ^ 是 toggle：面板关着时点 = 开，**开着时点 = 关**。硬化前的脚本是「先点 ^ 再枚举图标」——如果上一轮运行把面板留在了开态，这次盲点一下反而把它关了，枚举落空，又退化成命中任务栏按钮（回到坑 1）。

解法：把「点 ^」从无条件前置改成兜底——循环至多 3 轮，每轮先 FindAll 找 ≤60 宽的图标，找不到才去 UIA 找 chevron 按钮点开再查。这样 toggle 的两个方向都被循环兜住：面板关着→第一轮找不到→点开→第二轮找到；面板开着→第一轮直接找到，^ 根本不会被碰。教训的一般形态：**对 toggle 型 UI 控件，操作前先读状态，或把操作包进「检查→按需操作→再检查」的循环，绝不在不知道当前态的情况下盲点**。

### 坑 3：UIA 枚举不到 Tauri 托盘菜单项——键盘赛道 + 前置确证

右键菜单弹出后，最自然的做法是 UIA `FindAll(ControlType.MenuItem)` 按 Name「退出」定位再点击（`scripts/v10-tray.ps1` 的 `quit` Step 就是这条路线，且保留了 `QUIT ITEM NOT FOUND` 的失败分支）。实测在本机对 Tauri 托盘弹出菜单不可靠：菜单是瞬态的、不完全进 UIA 树，按 Name 找不到项。

解法=键盘赛道，且安全性不靠运气靠前置确证（`scripts/v10-tray.ps1` 的 `quitkeys` Step）：

1. **右键点本身已经 UIA 确证**——`Find-OverflowIcon` 返回的是真实的 40x40 溢出面板图标，`Click-At` 的坐标从它的 BoundingRectangle 算出，所以弹出的必是我们应用的三项菜单，不可能是别的东西。
2. **按键前同一次调用内先截图存证**——右键后、SendKeys 前先 `Shot-Rect` 存 `tray-menu-before-quit.png`，事后可审计按键瞬间屏幕上确实是我们的菜单。
3. **菜单内导航用方向键环绕**：三项菜单里 `{DOWN}{ENTER}` = 首项「开始/暂停」，`{UP}{ENTER}` = 从首项环绕到末项「退出」。
4. **结果用独立通道验证**：`{DOWN}{ENTER}` 后查实 timer 状态 idle→running（本会话经 CDP `timer_snapshot` 实证，该通道未固化进脚本）；`{UP}{ENTER}` 后用 `Get-Process pomodoro-clock` 查无进程——按键效果不靠截图猜，靠应用自身状态裁决。

```powershell
"quitkeys" {
  # UIA 枚举不到 Tauri 托盘菜单项 → 键盘赛道：{UP} 从首项环绕到末项「退出」+{ENTER}
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
```

### 事故纪律：物理注入链一律「先验证再按键」，拒绝盲注

纪律的由来是一起真实事故：硬化前的盲注链里，^ chevron 点击被吞（本机有并行 AI 会话全屏置顶抢镜/坐标漂移），右键落在桌面上弹出了**桌面右键菜单**，随后的 `{DOWN}{ENTER}` 在桌面菜单上误执行了一项。此后立法：任何物理注入链，点击落点必须先经独立通道确证（UIA 元素坐标），破坏性按键（Enter/方向键）前必须先截图存证，按键效果必须由应用状态（CDP 查询 / Get-Process）裁决而不是由「按键动作成功发出」推断。

### 本机环境事实清单（前置约束，换机须重测）

- SendInput 的键盘和鼠标按钮注入均被拦截；`mouse_event` 有效；`SetCursorPos` 最稳；WScript SendKeys 键盘有效（`v10-tray.ps1` 的 `Click-At` 即 SetCursorPos + mouse_event 组合）。
- PowerShell 工具每次调用是新进程：`Add-Type` 的类型、变量不跨调用存活，脚本必须自含（公共部分抽 `v7-env.ps1` 用 `. ` dot-source 引入）。
- 沙箱内 localhost 走代理，CDP（9223）命令需关沙箱执行。
- 用 pwsh 7 跑脚本——PS 5.1 会把无 BOM 的脚本按 GBK 误读，中文匹配串全废。
- PowerShell 表达式坑（v7 回灌）：`New-Object Type($w*$zoom, ...)` 参数模式不求值表达式——.NET 构造一律 `::new()`（已入全局 errors.md）；逗号优先级高于 `+`，`@($r.Left + 100, ...)` 会被解析成 `$r.Left + (100, ...)`，坐标数组全错（记录在 PROGRESS.md v7 条目）(session history)。

## Why This Matters

不遵守这三坑一纪律时的具体失败模式，每一个都是本项目真实踩过的：

- **不做几何过滤**（坑 1）：右键落在任务栏按钮上，截到的是 Windows 跳转列表而非应用托盘菜单——取证产物张冠李戴，且因为跳转列表里也有应用名，肉眼不细看看不出问题，错误证据可能直接流入验收结论。
- **盲点 toggle**（坑 2）：面板开着时点 ^ 把它关了，枚举落空后若脚本有 fallback 逻辑可能退而命中任务栏按钮，与坑 1 形成连环；没有 fallback 则报「找不到图标」的假阴性，浪费整轮排查。
- **盲注键盘**（事故）：点击落点被抢镜/漂移污染后，方向键+回车作用在错误的菜单上——本项目真实发生过在桌面右键菜单上误执行一项。桌面菜单里有「查看/排序/刷新/个性化」等项，误触后果不可枚举。
- **跳过独立裁决**：SendKeys「发出去了」不等于「生效了」，菜单可能没弹出、焦点可能被抢。不用 CDP/Get-Process 验证最终状态，QUIT/FOCUS 结论就没有事实基础。
- **合成事件与物理态要分清权威源** (session history)：v7 拖拽冻结 bug 的判例——JS `mouseleave` 在快拖时杀掉 Rust 拖拽线程（物理光标跑得比窗口跟随快，先出窗即触发 leave），修法是 leave 不再杀线程、Rust 侧用 `GetAsyncKeyState` 轮询物理左键松开自终止。「JS 合成事件不可信、物理键态以 GetAsyncKeyState 为准」与本文「按键效果由应用状态裁决」是同一条原则的两处应用。
- **CDP 够得着就不物理** (session history)：v7 起确立的分工——页内 hover/按压/rect 证据走 CDP 合成事件+截图，物理注入只用于焦点切换、全局按键、跨窗口拖拽、托盘这类 CDP 够不着的 OS 级动作。v7 失焦即收验收甚至因「全屏置顶终端让真实鼠标无处点桌面」改用 P/Invoke 抢前台模拟失焦，并书面登记环境偏离声明——不假装没验过的东西验过。

## When to Apply

- Windows 桌面应用的自动化验收/取证，尤其涉及系统托盘、任务栏、通知区溢出面板。
- 任何用 UIA 枚举桌面 shell 元素的场景（shell 里同名元素是常态）。
- 需要操作 Tauri/Electron 等框架的托盘菜单（这类瞬态原生菜单经常不进 UIA 树）。
- 任何物理输入注入链（SetCursorPos/mouse_event/SendKeys）编排——「先验证再按键」纪律普适。

## Examples

**反例（硬化前）**：脚本 UIA 按 Name 搜「番茄钟」取首个匹配并右键——命中的是任务栏窗口按钮（84x48），弹出 Windows 跳转列表（番茄钟/固定到任务栏/关闭窗口），截图留存后一度被当成「托盘菜单」证据。另一次 ^ 点击被吞后右键落在桌面，`{DOWN}{ENTER}` 在桌面菜单上误执行。

**正例（硬化后，`scripts/v10-tray.ps1`）**：`Find-OverflowIcon` 三轮循环 + 宽度 ≤60 过滤锁定溢出面板里的 40x40 图标（证据 `docs/screenshots/v10/tray-overflow.png`、`tray-icon.png`）；右键后截图得真正的应用三项菜单（`tray-menu.png` 为 idle 态首项「开始」，`tray-menu-running.png` 为 running 态首项「暂停」——动态文案证据）；`quitkeys` Step 右键后先存 `tray-menu-before-quit.png` 再 `{UP}{ENTER}`，`Get-Process` 查无 pomodoro-clock 输出 `QUIT PASS`。与 BLOCKED.md「长效环境教训」段、docs/DEV_LOG.md 2026-08-14 v10 条目「环境/工具」段的压缩版口径一致，本文是扩展版。

## Related

- BLOCKED.md「长效环境教训」（压缩版速查，本文是扩展版；后续可考虑改为指向本文的一行引用，见 ce-compound-refresh 建议）
- docs/DEV_LOG.md 2026-08-14 v10 条目「环境/工具」段
- 会话历史来源：v7~v9 会话（2026-08-13/14）提供物理注入纪律的起源现场（v7-env.ps1 收编、失焦抢焦偏离声明、GetAsyncKeyState 判例）
