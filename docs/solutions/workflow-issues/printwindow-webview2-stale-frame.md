---
type: solution
title: PrintWindow 对 WebView2 抓缓存陈帧——release 无 CDP 时的截图判活陷阱
date: 2026-08-15
category: workflow-issues
module: test-automation
problem_type: tooling_pitfall
component: test_scripts
severity: high
symptoms:
  - "release 构建（无 --remote-debugging-port）下用 PrintWindow(PW_RENDERFULLCONTENT=2) 连拍 WebView2 窗口，两帧像素全等"
  - "对照运行中的倒计时/动画窗口，误判为「UI 冻结/计时器停走」——产品缺陷假象"
  - "同窗不同时刻拍出内容偶有差异但不连续递进，与真实屏幕状态对不上"
root_cause: tooling_misuse
resolution_type: playbook
last_updated: 2026-08-15
related_components:
  - "src-tauri/src/lib.rs（debug 才注入 9223 CDP）"
  - "docs/DEV_LOG.md v14 条目"
tags: [windows, webview2, printwindow, copyfromscreen, release-build, no-cdp, screenshot, stale-frame, false-freeze]
---

# 机制 / 正解 / 反例

## 机制
WebView2 内容走 DirectComposition 独立合成，`PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)` 拿到的是**窗口表面缓存里最后一次提交的帧**，不保证当前实时内容；窗口后台/无重绘请求时缓存可以长时间停在旧帧。透明置顶小窗上尤甚——连拍两张「全等」只说明缓存没换，不说明页面静止。

## 正解
- **判「内容是否在动/活着」一律 `Graphics.CopyFromScreen` 屏幕直采**（真实合成输出）：同区域间隔两帧做逐像素 diff，非零=在变。窗口须置前且无遮挡（本机双全屏置顶终端的历史坑：先确认屏面干净）。
- **判「内容长什么样」**（读数字/读文案）也优先屏采；PrintWindow 结果只可当「某历史帧」参考，不可当现状。
- release 无 CDP 时的状态真相改走**应用自身出口**：托盘菜单动态文案=每次打开现查 `timer_snapshot`（Rust 状态），比像素更硬。
- dev 构建继续用 CDP（9223）最稳；release 验收脚本模板：`GetWindowRect` → `CopyFromScreen(rect)` → 像素 diff / 交视觉模型读数。

## 反例（v14 实录）
2026-08-15 安装版冒烟：PrintWindow 连拍 mini 胶囊 t3/t4（间隔 2.6s）像素 0 差异 → 一度定性「计时器冻结」疑似产品缺陷；CopyFromScreen 同窗 2.2s diff 2818px、视觉读数 23:41 ⏸ 在走——推翻。教训入 DEV_LOG v14：**release 验收禁用 PrintWindow 判活**。

## 附带
- 屏采做「冻结验证」（期望 diff=0）时把光标移开窗口，避免外部 hover 高亮污染（本机多会话/有人使用时会偶发几百 px 差异）；时间数字区裁剪 diff 可缩小干扰面。
