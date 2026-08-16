# DEV_LOG

## 2026-08-16 — v14.2 dev×安装版互斥根治（BLOCKED #14 P2 裁决执行，管理者亲执）

```evidence
verify_cmd: cd src-tauri && cargo test; cargo build; cargo build --release; 安装版驻留下双实验：Start-Process target\debug\pomodoro-clock.exe（4s 存活检查）与 Start-Process target\release\pomodoro-clock.exe（4s 自退检查）
verify_claim: 45 passed 0 failed 0 ignored；实验A=安装版 PID 247204 在役下 debug 实例存活 4s 未被顶掉（修前 dev 自退即 P2 现象）；实验B=release 实例 4s 内自退（单实例在 release 仍生效）；聚焦转交未复测（v14/v14.1 已有前台 pid=首实例双证据+前台锁 best-effort 记录）
files_touched: [src-tauri/src/lib.rs, BLOCKED.md, PROGRESS.md, docs/DEV_LOG.md]
```

### 根因与改动（仅 lib.rs builder 链头一处重构）
- 根因：单实例插件按 identifier 判重，dev 与安装版共 identifier → dev 被驻留安装版顶掉（`tauri dev` 自退而 vite 占 1430）；自启开着时每次登录必现。
- 改动：`tauri::Builder::default()` 拆出变量，单实例 `.plugin()` 移入 `#[cfg(not(debug_assertions))]` 链头（保持官方「第一个插件」位序）；debug 不注册 → dev 与安装版共存，release 注册 → 行为零变化。
- 让步如实记：debug 双开无保护（dev 工具可接受）；安装版 1.0.1 无需重装（release 行为不变）；本轮未重产 setup（下次打包自然带上，版本维持 1.0.1 因 release 行为无差）。

## 2026-08-15 — v14.1 托盘化（任务栏关闭死态根治 + 窗口驻托盘）

```evidence
verify_cmd: cd src-tauri && cargo test; bun run tauri build; setup /S 重装后真实输入验证（UIA 任务栏按钮枚举/keybd_event Alt+F4/托盘左键点击 + IsWindowVisible/GetForegroundWindow/Get-Process）
verify_claim: 45 passed 0 failed 0 ignored；1.0.1 静默安装 exit 0 卸载项 DisplayVersion=1.0.1；任务栏番茄钟按钮枚举=空（skip_taskbar 生效）；Alt+F4 后进程存活=True+窗口可见=False（CloseRequested→prevent_close+hide 生效）；托盘左键点击后窗口可见=True（唤回成功）；退出路径不受影响（app.exit 不经 CloseRequested）
files_touched: [src-tauri/src/lib.rs, src-tauri/tauri.conf.json, package.json, src-tauri/Cargo.toml, PROGRESS.md, docs/DEV_LOG.md]
```

### 领导报告的问题与根因
- 现象：任务栏「关闭窗口」后托盘图标仍在，点击无法唤回。
- 根因：主窗 `skip_taskbar(false)` 在任务栏有入口；任务栏关闭**销毁窗体**（进程因托盘驻留），托盘左键 handler 的 `show+focus` 对已销毁窗口无效——死态。

### 改动（仅 lib.rs 两处 + 版本 1.0.1 三处）
1. 主窗 `.skip_taskbar(false)`→`.skip_taskbar(true)`：任务栏无入口，无从销毁；悬浮胶囊+托盘图标照常
2. `on_window_event` 增 `CloseRequested` 臂：`prevent_close()+hide()`——Alt+F4 等一切关闭请求转为隐藏，托盘可唤回；退出仍只走托盘菜单「退出」（`app.exit` 不经 CloseRequested）
3. 版本三处 1.0.0→1.0.1（行为变更，避免两个不同行为的 1.0.0 安装包）

### 验证（1.0.1 安装版实测）
- 任务栏按钮枚举（UIA，宽 >60 过滤）：**空** ✓；托盘图标在 ✓
- Alt+F4（keybd_event 真实按键）：进程存活 ✓ + 窗口隐藏 ✓
- 托盘左键（真实点击）：窗口重新可见 ✓（220x76 迷你态）
- 已知细节（非回归）：其他窗口持前台时 set_focus 不强抢（Windows 前台锁，实测持前台=WindowsTerminal）——胶囊恒置顶可见性不受影响，点击胶囊自然取焦；与评审 P3-2 best-effort 同源，已记 BLOCKED #14

## 2026-08-15 — v14 打包 1.0.0（NSIS 安装包 + 单实例保护）

```evidence
verify_cmd: cd src-tauri && cargo test; bun run tauri build; setup /S 静默安装+安装级真实输入冒烟（Get-Process/GetForegroundWindow/注册表/文件/屏幕直采像素diff）；独立子智能体复核 Rust diff（opus，给 diff+测试输出不给自辩）
verify_claim: cargo test 45 passed 0 failed 0 ignored（含单实例接入后全量重跑两次）；NSIS 产物 番茄钟_1.0.0_x64-setup.exe（2,326,828B，sha256 769C4D5599E4DE573AD7C2F45396C3ED65B16A589959945E0F487CF51F631BB7）构建全程仅 makensis 无 WiX 步骤（bundle/msi 目录空）；安装后卸载项 DisplayVersion=1.0.0、开始菜单+桌面快捷方式指向安装 exe；图标像素级核对=新液态玻璃 B·计时环（exe 内嵌 32×32 帧与 icons/32x32.png 逐字节全等 0/4096，≠icon-old3 旧套）；双开实测进程恒=1 且失焦后二启把前台切回既有主窗（fgPid=首实例 pid）；UI 开=真实点击播放钮后 2.2s 屏采 diff 703px 倒计时在走+⏸+红进度条，停=托盘菜单「暂停」点击后 23:34+▶ 暂停帧；托盘菜单三态文案 开始/暂停/继续 全部实采；config.json 落盘安装目录（领导亲测改材质 classic 持久化）；自启往返=开→HKCU Run「番茄钟」=D:\番茄钟\pomodoro-clock.exe→关→值消失（还原关态）
files_touched: [src-tauri/Cargo.toml, src-tauri/Cargo.toml 内 version, package.json, src-tauri/tauri.conf.json, src-tauri/src/lib.rs, docs/screenshots/v14/*, PROGRESS.md, BLOCKED.md, docs/DEV_LOG.md]
```

### 改动摘要（生产代码仅 4 文件 5 处，计时/统计/UI 零改动）

| # | 项 | 改动 | 说明 |
|---|---|---|---|
| 1 | 版本三处 | tauri.conf.json / src-tauri/Cargo.toml / package.json：0.1.0→1.0.0 | 三处一致；Cargo.lock 随之更新 |
| 2 | 打包目标 | tauri.conf.json bundle.targets："all"→["nsis"] | 不再产 MSI（省 WiX 下载与构建时间）；构建日志确认仅 makensis |
| 3 | 单实例保护 | Cargo.toml +tauri-plugin-single-instance="2"；lib.rs builder 链**第一个插件位**接入 `init(\|app,_argv,_cwd\| { main 窗 show+set_focus })` | 官方文档要求注册为第一个插件；二启由插件转交 argv 给首实例回调后自退——修复双开时第二份全局热键静默注册失败的现状 |

单实例逻辑可测性说明：互斥/退出机制在官方 crate 内部（进程互斥+回调转发），我们的代码只是三行窗口回调（依赖 AppHandle/WebviewWindow 运行时，无纯函数可单测）→ 未加单元测试，以安装版真实双开验证（进程=1+前台聚焦双证据）。这是任务书「加不了在 DEV_LOG 说明」的预设情形。

### 打包产物（任务 2）

