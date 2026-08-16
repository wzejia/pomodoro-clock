# UI 优化实施计划（2026-08-12）

> 7 项改动，跨 6 文件。先改后端+测试（cargo test 绿）再改前端，最后视觉评审。

## 0. 执行顺序与依赖

```
后端先行（timer.rs 自动切换 + lib.rs 动画命令）→ cargo test 绿
   ↓
前端（bridge.ts 接口 → main.ts/html/css）
   ↓
图标资源（独立，tauri icon 生成）
   ↓
视觉评审循环（capture-round.ps1 → 独立评审子智能体 → 修 P1）
```

---

## 1. 自动切换开关（后端，先做，有测试）

**改 `timer.rs`**：
- `TimerConfig` 加字段 `auto_start_next: bool`，`Default` 实现里设 `false`（用户定）
- `tick()` 阶段结束分支（line 183-184）：`self.status = if self.config.auto_start_next { Status::Running } else { Status::Idle };`，Idle 时不再更新 `last_tick_ms`
- 仅 `auto_start_next=true` 时才 `last_tick_ms = now_ms`（Idle 停表不推进）

**测试适配**：
- 现有 `work_completion_auto_switches_to_short_break` / `break_completion_returns_to_work` / `fourth_work_triggers_long_break` 断言 `status==Running` -> 改用显式 `auto_start_next: true` 的 config（在 default 基础上覆盖该字段）
- **新增** `work_completion_stops_when_auto_start_disabled`：默认 config（false），tick 到结束断言 `status==Idle`、`phase==ShortBreak`、`remaining==5*MIN`
- **新增** `auto_start_enabled_continues_running`：true 时断言 `status==Running`（保留原语义）
- 基线：原 21 绿 -> 改后仍 ≥23 绿，0 ignored，不 skip 不放宽

**改 `lib.rs`**：`set_config` 命令签名加 `auto_start_next: bool` 参数，写入 cfg。

**改 `bridge.ts`**：`TimerConfig` 接口加 `auto_start_next: boolean`；mock 的 `get_config` / `set_config` 适配。

---

## 2. 展开动画（Rust 逐帧 SetWindowPos）

**改 `lib.rs`**：新增 `anim` 模块（复用 `drag` 模块的 user32 FFI：`SetWindowPos`/`GetWindowRect`/`GetDpiForWindow`）+ `animate_size` 命令：
- 参数：`to_w, to_h`（逻辑像素），Rust 端 `GetWindowRect` 读当前尺寸作 from
- ease-out cubic：`t' = 1 - (1-t)^3`，~220ms，~16ms/帧（60fps）
- 用 `AtomicBool` 防重入：新动画请求先置 ACTIVE=false 终止旧线程，再开新线程
- 仅 `#[cfg(windows)]`，非 windows 回退瞬时 `setSize`

**改 `bridge.ts`**：`createTauriBridge` 的 `setWindowSize` 改为调 `invoke("animate_size", { width, height })`；mock 保持 CSS transition（已可用）。

**验证**：dev 模式点展开/收起，窗口尺寸平滑插值（无瞬跳）；快速连点不撕裂（防重入生效）。

---

## 3. 内容重组（齿轮抽屉）

**改 `index.html`**：
- 主面板保留：计时区 + 控件 + 统计
- 设置区（setting-row ×3 + 主题）移入新 `<div class="drawer">`，默认 `transform: translateX(100%)` 隐藏
- `panel-head` 加齿轮按钮 `<button id="btn-settings" class="icon-btn">⚙</button>`
- 抽屉内加返回按钮

**改 `styles.css`**：
- `.drawer`：绝对定位覆盖面板右侧，`transition: transform 0.3s var(--spring)`，展开 `translateX(0)`
- 抽屉内背景同 surface，与主面板分层
- 齿轮按钮样式复用 `.icon-btn`

**改 `main.ts`**：`btn-settings` click -> toggle `app.classList("drawer-open")`。

**设计**：抽屉覆盖式（不改变 340px 窗口宽），从右滑入盖住主内容，点返回/齿轮收起。保持窗口尺寸恒定，避免动画叠加复杂度。

