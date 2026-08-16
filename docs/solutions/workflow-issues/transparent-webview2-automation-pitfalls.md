---
type: solution
title: 透明 WebView2 窗口自动化测试陷阱四则（SetCursorPos 无 WM_MOUSEMOVE / 合成 hover 跨 hide/show 残留 / CDP 双重 JSON / GetWindowTextW 缺 CharSet.Unicode）
date: 2026-08-15
category: workflow-issues
module: test-automation
problem_type: tooling_pitfall
component: test_scripts
severity: medium
symptoms:
  - "SetCursorPos 把光标挪到窗口上，页面 :hover/mousemove 毫无反应（误判为「窗口收不到真实输入」）"
  - "透明窗弹出层截图出现「幽灵 hover 蓝底 / 焦点环」，干净态复现不了"
  - "pwsh 里 ConvertFrom-Json 解析 CDP eval 结果得到字符串而不是对象"
  - "C# GetWindowTextW 读中文窗口标题得到乱码（ju儫?）"
root_cause: tooling_misuse
resolution_type: playbook
last_updated: 2026-08-15
related_components:
  - "scripts/v11-realinput.ps1"
  - "scripts/v11-tray.ps1"
  - "scripts/v13-menus.ps1"
  - "scripts/cdp-seq.mjs"
tags: [windows, webview2, transparent-window, automation, setcursorpos, mouse_event, hover-residue, cdp, json-encoding, pinvoke-charset, tray-menu]
---

# 四则陷阱与正确姿势

## 1. SetCursorPos 不产生 WM_MOUSEMOVE
SetCursorPos 只挪光标位置，**不向窗口投递鼠标消息**——页面 :hover 不会更新。
要造真实 hover：SetCursorPos 落点后用 `mouse_event(MOUSEEVENTF_MOVE, ±1, ∓1)` 相对微移
（相对位移无需坐标归一化，避开本机 SendInput/mouse_event 绝对坐标映射异常）。
实证：scripts/v11-realinput.ps1 REAL-INPUT PASS。

## 2. 合成 :hover 跨 hide/show 残留（Chromium 渲染器内鼠标位置无 Web API 可清）
长命 dev 实例被 CDP 合成点击污染后，渲染器记住的鼠标位置让 :hover 粘在元素上，
hide→show 甚至 location.reload() 都不清。页面侧 blur()/pointer-events 翻转均无效。
**姿势**：采集截图前 `cdp.mjs move 1 1` 把合成鼠标归位到角落，或重启实例；否则评审会看到
「无操作中某元素 hover 高亮」的假态（v11 round-40 P2-1 的蓝底即此残留，差点误报产品缺陷）。

## 3. cdp-seq eval 返回的双重 JSON 编码
`cdp-seq.mjs eval` 对返回值再 JSON.stringify 一次：IIFE 里 `return JSON.stringify(obj)` →
外层拿到的是字符串。要么 IIFE 直接 return 对象（returnByValue 带出，单层解析），
要么 PowerShell 侧解析两次。混用必踩（v11-realinput 首跑误判 FAIL 即此）。

## 4. P/Invoke 读中文窗口标题必须 CharSet.Unicode
`[DllImport("user32.dll")] GetWindowTextW(...)` 不写 CharSet 时按 Ansi 封送：
调 W 函数却按 ANSI 缓冲区读写 → 中文标题变乱码（且 128 字节缓冲 vs 128 UTF-16 码元有溢出风险）。
正确：`[DllImport("user32.dll", CharSet = CharSet.Unicode)]`。
另外 `EnumWindows` 回调别用 `System.Func<>` 泛型委托（non-blittable 不可封送），要定义具名 delegate。

## 5. 「move 归位」被半套用：SetCursorPos 挪光标 ≠ 发鼠标消息（v13 托盘菜单四图 hover 钉死）
§1/§2 的教训「采集前 move 归位」，本义是**让渲染器收到一条真实鼠标移动消息以重算 :hover**。
v13 首跑把「归位」实现成了 `SetCursorPos(400,400)`——只挪光标不发消息（§1），长命托盘页渲染器
残留的 hover（§2，钉在「开机自启」上，系此前会话合成操作所留）原样带进四张截图，全废重采。
**正解**（scripts/v13-menus.ps1 Open-TrayMenu）：真实右键弹出菜单后，光标本来就在菜单窗外的
托盘图标上，原地补一条真实微移即可：`mouse_event(MOUSEEVENTF_MOVE, 1, 0, ...)`——渲染器收到
WM_MOUSEMOVE 后按真实光标坐标（窗外）重算，hover 自清；+1px 即可，无需挪到固定坐标。
反例对照：v12 同流程完全不做归位反而零残留（该实例渲染器恰无残留 hover）——**别拿单次侥幸
当依据，一律发消息**。

## 6. mouse_event 的 dx/dy 是 DWORD：负相对位移直接 ArgException
P/Invoke 声明 `mouse_event(uint f, uint dx, ...)` 时传 `-1` → PowerShell 立即
`Cannot convert argument "dx" ... for "mouse_event" to type "System.UInt32"`，脚本中断且状态
残留在染料中途（v13 次跑即此，托盘菜单悬空需手动 tray_menu_hide + dataset 还原）。
相对微移需要负位移时传 `[uint32]"0xFFFFFFFF"`（= -1 的位模式）；通常 +1px 单向微移就够用。

## 7. 双 Windows Terminal 全屏置顶：真实鼠标物理不可达 → 四件套组合拳（v7 教训，自 BLOCKED 教训节收编）
本机双 WT 全屏置顶时物理鼠标点击全落空（点到终端），「真实点击别的窗口验失焦」这类需求走
组合拳：**CDP Input.dispatchMouseEvent（页内合成）+ SetCursorPos/mouse_event（光标与真实
移动消息）+ WScript SendKeys（真实键盘，SendInput 键盘注入在本机被拦）+ AttachThreadInput
重试（SetForegroundWindow 单发前台权不稳）**。同一 Focused(false) 链路等价验证，环境偏离
书面登记（v7 先例）。

## 8. 透明窗圆角外像素点击穿透：合成鼠标终点须落胶囊中线（v6.1 教训，自 BLOCKED 教训节收编）
无边框透明窗的透明圆角区不收点击（点穿到桌面）——合成鼠标事件终点落在胶囊圆角外
（y<38 端部区）会触发 mouseleave 杀拖拽判定，拖拽验收会假 FAIL。
**正解**：合成移动/点击的终点一律落在胶囊**中线**（高度中点附近）。

## 附带：pwsh 嵌套 SendKeys 的引号坑
脚本里 `pwsh -Command "$w=New-Object ...; $w.SendKeys(...)"` 用双引号会被外层提前展开 $w 为空。
同进程直接用 `$ws = New-Object -ComObject WScript.Shell` 即可，不要嵌套 pwsh。