- **setup**：`src-tauri\target\release\bundle\nsis\番茄钟_1.0.0_x64-setup.exe`，2,326,828 B，sha256 `769C4D5599E4DE573AD7C2F45396C3ED65B16A589959945E0F487CF51F631BB7`
- 裸 exe：10,684,928 B，sha256 `6A7A3A4F965D81120D73A29D6789F2CE40A60B27E785D0C633FE543247802685`
- 无 WiX：bundle/msi 目录为空（08-13 残留结构）；旧 0.1.0 setup（08-13 产物）仍在 nsis 目录，未清理（无清理授权）
- 图标防旧 .res 复用：打包前核实 build/out 无历史 .res；打包后 ExtractAssociatedIcon 像素级比对——exe 内嵌图标与 v13 液态玻璃源图标**逐字节全等**（md5 524da677…，diff 0/4096），与 icon-old3 旧套（919c11f6…）明确不同

### 安装冒烟（任务 3，本机 per-user，安装目录 D:\番茄钟）

1. 静默安装 `/S` exit 0；卸载项 `HKCU\…\Uninstall\番茄钟` DisplayVersion=**1.0.0**；开始菜单+桌面快捷方式均指向 `D:\番茄钟\pomodoro-clock.exe`
2. **installed exe 与 built exe 3 字节差异**（offset 0x7B1BC8-0x7B1BCA）：tauri NSIS 安装器对已装 exe 打 bundle-type 标记的落点（与构建日志「Patching … with bundle type information: nsis」同机制），非损坏；版本资源 FileVersion/ProductVersion=1.0.0、图标复验仍=新图标
3. **InstallLocation=D:\番茄钟 而非任务书预期的 %LOCALAPPDATA%**：NSIS 记住了本机此前安装 0.1.0 时的路径（注册表记忆），per-user 可写不受影响，config.json/stats.json 落安装目录正常
4. 双开：首启→记事本抢前台→二启→Get-Process 恒=1 + GetForegroundWindow 的 pid=首实例（单实例+聚焦双证据）；首启主窗 220x76 迷你态正常渲染
5. UI 开停（真实输入 mouse_event 通道）：开=播放钮点击→2.2s 屏采 diff 703px（倒计时在走）+⏸ 图标+红进度条（v11 规格行/经典材质下均验证）；停=托盘菜单「暂停」项真实点击→23:34+▶ 暂停帧定格；托盘菜单动态三态文案 开始/暂停/继续 全部实采（=Rust timer_snapshot 现查真相）
6. config.json：冒烟期间由领导亲测改材质 classic 落盘（内容完整含 hotkey 默认段），证明安装目录读写链路通
7. 自启往返：托盘菜单「开机自启」开→HKCU Run「番茄钟」=「D:\番茄钟\pomodoro-clock.exe」（精确安装 exe 路径）→关→值消失（终态还原为关）
8. 证据 9 张归档 `docs/screenshots/v14/`

### 环境/工具（翻车与干预如实记）

- **PrintWindow 对 WebView2 抓的是缓存陈帧**（PW_RENDERFULLCONTENT 也一样）：release 无 CDP 后首选拍窗法连续产出「冻结假象」（t3==t4 像素全等但实际计时在走），一度误导为「计时器冻结」产品缺陷——CopyFromScreen 屏幕直采裁决（同一窗口 2.2s diff 2818px 证明在走）。教训：**release 构建验收禁用 PrintWindow 判活，一律 CopyFromScreen**
- PowerShell 工具每次调用新进程（已知坑复踩）：Add-Type 跨调用不存活导致一轮焦点断言失效，重跑补证
- **首启 tray_menu_fit 竞态（P3，BLOCKED 留档）**：安装后首启（及一次重现实测）托盘菜单窗被 fit 到 136×116（正常 104×116）：菜单内容盒实测仍 104 正常、右侧 32px 为窗外透明区（透桌面壁纸，视觉不可见），重启后实测恢复 104——fit 在页面加载早期一次性测量，首跑 WebView2 布局热身期偶发偏宽且不重测。界限内不修，留领导裁决
- **外部干预三次（神秘 toggle）**：冒烟期间计时器在无输入窗口期被外部 toggle 三次（暂停→恢复→再暂停方向各异）；每次均为单次翻转且菜单文案与实际状态恒自洽——事后领导确认是其本人在实测。机理上安装版接管 Ctrl+Alt+O 全局键（默认 Ctrl+Alt+P 本机被占降级），本机任何会话/人发 ^%o 都会触发，属预期行为非缺陷
- dev 重启命令：`bun run tauri dev`。**注意**：安装版与 dev 共 identifier，单实例插件使二者互斥——dev 启动前须先退安装版（托盘退出或 Stop-Process），否则 dev 会作为第二实例聚焦安装版后自退
- NSIS 工具链已缓存（%LOCALAPPDATA%\tauri\NSIS），未触发代理下载路径

### 独立评审（opus 子智能体，给 diff+测试输出不给自辩）

**PASS（无 P1）**。已验证非 findings 的正确性要点：插件确为 Builder 首个 .plugin（第二实例在 setup 含热键注册/stats 加载之前自退——顺带消灭 stats.json 双进程写竞态）；回调与既有托盘左键唤起主窗语义逐行同构；tray-menu 窗不应聚焦（常驻 hidden 瞬态弹窗）；argv/cwd 忽略恰当（无 deep-link/文件关联/CLI 消费者）；三处版本一致、Cargo.lock 已锁 2.4.3、回滚无锁死。
- **P2-1**（dev×安装版互斥被自启放大：自启驻留后 `tauri dev` 自退但 vite 占 1430）→ 界限内不改码，BLOCKED #14 留档二选一（接受限制 / 下轮 `#[cfg(not(debug_assertions))]` 一行修）
- P3-1 MSI→NSIS 迁移残留（旧 MSI 机器先卸净再装）/ P3-2 最小化唤起 best-effort（与托盘左键同病，若修两处一起）/ P3-3 无签名预期 / P3-5 可选 double-launch 冒烟脚本——全记 BLOCKED #14
- P3-4（CLAUDE.md 死规矩「无安装包」需破例注记+基线 35 过时）→ **已当场修**：定位段补 v14 破例注记（比照 v10 自启先例）、测试基线 35→45

### 留裁（见 BLOCKED #14）

首启 fit 竞态（P3 不修留档）；dev 互斥+自启放大（P2-1 待裁决）；SmartScreen 首启「仍要运行」属无签名预期（任务书已声明非缺陷）；旧 0.1.0 setup 产物未清理。

## 2026-08-15 — v13 液态玻璃菜单区分度（光影移植：一眼可辨液态 vs 经典）

```evidence
verify_cmd: cd src-tauri && cargo test; pwsh scripts\v13-menus.ps1（8 单图+4 并排+ghost×2+几何 JSON 一次采全，起始/终态自动校验还原）; pwsh scripts\v11-tray.ps1 -Step popup; CDP eval mini 右键沉默断言; 评审 round-44/45（opus 独立子智能体×2，像素级 RGB 采样自证）
verify_claim: 45 passed 0 failed 0 ignored（纯 CSS 改动）；两菜单 offsetWidth=104、项高 27、明暗×两材质八组合 rect 逐字段全等（v13-geo.json：应用内 104×89@170,200 项 27×3 / 托盘 104×116 项 27×4，=v12 基线）；popup PASS（104×116@(1545,773) 工作区内贴图标上方）；mini 右键 menuOpen:false；透字 ghost 像素 diff 深档 avg1.77/max5、浅档贴边 12 像素 Δ=10 无字形结构、菜单内部黑白衬底同行亮度差 ≤2 L；浅档末项对比度 13.5:1；round-44 无 P1（P3-1 浅档弱→立修）+ round-45 无 P1 无 P2 = 连续两轮无 P1 收敛（封顶 4 用 2）
files_touched: [src/styles.css, scripts/v13-menus.ps1, docs/review/round-44.md, docs/review/round-45.md, docs/screenshots/v13/*, PROGRESS.md, BLOCKED.md, docs/DEV_LOG.md]
```

