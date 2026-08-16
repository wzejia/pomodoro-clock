---
type: index
title: 项目机构解法索引
description: Auto-generated index of docs/solutions/
generated_at: 2026-08-15
tags: [index, solutions]
---

# Solutions Index（pomodoro-clock）

| 文件 | 标题 | 类目 | 要点 |
|---|---|---|---|
| [ui-bugs/liquid-glass-on-light-surface-invisible](ui-bugs/liquid-glass-on-light-surface-invisible.md) | 浅色近白表面上做液态玻璃区分度 | ui-bugs | 白 sheen 白上叠白不可见（ΔRGB 6~7）；三支柱=冷灰底霜+底部暗带+深描边；像素校准纪律 |
| [ui-bugs/css-box-shadow-shorthand-override-annihilation](ui-bugs/css-box-shadow-shorthand-override-annihilation.md) | box-shadow 简写覆盖整棵替换 | ui-bugs | 杀外阴影连废液态 inset；覆盖段须显式重写要保留的层；跨环境同组件改动双环境截图 |
| [ui-bugs/settings-drawer-flex-shrink-clipping](ui-bugs/settings-drawer-flex-shrink-clipping.md) | 设置抽屉 flex-shrink 裁切 | ui-bugs | overflow:hidden 的 flex 子项 min-height:auto=0 被 shrink |
| [workflow-issues/transparent-webview2-automation-pitfalls](workflow-issues/transparent-webview2-automation-pitfalls.md) | 透明 WebView2 自动化陷阱六则 | workflow-issues | SetCursorPos 无 WM_MOUSEMOVE（造/清 hover 同理须发消息）；合成 hover 跨 hide/show 残留；cdp-seq 双重 JSON；GetWindowTextW CharSet；mouse_event DWORD 负数 |
| [workflow-issues/windows-tray-automation-uia-forensics](workflow-issues/windows-tray-automation-uia-forensics.md) | Windows 托盘 UIA 取证四教训 | workflow-issues | UIA 同名按宽度过滤；^ toggle 先查再点；键盘赛道 {DOWN}/{ENTER}；SendInput 被拦→mouse_event |
| [workflow-issues/living-timer-capture-phase-race](workflow-issues/living-timer-capture-phase-race.md) | 活体计时应用采集相位竞速 | workflow-issues | async IIFE 模板；时敏帧=相位归一+断言硬化，禁运行中随手截 |
| [workflow-issues/user-parallel-capture-discipline](workflow-issues/user-parallel-capture-discipline.md) | 用户并行使用期采集纪律五条 | workflow-issues | 备份-恢复+哈希；idle 门控；只翻 DOM 不落盘；还原目标=开工现读 config；hover 卫生 |
| [workflow-issues/printwindow-webview2-stale-frame](workflow-issues/printwindow-webview2-stale-frame.md) | PrintWindow 对 WebView2 抓缓存陈帧 | workflow-issues | PW_RENDERFULLCONTENT 也拿缓存帧，连拍全等≠页面静止（冻结假象）；判活/读数一律 CopyFromScreen 屏采；release 无 CDP 状态真相走应用自身出口（托盘文案=Rust 现查） |
| [platform-integration/tauri-webview2-second-window-invalid-state](platform-integration/tauri-webview2-second-window-invalid-state.md) | WebView2 第二窗 browser args 一致性 | platform-integration | 同 user data folder 第二窗 additional_browser_args 须与主窗逐字一致，否则 ERROR_INVALID_STATE 被吞窗永不建 |