---

## 4. 展开页拖拽

**改 `main.ts`**：
- 提取 `attachThresholdDrag(el: HTMLElement)`：把 pill 的 mousedown/mousemove/mouseup 6px 阈值逻辑抽成函数，排除 `button`/`.icon-btn` 子元素
- 绑到 `#pill`（保留）+ `.panel-head`（新增）
- panel-head 拖拽时排除齿轮按钮、收起按钮

**改 `index.html`**：删掉 `panel-head` 上失效的 `data-tauri-drag-region` 属性（CLAUDE.md 坑#1，自研拖拽不用它）。

**验证**：展开态按住 phase/time 区域可拖动窗口；点齿轮/收起不触发拖拽。

---

## 5. 控件统一 + 数字居中

**改 `styles.css`**：
- 隐藏原生 spinner：`.stepper-val input::-webkit-inner-spin-button, .-outer-spin-button { display: none; }` + `appearance: none`
- `.stepper-val input` 的 `text-align: right`（line 404）-> `text-align: center`
- input 加 `inputmode="numeric"`（html 改）

**改 `index.html`**：两个 input 加 `inputmode="numeric"`。

**验证**：数字居中；仅一套 -/+ 控件，无原生上下箭头。

---

## 6. 跳过 -> 切换多功能键

**改 `index.html`**：`#btn-skip` -> `#btn-switch`，去掉静态「跳过」文案，改为动态填充（含图标 span + 文案 span）。

**改 `main.ts`**：
- `render()` 内根据 `s.phase` 设置 switch 按钮：
  - 专注态：图标 ☕ + 文案「休息」
  - 休息态（短/长）：图标 ▶ + 文案「专注」
- click handler 复用 `timer_skip` 命令（零后端改动，语义=切阶段+停表+不计完成数）
- 切换后 `render` + `refreshStats`

**验证**：专注态按钮显「休息」，点击后进入休息阶段且停表（Idle）；休息态显「专注」，点击回到工作停表。

---

## 7. 应用图标

**资源**：创建 SVG 源 -> 极简番茄红圆角矩形 + 白色时钟指针（Apple HIG 风格，单焦点无文字）。
**生成**：`bun run tauri icon <src.svg>` 生成全套（32/128/128@2x/ico/icns + Windows Square*Logo/StoreLogo）覆盖 `src-tauri/icons/`。
**改 `tauri.conf.json`**：`productName` `pomodoro-clock` -> 可选改更友好的「番茄钟」；确认 icon 路径。
**验证**：dev 模式看任务栏图标；`bun run tauri build` 后看 exe 图标。

---

## 8. 自动切换开关 UI（设置抽屉内）

- 抽屉加一行 setting-row：「自动开始下一阶段」+ toggle switch（iOS 风格）
- `set_config` 调用时传 `auto_start_next`
- `loadConfig` 读取并同步 toggle 状态
- 验证：关时专注结束停表；开时自动进入休息开跑

---

## 验证总表

| 项 | 验证方式 | 成功标准 |
|---|---|---|
| 1 自动切换 | `cd src-tauri && cargo test` | ≥23 绿 0 ignored |
| 2 动画 | dev 手测 | 窗口平滑插值无瞬跳 |
| 3 抽屉 | dev 手测 + 截图 | 齿轮滑出/收起流畅 |
| 4 拖拽 | dev 手测 | 展开态可拖拽 |
| 5 控件 | 截图 | 数字居中无双箭头 |
| 6 切换 | dev 手测 + cargo test(skip 逻辑) | 文案随阶段变 |
| 7 图标 | dev 看任务栏 | 番茄红图标 |
| 视觉 | capture-round.ps1 + 独立评审 | 连续 2 轮无 P1 |

## 风险与约束
- **不碰**：计时真相（tick/skip 逻辑语义保持）、stats.json 存储、通知机制
- **JS 不写计时逻辑**：自动切换是 Rust 状态机行为，JS 只转发
- **回归基线**：每步后 `cargo test` 保持全绿
- **vite 端口 1430 / CDP 9223**：不改
