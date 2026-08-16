# CLAUDE.md（项目规则入口）

## 定位
Windows 悬浮番茄钟，「假如苹果做番茄钟」。Tauri 2 + Vanilla TS。无 git 仓库、无安装包、无云同步/任务管理/白噪音（死规矩，不许加）。自启为 v10 任务书破例项（tauri-plugin-autostart，HKCU Run，默认关）；NSIS 打包为 v14 任务书破例项（1.0.0，targets=["nsis"]，含单实例保护——dev 与安装版共 identifier 互斥，见 BLOCKED #14）。

## 跑
- `bun install` → `bun run tauri dev`（vite 端口 **1430**，1420 属他人项目勿动）
- 测试：`cd src-tauri && cargo test`（基线 45 绿、0 ignored；不许 skip/放宽断言/mock 被测对象）

## 技术栈
- 计时/统计唯一真相在 Rust（`timer.rs` 状态机、`stats.rs` 聚合）；**禁止在 JS 再写计时逻辑**（mock 仅供截图评审，不参与真实计时）
- UI 只显示+转发 IPC；窗口代码创建于 `lib.rs` setup（debug 注入 `--remote-debugging-port=9223` CDP）
- stats.json 存 exe 同目录；缺失/损坏文件必须不崩归零（有测试守住）

## 目录与约定
- `docs/review/` 视觉评审循环档案（round-N.md 原文+分诊、round-N*.png 截图）；视觉改动须走 `pomodoro-visual-review` skill（`.claude/skills/`，2026-08-14 由四轮实战沉淀：采集管线选择、opus 独立评审员提示词纪律、分诊口径、收敛判据全在里面），连续 2 轮无 P1 才达标，封顶 6 轮
- `docs/solutions/` 已解方案知识库（按类目组织、带 YAML frontmatter 可检索：module/tags/problem_type）；在已记录领域做改动或调试前可先检索，非平凡问题解决后经 `/ce-compound` 沉淀
- `CONCEPTS.md` 项目共享领域词汇表（明卷/评审循环/物理注入/morph/材质立法等核心术语的唯一定义处）——定向代码库或讨论领域概念时可查
- **材质规则立法（v8）**：`[data-material]` 作用域只允许改 background/box-shadow/color/border-color/backdrop-filter；禁止改 padding/margin/width/height/gap/font-size/border-width——切换材质不得引起任何控件位移（几何两档恒等，改动后须 CDP rect 全等实测）
- `docs/screenshots/` 验收证据；`PROGRESS.md` 断点状态；`BLOCKED.md` 待裁决清单（顺手功能一律先进 BLOCKED，不许直接做）
- 自动化验证用 `scripts/cdp.mjs`（eval/click/drag/down/move/up）+ `scripts/win.ps1`；评审/验收管线已收编进 scripts/（2026-08-14，原 job tmp 会随会话清理）：`v7-env.ps1`（统一 P/Invoke 环境，各脚本 dot-source）、`cdp-seq.mjs`（管线版 CDP：state/jitter/downmove/leave/esc/shot）、`v61-capture.ps1`（八态采集）、`v8-geometry.ps1`（材质几何不变性验收）、`v61-regression.ps1`/`v9-blur.ps1`（v7 回归/失焦抽验）、`v9-hover.mjs`+`v9-capture.ps1`（悬停/按压定时帧+裁切放大）、`v10-capture.ps1`（八态+统计四态）、`v10-tray.ps1`（托盘 UIA 取证：图标/聚焦/菜单/退出）、`v10-multimon.ps1`（出屏钳制+边缘拖拽实测）；中间帧统一落 `%TEMP%\pomo-shots`；PowerShell 注入输入需关沙箱，SetCursorPos 最稳，SendInput 绝对坐标在此机映射异常

## 当前状态与下一步
2026-08-15 v14.1 托盘化收口：主窗 skip_taskbar(true)+CloseRequested 转隐藏（任务栏关闭死态根治），1.0.1 重装实测全过，cargo test 45 绿。安装版 1.0.1 在跑（D:\番茄钟）。**dev 与安装版共 identifier 互斥**——重启 dev（`bun run tauri dev`）前先退安装版。留裁：首启 tray_menu_fit 136 竞态（P3，BLOCKED #14）+ dev 互斥被自启放大（P2 待裁决）+ 真双屏（#3）+ 通知横幅（#1）。