### 改动摘要（生产代码仅 styles.css 三个规则块，经典档零改动）

| # | 项 | 改动 | 关键数值 |
|---|---|---|---|
| 1 | 浅色液态菜单 | `[data-material="liquid_glass"] .ctx-menu` 重写 | 校准①实测首版白 sheen 落近白底不可见（ΔRGB 仅 6~7）→ 改走「冷灰底霜+底部暗重」：多档 sheen + 底霜 247→241（同面板 236 家族）+ 底部暗带梯度；校准②（round-44 P3）底带 0.06→0.09/起点 55%→50%、底部内阴影 0.16→0.2，摆幅拉至 ~25 与深档同量级；顶高光 0.9 + 左缘折光 0.35 + 内发光；v8⑫/R34 深色发丝边线 0.16 纪律保留；底色 alpha 0.95 防透字线不动 |
| 2 | 深色液态菜单 | `[data-theme="dark"][data-material="liquid_glass"] .ctx-menu` 重写 | 对标深色 .app：多档 sheen（0.2→0.08→0.03→0.09）+ 顶高光 0.28→0.38 + 左缘折光 + 底部内阴影 0.45 + 内发光；底色 56→60 再提半档做玻璃分层提亮（v8 R33 提亮纪律方向延续），alpha 0.97 不动、发丝边线 0.22 不动 |
| 3 | 托盘页液态签名 | 托盘页覆盖段补两个液态专属 box-shadow 规则 | 根因：托盘页原规则把 box-shadow 整棵杀成发丝边线（外阴影裁角教训），液态顶高光/底厚度随之陪葬 → inset 光影窗内渲染不裁角，按主规则同款补齐（外阴影仍禁），注释声明两处同步维护 |

### 任务 2/3 数值自证（全部 CDP/像素实测）

- 可辨度（评审像素裁定）：浅档液态 顶 252→中 244→底带 229-237 + 冷色调（B-R≈+4）vs 经典通体平直 254；深档液态 顶沿 89-99→中段 66-70→底回弹 74-78 vs 经典死平 56-58
- 无透字：ghost 盖纯黑|纯白——深档逐像素 diff avg 1.77/max 5（≥10 零个）；浅档 avg 4.67/max 10（仅贴边 12 像素、12x 放大复查均匀浅灰无字形）；菜单内部横跨黑白衬底同行亮度差 ≤2 L
- 几何：八组合 rect 全等（104×89 项 27×3 / 104×116 项 27×4），v8 立法照守（材质段零几何属性）
- 回归：popup PASS；mini 右键沉默；cargo test 45 绿 0 ignored

### 评审循环（round-44~45，2 轮收敛）

- round-44（opus）：无 P1 无 P2；P3-1 浅档辨识度弱于深档（摆幅 19 vs 23）→ 立修（底带/底影加强）+重采——该建议直指任务核心指标（领导场景是切换非并排）
- round-45（opus 收敛复验）：无 P1 无 P2 无新增 P3；P3-1 专项判定「到位且未过火」（底带纯净冷色调、退出 13.5:1、属可辨与发脏间正确平衡点，不建议继续加）→ **连续两轮无 P1 收敛**

### 环境/工具（翻车与干预如实记）

- 新管线 scripts/v13-menus.ps1：v12 脚本扩展——托盘侧补材质维度（ui-style 广播带 material）、几何 JSON 随采集落盘、ghost 高对比图案注入+RGB 采样+对比度拉伸一站完成、并排对比图自动拼（左经典右液态 2x nearest）
- **首跑 hover 事故**：Open-TrayMenu 采集后 SetCursorPos(400,400) 归位 → 四张托盘图「开机自启」全被钉 hover 蓝底。v12 同流程不归位零残留。机理未完全查明（SetCursorPos 无 WM_MOUSEMOVE 但疑似触发 WebView2 重命中），修法=弹出后在图标原位做 mouse_event 相对 +1px 真实微移生成 WM_MOUSEMOVE 重命中清除（复跑全净）
- **次跑脚本事故**：mouse_event P/Invoke 签名 dx 是 DWORD，-1 直接 ArgException 中断于托盘段前（染料未及状态污染：手动 tray_menu_hide + dataset 还原）；负相对位移应传 0xFFFFFFFF，本次改单次 +1px 规避
- **外部干预迹象**：三次采集起跑实测 DOM 态分别为 dark/classic → dark/liquid → light/classic；config.json material 期间被外部两改（采集期 liquid_glass → 收口时又翻回 classic）——符合 BLOCKED 长效教训「本机有并行会话外部干预」特征；终态按「material=config 持久值」纪律对齐 classic（DOM+托盘 ui-style 广播同步），theme=当次开工实测值
- 证据卫生声明：v11-tray.ps1 -Step popup 回归重跑覆盖 docs/screenshots/v11/traymenu-popup.png（同名新帧，沿用 v11/v12 同口径）；v13 证据独立成档 docs/screenshots/v13/
- 采集改进项（round-45 留档，非 finding）：ghost 测试字形未整字压菜单中心，下轮 ghost 采集 glyph 对准菜单几何中心补齐构图

## 2026-08-15 — v12 两套右键菜单留白统一（贴内容 / 不折行 / 左右平衡）

```evidence
verify_cmd: cd src-tauri && cargo test; CDP eval 四组合实测（主窗展开+真 contextmenu 派发 / 托盘页 location.reload 触发 tray_menu_fit）；pwsh scripts\v11-tray.ps1 -Step popup; pwsh scripts\v8-geometry.ps1 + 逐键对比（容差 0.5px）; pwsh scripts\v61-regression.ps1; pwsh scripts\v12-menus.ps1; 评审 round-42/43（opus 独立子智能体×2）
verify_claim: 45 passed 0 failed 0 ignored（纯 CSS 改动，menu_origin 6 定位单测断言零改动随套过）；两菜单 offsetWidth 相等=104；每项 offsetHeight 27 单行（不再 38）；算术自洽 104=textW 52(开机自启@13px)+24 左槽+20 右 padding+8 菜单内边距；明暗×两材质四组合 rect 全等（应用内 104×89 项 96×27 / 托盘 104×116 项 96×27，v8-geometry 4/4 IDENTICAL×39 元素）；popup PASS（104×116@(1545,773) 工作区内贴图标上方）；mini 右键沉默/展开态右键正常/Esc 整收/失焦 ~158ms 不回归；round-42/43 连续两轮无 P1 收敛（封顶 4 用 2，43 轮无 P2 无 P3）
files_touched: [src/styles.css, scripts/v12-menus.ps1, docs/review/round-42.md, docs/review/round-43.md, docs/review/round-42-*.png, docs/review/r42-zoom*.png, docs/review/r43/, docs/screenshots/v12/*, PROGRESS.md, BLOCKED.md, docs/DEV_LOG.md]
```

### 改动摘要（生产代码仅 styles.css 两处规则）

| # | 项 | 改动 | 关键数值 |
|---|---|---|---|
| 1 | 永不折行 | `.ctx-item` 加 `white-space: nowrap` | macOS 菜单铁律；两菜单同一 CSS 同时生效 |
| 2 | 宽度贴内容 | `.ctx-menu` 加 `width: max-content`（min-width:76 下限保留兜单项） | 破托盘页 160 自证循环：static 菜单不再撑视口，tray-menu.ts getBoundingClientRect 量出真实内容宽回告 tray_menu_fit（回告链零改动）；窗口 160→104 |
| 3 | 左右平衡 | `.ctx-item` padding `5px 12px 5px 24px` → `5px 20px 5px 24px` | 右 padding 12→20；评审 round-42 实测左 27–30 vs 右 24–26（含边线）判平衡，定稿 20（任务书 18–20 区间上限档），round-43 独立复验支持 |

