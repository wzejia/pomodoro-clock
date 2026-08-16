# 番茄钟（pomodoro-clock）

「假如苹果做番茄钟」——Windows 桌面悬浮番茄时钟：只有计时和统计，质感对标 Apple。Tauri 2 + Vanilla TS，计时与统计的唯一真相在 Rust，UI 只负责显示和转发事件。

## 跑起来

```bash
bun install
bun run tauri dev        # vite 端口 1430（1420 被本机其他项目占用，勿改回）
```

- 测试：`cd src-tauri && cargo test`（21 个，全绿基线）
- 构建：`bun run build`（前端）；发布构建 `bun run tauri build`（未做过，无安装包目标）

## 功能

- 迷你胶囊悬浮窗（无边框、置顶、可拖拽）：倒计时 + 阶段色标签 + 播放/暂停
- 单击胶囊展开统计面板：开始/暂停/重置/跳过、今日番茄/专注分钟/连续天数、本周每日柱状、工作/短休时长设置（默认 25/5，连 4 个番茄长休 15 分钟）、亮暗双主题
- 阶段完成自动切换下一阶段 + 系统通知 + 一声轻提示音（WebAudio 合成）
- 统计持久化：`stats.json` 存 exe 同目录（dev 时为 `src-tauri/target/debug/stats.json`）

## 结构

| 位置 | 作用 |
|---|---|
| `src-tauri/src/timer.rs` | 计时状态机（唯一计时真相，全单测覆盖） |
| `src-tauri/src/stats.rs` | 记录、聚合（今日/本周/连签）、JSON 存取 |
| `src-tauri/src/lib.rs` | IPC 命令、250ms tick 线程、通知、自研拖拽线程 |
| `src/main.ts` | UI 渲染与事件转发（无计时逻辑） |
| `src/bridge.ts` | Tauri IPC 桥 + 浏览器 mock（仅供截图评审） |
| `scripts/cdp.mjs` | WebView2 CDP 驱动（debug 构建自动开 :9223） |
| `scripts/win.ps1` | 窗口矩形/截图/输入注入工具 |
| `scripts/capture-round.ps1` | 评审轮四态截图采集 |

## 状态（2026-08-12）

全部既定任务完成：21 测试全绿；悬浮窗/拖拽/展开收起实测通过；统计跨天/空数据/断签有测试；10 轮独立视觉评审循环收敛（round-9/10 连续「无 P1」，记录见 `docs/review/`）。待裁决项见 `BLOCKED.md`（最重要的是：系统通知总开关在本机为关，通知横幅需开通知后亲验）。
