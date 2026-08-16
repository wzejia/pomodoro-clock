---
type: solution
title: Tauri/WebView2 同 user data folder 第二窗口创建失败（additional_browser_args 必须与主窗逐字一致）
date: 2026-08-15
category: platform-integration
module: tray-menu-window
problem_type: platform_trap
component: rust_backend
severity: high
symptoms:
  - "第二个 WebviewWindow build() 返回 Ok、窗口在 Tauri 管理器注册，但原生句柄永不创建（hwnd=Unavailable）"
  - "show() 返回 Ok 但窗口不可见；is_visible()/set_position() 全部假成功"
  - "错误原文 ERROR_INVALID_STATE(0x8007139F) 被 tauri-runtime-wry 吞进 log::error，无 panic 无弹窗"
root_cause: platform_behavior
resolution_type: config_fix
related_components:
  - "src-tauri/src/lib.rs (tray-menu 窗口创建块)"
tags: [tauri2, webview2, second-window, additional_browser_args, error-invalid-state, transparent-window, tray-menu]
---

# 现象与根因

v11 托盘自绘菜单新建第二个 WebviewWindow（无边框透明小窗）：`build()?` 不报错，窗口对象存在，
但原生句柄永不创建，show 无效。挂 log crate 捕获到被吞的原文：
`ERROR_INVALID_STATE (0x8007139F)`——WebView2 同一 user data folder 下的第二个浏览器环境，
**浏览器参数（additional_browser_args）必须与首个窗口逐字一致**，否则环境创建失败且失败被运行时吞掉。

# 修法

第二窗的 `additional_browser_args` 与主窗**逐字相同**（含 `--remote-debugging-port=9223`）：

```rust
// 实测（v11 排障）：WebView2 同 user data folder 的第二窗口必须以「与主窗完全一致」的
// browser args 创建环境，否则 ERROR_INVALID_STATE(0x8007139F) 建窗被运行时吞掉。
#[cfg(debug_assertions)]
{
    menu_builder = menu_builder.additional_browser_args(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-port=9223",
    );
}
```

# 副作用（反而是好事）

参数一致 → 两窗共用浏览器进程 → 第二页直接出现在同一 CDP 端口（9223）的 /json 目标列表，
无需第二调试口。多目标时用 URL 区分（本项目 scripts/cdp.mjs 的 `CDP_MATCH=tray-menu`）。

# 排障教训

- Tauri 的窗口创建错误可能被 wry 吞进 log::error：窗口「假注册」时先挂 `log` crate 或查 dev 控制台 stderr。
- 排障用的临时日志代码用完必须删干净（grep 确认无残留）。