### 任务 2 数值自证（CDP 实测，修复前 → 修复后）

- 托盘菜单：160×116（右死白 ~64px，视口自证循环）→ **104×116**，四项 96×27 单行
- 应用内菜单：96×89（右 padding 12 偏窄）→ **104×89**，三项 96×27 单行
- 同宽：两菜单 offsetWidth 均 **104**（最长项同为「开机自启」textW 52）
- 单行：各项 offsetHeight **27**（≈27–28 区间，无双行 38）
- 算术自洽：52+24+20+8=**104**=菜单宽 ✓
- 四组合 rect 全等：light/classic、light/liquid_glass、dark/classic、dark/liquid_glass 下菜单 rect 逐项全同；`[data-material]` 作用域零几何属性（v8 立法照守，v8-geometry 4/4 IDENTICAL 复证）

### 评审循环（round-42~43，2 轮收敛）

- round-42（opus）：无 P1 无 P2；P3-1 建议项=右留白维持 ≈24 总量级（20+4），不建议 18（27:18≈1.5 超 macOS 原生比例带 1.2–1.4）→ 采纳定稿 20（代码已是，零改动）
- round-43（opus 收敛复验）：无 P1 无 P2 无 P3；像素级复测两菜单宽均 104、左 27/右 25、行距 27、圆角 10–11px 一致；透字拉伸检验无鬼影；周边波及扫描无异常 → **连续两轮无 P1 收敛**

### 环境/工具

- 探针先行：`box-sizing:border-box` 下 `width:max-content` 在 WebView2（Chromium）中 border-box=内容 max-content+padding（探针 52+44=96 实测），算术自洽有理论底座
- 任务书「应用内菜单 min-width 76 压窄折行（textW 25/高 123）」在终码上未复现——实测修复前应用内 96×89 单行（fixed 定位 shrink-to-fit 已贴内容）；折行风险由 nowrap 硬保证，不留口子
- 新管线 scripts/v12-menus.ps1：六态一次采全（应用内 4 组合真 contextmenu 派发 + 托盘明暗真实右键），起始/终态自动校验还原；踩坑：cdp-seq eval 返回 JSON.stringify 字符串是双重编码（ConvertFrom-Json 得裸串）——return 对象才单层（v11 已录的坑复踩一次，本次已修进脚本注释）
- 证据卫生声明：v11-tray.ps1 -Step popup 回归重跑覆盖了 docs/screenshots/v11/traymenu-popup.png（旧 160px→新 104px 同名）；v12 证据独立成档于 docs/screenshots/v12/ 与 docs/review/round-42-*

## 2026-08-15 — v11 亲验四项（mini 右键取消 / 托盘自绘菜单 / 快捷键改键 / 胶囊进度条）

```evidence
verify_cmd: cd src-tauri && cargo test; bunx tsc; bun run build; pwsh scripts\v11-progress.ps1; pwsh scripts\v11-tray.ps1 -Step popup/dark/items/check/blur/esc/quit; pwsh scripts\v11-realinput.ps1; pwsh scripts\v11-capture.ps1 -Round 40; WScript SendKeys ^%u/^%o/^%p 链路 + CDP get_hotkey/timer_snapshot; pwsh scripts\v61-regression.ps1; pwsh scripts\v8-geometry.ps1; pwsh scripts\v9-blur.ps1
verify_claim: 45 passed 0 failed 0 ignored（35 基线+4 改键+6 定位）；mini 右键无响应+展开态正常双证据；托盘弹层 POPUP（160x116 工作区内贴图标上方）/ITEMS（动态文案）/CHECK（注册表双向）/BLUR/ESC/QUIT 六 PASS + REAL-INPUT PASS（真实 hover+点击可达）；改键全链（录 U 落盘 SendKeys 双向翻转、占用 P 回退保留旧键红帽提示、× 关闭键死、重启保持、删键回默认 ^%o 复活）；进度条三态 bar↔snapshot 逐 tick 一致+暂停冻结；PANEL_H 467 几何 4/4 IDENTICAL×39 元素；round-40/41 连续两轮无 P1 收敛；v61 四项+v9-blur 回归 PASS
files_touched: [src-tauri/src/appconfig.rs, src-tauri/src/lib.rs, src-tauri/capabilities/tray-menu.json, src/main.ts, src/bridge.ts, src/tray-menu.ts, index.html, tray-menu.html, src/styles.css, vite.config.ts, scripts/cdp.mjs, scripts/cdp-seq.mjs, scripts/v11-progress.ps1, scripts/v11-tray.ps1, scripts/v11-realinput.ps1, scripts/v11-capture.ps1, docs/review/round-40.md, docs/review/round-41.md, docs/review/round-40-*.png, docs/screenshots/v11/*, docs/solutions/platform-integration/tauri-webview2-second-window-invalid-state.md, docs/solutions/workflow-issues/transparent-webview2-automation-pitfalls.md, PROGRESS.md, BLOCKED.md, docs/DEV_LOG.md]
```

### 改动摘要

| # | 项 | 改动文件 | 关键变更 |
|---|---|---|---|
| 1 | mini 右键取消 | `main.ts` | contextmenu 处理器加 `if (!expanded) return;`——菜单(89px)比迷你窗(76px)高物理裁切无解，入口=展开态右键+托盘 |
| 2 | 胶囊 2px 进度条 | `index.html`, `styles.css`, `main.ts` | pill 底部轨道（左右 26px inset=底边弦长，小进度不被圆角裁没）；render 复用面板同一 done（snapshot remaining/total，暂停即停）；颜色跟 var(--work/--break/--long)；几何全在基础段（材质立法零差异） |
| 3 | 快捷键改键 | `appconfig.rs`, `lib.rs`, `main.ts`, `index.html`, `styles.css`, `bridge.ts` | HotkeyConfig{enabled,shortcut}（serde default 旧配置不崩）+validate（可解析+≥1 修饰键）+4 测试；set_hotkey 先注册新键再卸旧键=占用回退保旧键；启动读配置注册（P 占降级 O）；抽屉单行无标题卡（v8 拍板先例）：键帽点击即录（capture 阶段拦键，Esc 取消/Delete 关闭/修饰键+主键提交）、× 一键关闭、红帽行内错误提示 2.2s 自恢复；PANEL_H 456→467（+11≤+16 预算，drawer 行 9→7+caption 6→4 回收） |
| 4 | 托盘自绘菜单 | `lib.rs`, `tray-menu.html`, `tray-menu.ts`, `styles.css`, `bridge.ts`, `main.ts`, `vite.config.ts`, `capabilities/tray-menu.json` | 弃 muda 原生菜单；hidden 常驻第二 WebviewWindow 复用 ctx-menu 苹果风；右键托盘图标贴图标弹出（traymenu::menu_origin 纯函数四边任务栏+边缘钳制，6 单测）；四项现查 snapshot/注册表；失焦/Esc 关、再点关；焦点语义（主窗让焦给菜单不收、菜单关且主窗未聚焦→整收补全瞬态）；ui-style 广播对表明暗/材质跟随；tray_menu_fit 自适配窗尺寸；vite 多页构建 |
| 5 | P2-1 修复 | `styles.css`, `tray-menu.ts` | 托盘页 focus/:focus-visible outline:none + opened 时 blur 残留焦点（根因：UA focus-visible 环+CDP 合成 hover 跨 hide/show 残留双因，长命实例自动化污染产物） |

### 评审循环（round-40~41，2 轮收敛）

- round-40（opus）：无 P1；P2-1 托盘首项 hover 描边环（浅黑/深白 ~2px，Windows focus visual 观感）→ 立修+重采；其余判据（托盘一致性/抽屉新行密度/进度条融合/常规扫描）全成立
- round-41（opus）：无 P1 无 P2（P2-1 明暗两档复验通过、全量复核无漏网）→ **连续两轮无 P1 收敛**（封顶 6 轮用 2 轮）；P3-1 档案卫生（autostart-on 旧帧带环影）→ 重采替换

### 环境/工具

- **根因级**：WebView2 同 user data folder 第二窗口 `additional_browser_args` 必须与主窗逐字一致，否则 ERROR_INVALID_STATE(0x8007139F) 被 tauri-runtime-wry 吞掉、窗口假注册句柄永不创建；参数一致→共用浏览器进程→第二页直接进 9223 CDP 目标列表。已立法注释（lib.rs 菜单窗创建块）+沉淀 docs/solutions/platform-integration/
- **自动化四陷阱**（沉淀 docs/solutions/workflow-issues/）：①SetCursorPos 不产生 WM_MOUSEMOVE（真实 hover 要 mouse_event 相对微移）②CDP 合成 :hover 跨 hide/show 残留无 Web API 可清（采集前 move 1 1 归位或重启）③cdp-seq eval 双重 JSON 编码（IIFE return 对象单层、return JSON.stringify 双层）④GetWindowTextW 必须 CharSet.Unicode 否则中文标题乱码；附 pwsh 嵌套 SendKeys 双引号 $w 提前展开坑
- v11-tray.ps1 自踩：EnumWindows 泛型委托 non-blittable 不可封送（须具名 delegate）、cdp-seq 无 click 命令（首跑 QUIT FAIL 系脚本 bug 非产品缺陷，换 cdp.mjs click 后 PASS）
- **领导并行亲验纪律成型**：领导 8-14 晚~8-15 午持续用 app（自启开关/材质 classic/短休 5min/自录 Ctrl+P/跑番茄）——采集脚本全程备份-恢复+stats 哈希比对，零次 clobber 用户配置；计时器运行期间只跑非破坏性步骤，quit/capture 等 idle 监控触发后才跑
- 工具增量：cdp.mjs/cdp-seq.mjs 支持 `CDP_MATCH=tray-menu` 双窗锁目标（默认主窗）；scripts/v11-{progress,tray,realinput,capture}.ps1 四条新管线

## 2026-08-14 — v10 顺手功能五件套（托盘/自启/全局快捷键/统计升级/多屏模拟）

```evidence
verify_cmd: cd src-tauri && cargo test; pwsh scripts\v10-tray.ps1 -Step icon/focus/menurun/quitkeys; reg query HKCU\...\Run /v 番茄钟（正反向）; WScript SendKeys ^%o + CDP timer_snapshot; pwsh scripts\v10-capture.ps1 -Round 38/39; pwsh scripts\v10-multimon.ps1; pwsh scripts\v9-blur.ps1; pwsh scripts\v8-geometry.ps1
verify_claim: 35 passed 0 failed 0 ignored（+5 月聚合测试）；托盘 FOCUS PASS + idle「开始」/running「暂停」动态菜单截图 + QUIT PASS（Get-Process 查无）；自启注册表正反向输出齐；Ctrl+Alt+P 被占降级 Ctrl+Alt+O 真实触发、SendKeys 实测 status 双向翻转；统计四截图（个数周/分钟周/当月/上月月历）；MULTIMON PASS（出屏三态钳回工作区、边缘拖拽跟到 OS 钳位点、up 即停）；round-38/39 连续两轮无 P1 收敛；失焦即收 ~159ms PASS；材质几何 4/4 IDENTICAL
files_touched: [src-tauri/Cargo.toml, src-tauri/src/lib.rs, src-tauri/src/stats.rs, src/bridge.ts, src/main.ts, index.html, src/styles.css, scripts/v10-tray.ps1, scripts/v10-capture.ps1, scripts/v10-multimon.ps1, docs/review/round-38.md, docs/review/round-39.md, docs/review/round-38-*.png, docs/review/round-39-*.png, docs/screenshots/v10/*, PROGRESS.md, BLOCKED.md, docs/DEV_LOG.md]
```

### 改动摘要

| # | 项 | 改动文件 | 关键变更 |
|---|---|---|---|
| 1 | 系统托盘 | `lib.rs`, `Cargo.toml` | tray-icon feature + TrayIconBuilder（icon.ico→32x32.png→default_window_icon 链）；三菜单：开始/暂停（tick 循环侦测 status 变化 set_text 动态文案）+ 设置…（emit open-settings）+ 退出（app.exit(0)）；左键 Up show+set_focus；show_menu_on_left_click(false) |
| 2 | 开机自启 | `lib.rs`, `main.ts`, `index.html` | tauri-plugin-autostart（HKCU Run，值名 productName「番茄钟」，默认关）；get/set_autostart 命令返回 is_enabled 真实态；窗口右键菜单加「开机自启」（menuitemcheckbox，每次开菜单现查）+「退出」 |
| 3 | 全局快捷键 | `lib.rs`, `Cargo.toml` | tauri-plugin-global-shortcut：Ctrl+Alt+P 切换开始/暂停（toggle_run 与前端同语义）；注册失败 eprintln 降级 Ctrl+Alt+O，再失败只记日志不崩——本机 P 被占，降级路径真实触发 |
| 4 | 统计升级 | `stats.rs`, `lib.rs`, `bridge.ts`, `main.ts`, `index.html`, `styles.css` | Rust `Stats::month(year,month)->MonthSummary`（first_weekday 周一=0 偏移 + 每日 DayBar，越界钳 12 月）+5 测试；前端单位 seg（个数/分钟，同作用周柱与月格）+ 范围 seg（周/月）+ 月历热力格（level 0-4 番茄红梯度、today 描边、‹ › 翻历史 ›当月 disabled）；mock 通道 stats_month 确定性伪数据；[hidden] 补刀（class display 覆盖） |
| 5 | 多屏/DPI | `lib.rs`, `scripts/v10-multimon.ps1` | drag 立修：GRAB 改存 CSS 坐标 f64 bits，跟随线程每帧 GetDpiForWindow 现算物理偏移（跨屏 DPI 漂移修复）；morph 审查确认健壮（DEFAULTTONEAREST+i32 负坐标+clamp 兜底）；v10-multimon.ps1 四场景实测 PASS |

### 评审循环（round-38~39，2 轮收敛）

- round-38（sonnet）：无 P1；P2-1 月历占位格不可见 → 修 `.month-pad` track 填充；level-0 对比/翻页钮颜色两小项同修；首采污染事故按 R33 先例收起重采
- round-39（haiku）：无 P1 无 P2，P2-1 复验通过（像素级网格扫描+RGB 采样，今天描边/红梯度/连续天数全自洽）；P3-1 垫格对比立修 0.16→0.35（首试 0.20 因 --track 自带 alpha 0.16 双重衰减纹丝不动，实测 RGB 248→245 达标），P3-2/3/4 裁决不修 → **连续两轮无 P1 收敛**（封顶 6 轮用 2 轮）

### 环境/工具

- **v10-tray.ps1 两坑定型**：UIA 搜「番茄钟」Button 同中任务栏按钮(84x48)与溢出面板图标(40x40)→按宽度 ≤60 过滤；「^ 显示隐藏的图标」是 toggle→先查面板是否已开再点，3 轮循环兜底
- **UIA 枚举不到 Tauri 托盘菜单项** → 键盘赛道 {DOWN}/{UP}+{ENTER}（右键点经 UIA 确证面板开着+图标命中，非盲注；{DOWN}{ENTER}=「开始」与 {UP}{ENTER}=「退出」均实证）
- **评审员配额**：opus/sonnet 先后 403（kimi 计费周期），haiku 槽位完成 R39——多槽探测顺序记入 skill 经验
- 真双屏物理实测留 BLOCKED（需设备条件）；月历扁条造型维持（已裁决）



- **采集管线收编进 `scripts/`**（原 job tmp 随会话清理，资产防丢）：`v7-env.ps1`（枢纽，新增 `%TEMP%\pomo-shots` 中间帧目录）、`cdp-seq.mjs`、`v61-capture.ps1`、`v61-regression.ps1`、`v8-geometry.ps1`、`v9-hover.mjs`/`v9-capture.ps1`/`v9-blur.ps1`；硬编码路径全改（grep 零残留），冒烟 `v9-blur.ps1` PASS（~166ms）。cdp-seq 与 cdp.mjs 未合并（eval 输出格式不同，合并风险 > 重复成本）
- **crop-tool skill 增强**：`--nearest`（像素级审判禁 LANCZOS——插值糊掉重采样证据）、`--zoom N`、`--bg COLOR`（透明帧合成色，深色主题必需）、`--batch jobs.json`（批量裁剪，单条失败退出码 1）；三模式冒烟全过
- **新 skill `pomodoro-visual-review`**（`.claude/skills/`）：v6.1~v9 四轮评审循环沉淀——采集管线选择表、opus 独立评审员提示词三条纪律、P1/P2/P3 分诊口径（采集事故严格计数重起）、连续 2 轮无 P1/封顶 6 轮收敛、归档链路；项目 CLAUDE.md 评审约定行已指向该 skill
- **errors.md 回灌**：`New-Object Type(表达式)` 参数模式不求值坑（Harness/配置主题下）

## 2026-08-14 — v9 按钮悬停文字发毛根治

```evidence
verify_cmd: grep 断言五选择器 hover/active 无 scale/transform; pwsh tmp/v9-capture.ps1 -Tag pre / -Tag r36 -Dark / -Tag r37 -Dark; cd src-tauri && cargo test; pwsh tmp/v9-blur.ps1
verify_claim: 五处（含 .icon-btn:active 共六条规则）零 scale/transform；修复前 mid1 帧「开始」发毛有重影 vs 修复后 mid1/mid2/end 三帧同等锐利（round-36-pre vs round-36-r36 对照图）；30 passed 0 failed 0 ignored；失焦即收 ~164ms 收至 220x76 PASS
files_touched: [src/styles.css, docs/review/round-36.md, docs/review/round-37.md, docs/review/round-36-*.png, PROGRESS.md, BLOCKED.md, docs/DEV_LOG.md]
```

### 改动摘要

| # | 项 | 改动文件 | 关键变更 |
|---|---|---|---|
| 1 | 五处 transform 缩放病灶 | `styles.css` | 根因：hover/active 的 scale() 缩放合成层位图→文字重采样发毛，动画结束重新光栅化→清晰。修法：`.btn`/`.icon-btn`/`.collapse-btn` hover+active、`.step-btn` active 全部去 transform；transition 清单同步去 transform。任务书未列的 `.icon-btn:active scale(0.92)` 属同款病灶，按「按压同治」拍板一并治 |
| 2 | 明暗反馈方案 | `styles.css` | hover/active 改 `filter: brightness()`——不缩放合成层、文字零重采样、天然兼容液态档渐变背景（免逐材质 hover 覆盖）。浅色 hover 微沉 0.96/0.94、深色提亮 1.15~1.22（macOS 惯例方向）；按压=快速变暗 0.82~0.88（active 内 transition-duration 0.06s）；`.step-btn` 按压用 `--input-bg`（比 hover 的 `--separator` 深一档，明暗两主题自适应） |

### 评审循环（round-36~37，2 轮收敛）

- round-36：无 P1 无 P2；判据 A（中途帧锐利）全样本通过、判据 B（反馈克制可感知）通过；P3×2 不修（btn-switch 锯齿=13px vs 14px 固有渲染差异且 pre 帧同在；reset 浅色 hover delta=macOS 克制幅度）
- round-37：无 P1 无 P2（代码零变更重采集复审）→ **连续两轮无 P1 收敛**（封顶 4 轮用 2 轮）；P3 collapse 浅色 hover 静帧不可辨按同口径留档

### 环境/工具

- 新管线 `tmp/v9-hover.mjs`：单 WS 会话真实悬停/按压定时帧（+40/+120/+700ms），按压帧移开再松手不触发 click（不改计时状态）；`tmp/v9-capture.ps1` 驱动+4x/5x 最近邻裁切放大
- 教训：PowerShell `New-Object Type($w*$zoom,...)` 括号内是参数模式不做乘法求值（报 Object[]→UInt32），GDI 构造一律 `::new()`
- 保护项未碰：ctx-menu 入场 scale、抽屉 translateX、toggle 旋钮、morph（拍板 3）

## 2026-08-14 — v8 材质几何漂移根治 + 全部留裁清理


```evidence
verify_cmd: cd src-tauri && cargo test; bunx tsc --noEmit; pwsh tmp/v8-geometry.ps1（两档材质 rect 对比）; pwsh tmp/v61-regression.ps1; CDP 真实点击材质往返 + 重启 dev 验证保持
verify_claim: 30 passed 0 failed 0 ignored; tsc clean; 4 态×37 元素 rect 0 diff（头差 ≤0.5px 容差内全等）; v7 回归四 PASS（宽容点击/Esc 整收 220x76/失焦 ~176ms/拖拽 leave 后续跟 100px）; classic↔liquid_glass 点击落盘往返正确，重启后 DOM/config 均 liquid_glass
files_touched: [src/styles.css, src/main.ts, CLAUDE.md, PROGRESS.md, BLOCKED.md, docs/DEV_LOG.md, docs/review/round-32~35（md+png）]
```

### 改动摘要

| # | 项 | 改动文件 | 关键变更 |
|---|---|---|---|
| 1 | P0 材质几何漂移 | `styles.css` + `CLAUDE.md` | 根因：液态档 `.drawer .section-caption` padding-top 6px vs 经典 2px，抽屉 space-evenly 均分→全控件重排。修法：几何统一（6px 上提到材质无关的 `.drawer .section-caption` 规则）+ **立法**：材质作用域只许 background/box-shadow/color/border-color/backdrop-filter，禁改一切几何属性（写进 styles.css 段头注释 + CLAUDE.md 约定）。验收：CDP 实测 4 态（面板/抽屉/菜单/迷你）×37 元素 rect 两档全等 |
| 2 | 清单 P2 七项 | `styles.css` + `main.ts` | ①进度 ≤2% 不渲染圆头（main.ts 阈值，空轨道=未开始态）②迷你暂停钮明暗统一为同一四层配方（填充+顶高光+0.5px 描边+轻投影）③浅色「专注」红字用文字专用深红 --work-text #d70015 ④Switch 关态 --input-bg 0.14→0.18 三层明度拉开 ⑤深色分段：未选中文字 #c7c7cc + 选中 pill #636366 + 发丝边线 ⑥无数据日满高灰轨道→基线 4px 短划（.week-track.empty）⑦有数据日去灰底纯红柱（.week-track.has-data），消除「两段式=渲染未完成」 |
| 3 | 清单 P3 八项 | `styles.css` | ⑧菜单 min-width 132→76 贴内容 ⑨深色迷你胶囊顶部高光专属降亮配方 ⑩外观组两分段统一 136px 等宽 ⑪本周标题 +8/周图 −4 节奏均衡 ⑫浅色菜单深色发丝边线（R34 再加强 0.16）⑬胶囊左右 inset 统一 16px ⑭圆点空心态 opacity 0.6→0.85 ⑮收起钮 30px 圆钮→44×22 把手胶囊（sheet handle 语义，评审两轮通过） |
| 4 | 圆点「进行中」中间态（领导拍板） | `main.ts` + `styles.css` | 当前个=半填充（linear-gradient 左半实填+工作色描边），完成=全亮，未到=空心，长休=全亮；索引全部由 Rust snapshot（phase+completed_work_count）推导，前端不自计数。R32 评审 P2「仅靠颜色」→ R33 起改半填充形状级差异 |
| 5 | 不修关闭（领导拍板） | `BLOCKED.md` | #7-1 迷你胶囊不加展开箭头（胶囊即按钮既有语义）；#5-4 迷你态菜单压计时数字（76px 窗高物理不可避，瞬态）；#5-6 单行分组卡加组标题（R29 刚收紧 42px 高度预算，加行必再溢出） |

### 评审循环（round-32~35，4 轮收敛）

- round-32：无 P1；P2=深色空态短杠太暗（--track 0.2→0.36，浅色同步 0.16）+ 圆点中间态只靠颜色（→半填充）；P3 深色选中 pill 发丝边线立修
- round-33：P1=浅色 mini 截图采错（采集事故：toggle-off 证据脚本留展开态残尾，非产品缺陷，补采+尺寸自检 340×196）；P2=深色菜单 dark-on-dark 层级弱（底色 42→56 提亮+边线 0.22）
- round-34：无 P1 无 P2；P3 浅色菜单边线 0.12→0.16 立修；P3 深色红字/圆点辨别余量裁决不修（Apple 官方深色 systemRed / 评审自评无误读）
- round-35：无 P1 无 P2；P3 省略号疑点证伪（源码实测 U+2026 单字符）→ **连续两轮无 P1 收敛**（封顶 6 轮内用 4 轮）

### 环境/工具

- 新管线 `tmp/v8-geometry.ps1`：两档材质 4 态 ×37 元素 getBoundingClientRect 采集+对比（几何不变性验收的标准工具）
- 教训：证据补拍脚本必须恢复窗口起始状态（收起态），否则污染下一轮采集首帧（R33 P1 采集事故根因）
- BLOCKED #0/#5/#7/#8 全部勾销：15 项已修（证据=round-32~35 截图）、3 项拍板不修、长效环境教训浓缩保留

## 2026-08-13/14 — v6.1 液态玻璃改版

```evidence
verify_cmd: cd src-tauri && cargo test; bunx tsc --noEmit; pwsh tmp/v61-regression.ps1; 重启 dev 后 CDP get_config
verify_claim: 30 passed 0 failed 0 ignored; tsc clean; v7 回归四项 PASS（宽容点击/Esc 整收/失焦 ~156ms/拖拽 leave 后续跟 100px）；重启后 material=liquid_glass 保持
files_touched: [src-tauri/src/appconfig.rs, src-tauri/src/lib.rs, src/bridge.ts, src/main.ts, index.html, src/styles.css, docs/liquid-glass-spec.md]
```

### 改动摘要

| # | 项 | 改动文件 | 关键变更 |
|---|---|---|---|
| 1 | 官方 spec | `docs/liquid-glass-spec.md` | 两个子智能体抓苹果官方 JSON 文档 + WWDC25-219/356 转录：变体语义/35%暗层/克制原则/同心圆角/动效/可访问性退化，每条带出处；数值仅官方给出的（模糊半径等明确标注「官方未给」） |
| 2 | 材质配置管道 | `appconfig.rs`（新）+ `lib.rs` | `Material` 枚举（classic/liquid_glass，默认 liquid_glass）+ `AppConfig`（flatten TimerConfig + material）接管 config.json 读写——旧 `TimerConfig::save` 直写会丢 material 键；新增 `set_material` 命令，`get_config` 改返 AppConfig；timer.rs/stats.rs 零改动；新增 4 测试（roundtrip/legacy 无 material 键回默认/损坏回默认/snake_case 序列化）→ 30 绿 |
| 3 | 前端设置项 | `index.html` + `main.ts` + `bridge.ts` | 外观组加「材质」分段（经典/液态玻璃，复用分段样式新增 .material-opt 类避免与主题事件串扰）；`applyMaterial` 切 `data-material` 属性 + `set_material` 落盘；mock bridge 同步补 material 字段与 set_material 分支 |
| 4 | 液态玻璃 CSS | `styles.css` | 全部玻璃样式收在 `[data-material="liquid_glass"]` 作用域，经典=既有样式零改动；功能层（外壳/菜单/抽屉）分层渐变+顶部镜面高光+边缘光+内阴影；内容层卡片比外壳更白/更亮拉开层级（避免 glass-on-glass）；抽屉/菜单全实底防盖面板透字；prefers-reduced-transparency / prefers-contrast 退化（WWDC25-219 官方语义：frostier / 对比描边） |
| 5 | 评审循环修复 | `styles.css` | R28 深色主键失红（权重+源码序）→ 深色域补 .btn-primary；R29 抽屉分组卡裁行（flex-shrink + overflow:hidden + min-height:auto=0 陷阱）→ 行 padding 12→9、抽屉 padding/gap/头距回收 64px；R30 浅色步进器/次级按钮白上白 → iOS tertiary 灰填充 |

### 评审循环（round-27~31，5 轮收敛）

- round-27：P1=右键菜单未采到（采集事故：eval 顶层 const 全局残留报重声明被吞，IIFE 修）→ P2 浅色分组卡/节标题间距立修
- round-28：P1=深色主键失红（真 bug）→ 修；P1「开关裁切」经 crop+几何实测=误读（白旋钮融白卡，顺手加发丝描边）
- round-29：P1=抽屉分组卡裁断开关/长休息行（真 bug，v6 起隐性存在）→ 修
- round-30：无 P1（P2 步进器灰填充立修）
- round-31：无 P1 → **连续两轮收敛**；P2/P3 留裁 → BLOCKED #8

### 环境/工具

- 评审采集新管线 `tmp/v61-capture.ps1`：四界面×明暗×材质 8 态一轮，CDP 离屏 shot + 底色合成
- 旧 dev 后台任务（bun run tauri dev）跨日后死亡：vite 1430 孤儿残留 → 整树杀后重启（任务 bgdf8fl0c）
- HIG JSON 正确前缀 `/tutorials/data/design/human-interface-guidelines/<slug>.json`（documentation/ 前缀 404）

## 2026-08-12 下午 — UI 优化 7 项

```evidence
verify_cmd: cd src-tauri && cargo test
verify_claim: 22 passed; 0 failed; 0 ignored
files_touched: [src-tauri/src/timer.rs, src-tauri/src/lib.rs, src/bridge.ts, src/main.ts, index.html, src/styles.css, src-tauri/tauri.conf.json, src-tauri/app-icon.svg, src-tauri/icons/*]
```

### 改动摘要

| # | 项 | 改动文件 | 关键变更 |
|---|---|---|---|
| 1 | 自动切换开关 | `timer.rs` | `TimerConfig.auto_start_next: bool` 默认 false；`tick()` 结束分支按开关决定 Running/Idle；新增 `work_completion_stops_when_auto_start_disabled` 测试（22 绿） |
| 2 | 窗口动画 | `lib.rs` | 新增 `anim` 模块：ease-out cubic 逐帧 SetWindowPos，~220ms，代际令牌 AtomicU64 防重入；`animate_size` 命令注册 |
| 3 | 抽屉 | `index.html` + `styles.css` + `main.ts` | 设置区移入 `.drawer`，覆盖式从右滑入（`translateX`）；齿轮按钮 `.gear-btn` 绝对定位右上角 |
| 4 | 展开拖拽 | `main.ts` + `index.html` | 提取 `attachThresholdDrag` 函数，绑 `#pill` + `#panel-head`；删失效 `data-tauri-drag-region` |
| 5 | 控件居中 | `styles.css` + `index.html` | `stepper-val input`：`text-align:center` + 隐藏原生 spinner（`-webkit-appearance:none`）；input 加 `inputmode="numeric"` |
| 6 | 跳过→切换 | `main.ts` + `index.html` | `#btn-skip` → `#btn-switch` 动态填充；`render()` 内专注态「☕休息」/休息态「▶专注」；复用 `timer_skip` 零后端改动 |
| 7 | 图标 | `app-icon.svg` + `tauri.conf.json` | SVG 源（番茄红#ff3b30 圆角矩形+白色时钟指针）→ `tauri icon` 生成全套；`productName` → 番茄钟 |

### 新增配置

- `TimerConfig.auto_start_next: bool` — 阶段结束后是否自动开跑下一阶段，默认 false（停表等手动开始）
- 设置抽屉 toggle 开关同步此配置

### 新增 Rust 模块

- `anim` — 窗口尺寸动画（ease-out cubic，~220ms，代际令牌防重入），仅 `#[cfg(windows)]`

### 待验证

- 视觉评审：`scripts/capture-round.ps1 -Round 11` → 独立评审子智能体 → 修 P1 → 连续 2 轮无 P1 达标
## 2026-08-12 晚 · 领导 4 项 P1 修复 + 评审循环收口

| # | 项 | 触及文件 | 要点 |
|---|---|---|---|
| 1 | 面板留白 | `main.ts` + `styles.css` | PANEL_H 600→441（CDP 实测内容底 426.2+padding 14）；底部留白=14px；mock-preview 同步 |
| 2 | 动画单源 | `lib.rs` + `styles.css` | 删双层打架：Rust 窗口动画 220→340ms ease-out quart（单调无回跳）；CSS 删 transform spring 只留 opacity 淡入淡出；border-radius 同步 340ms 同曲线；逐帧证据 docs/review/anim/sizes.txt（双向单调）+ 连点 10 次无残影 |
| 3 | 设置键 | `index.html` + `styles.css` | 齿轮移出 panel-head → 新 `.panel-foot` 底栏居左（收起居中、同宽 spacer 平衡）；功能零改动 |
| 4 | 图标 | `app-icon.svg` + `icons/` | 3 候选手写 SVG（A 玻璃高光/B 果实纵深/C 极简徽章）→ 独立评审盲选 A → 高光降 0.26 → `tauri icon` 全套重生成；旧图标备份 docs/review/icon-old/；新增 `scripts/render-icon.mjs`（CDP 光栅化） |
| 5 | 评审循环 | `docs/review/round-1{2,3}.md` | round-12 无 P1（进度轨道对比度立修）→ round-13 无 P1，连续 2 轮收敛（round-15 封顶内） |

### 环境坑（新）
- `powershell`(5.1) 误读无 BOM 的 win.ps1 中文注释报解析错 → 用 pwsh 7 跑 capture 脚本
- 沙箱内 localhost 被代理 502/拒绝 → CDP/curl 本机操作需关沙箱

### 新增工具
- `scripts/capture-anim.ps1`（动画逐帧取证）、`scripts/capture-screencast.mjs`（CDP screencast）、`scripts/render-icon.mjs`（SVG→PNG）

### 结果
cargo test 22 绿 skipped=0；4 项前后对比 round-11 vs round-13；评审连续 2 轮无 P1 达标

## 2026-08-13 · 领导亲验 4 项返工

- 动画重做：Rust 逐帧 SetWindowPos 动画（50fps+锚左上角+class先行）→ 窗口瞬时 morph_begin/morph_commit + 卡片 WAAPI morph（60fps 单调、就地形变、交叉淡入、可打断）；anim 模块删除
- 切换键去 emoji 纯文字；设置入口改自绘右键菜单（齿轮删除、拖拽限定左键）；图标 Liquid Glass 重绘（盲选 B，tauri icon 全套）
- 评审循环 round 15-20：关键平台定论=透明 WebView2 backdrop-filter 糊不到桌面，材料改实心；round-19/20 连续无 P1 收敛
- cargo test 22 绿 skipped=0；证据 docs/review/{anim15,ctx,icon2-candidates,round-15..20.*}

## 2026-08-13 图标重设计（领导选定 B·计时环）
- 4 候选（番茄果实/计时环/番茄切面/25分钟）经领导过目选定 B；app-icon.svg 重写（保留玻璃底工艺，glyph 换 3/4 进度环）
- render-icon.mjs 光栅化 + tauri icon 全套重生成；旧玻璃版备份 docs/review/icon-old3-glass/
- 坑（修正版）：换图标后 exe 图标不更新——tauri-build 的 build.rs 不监视 icons/，链接器复用缓存的旧 .res，touch main.rs 强制重链也无效（实锤：ExtractAssociatedIcon 提出的还是初代扁平图标）；正解=cargo clean -p pomodoro-clock 清掉 build script 缓存再全量构建；另：杀进程重启时注意先杀后启竞态会留孤儿实例（桌面出现两个窗口）

## 2026-08-13 长休息功能暴露（设置两项 + 循环指示器）

- Rust：set_config 加 long_break_min/long_break_every（every 钳 ≥1 防 mod-0 panic）；新增 config.json 持久化（exe 同目录，TimerConfig load/save 仿 stats.rs，#[serde(default)] 兼容缺字段，损坏回默认不崩）；TimerSnapshot 加 long_break_every（指示器数据源，前端零自计数）
- 前端：抽屉「时长」组加长休息步进器（1-60 分钟），新「循环」组=长休间隔步进器（2-10 个）+自动开始开关，「外观」独立成组；面板计时区下方循环圆点（完成一个亮一个、长休全亮、点数随间隔变）
- 评审：round-22（1 P1=采集 artifact、2 P2 立修）→ round-23/24 连续两轮无 P1 收敛（round-24 封顶轮内）；残留 P2/P3 记 BLOCKED #0
- 实测：间隔改 2 连打 2 番茄自动进长休（60s 整链日志）；重启后设置保持（IPC+UI 双路）；三状态两主题截图 docs/screenshots/cycle-*
- 坑：config 此前毫无持久化（goal 假设有误，按验收条件补建）；并行 Claude 会话会干预运行中 app（异常先排查外部）；杀 dev 要 taskkill /T 整树否则 1430 残留；PANEL_H 441→456
- cargo test 26 绿 skipped=0（新增 4：config roundtrip/缺失/损坏 + 间隔可调触发长休）

## 2026-08-13 v7 瞬态面板（失焦即收 / Esc / 宽容点击）

- 交互对齐苹果瞬态面板：Rust on_window_event(Focused(false)) → emit window-blurred → 前端 collapsePanel（关右键菜单+复位抽屉+setExpanded(false)，mini 态忽略）；Esc 同链路整收；底部收起键保留
- 宽容点击：拖拽启动阈值 6→12px；松手位移<10px 且按下<500ms 一律算点击展开（即便中途触发过拖拽，接受微拖误展开取舍）
- 顺手修拖拽冻结：旧 mouseleave→endNativeDrag 在快拖时光标跑出窗口即杀线程；改为 mouseleave 只清点击判定，Rust 拖拽线程 GetAsyncKeyState 物理松键兜底自终止（1500ms 宽限防误杀 CDP 合成拖拽），mousedown 重置陈旧 dragging 态
- 验收：手抖点击 10/10、30px 拖拽不展开、leave 后续拖 200px 不冻结、快速交替 10 次终态干净、切窗/Alt+Tab 失焦 1s 内收起、mini 失焦无变化、抽屉失焦收+再展开无残留、morph 中途失焦无残影、真实 Esc（SendKeys）收起
- 环境坑：双 WT 全屏置顶→真实点击不可达，验证改 CDP dispatch+SetCursorPos；SendInput 键盘被拦用 WScript SendKeys；SetForegroundWindow 需 AttachThreadInput+重试；cargo test 26 绿 skipped=0
