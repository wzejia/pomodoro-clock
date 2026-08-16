# BLOCKED.md（待裁决清单）

## 14. v14 留档（2026-08-15，一项待裁决）

- **首启 tray_menu_fit 竞态（P3，建议不修留档）**：安装后首启（及复现一次）托盘菜单窗被 fit 到 136×116（正常 104×116）——fit 在页面加载早期一次性测量，首跑 WebView2 布局热身期偶发偏宽且无重测机制。影响仅限首会话窗外 32px 透明区（透桌面壁纸，菜单内容盒实测仍 104 正常、视觉不可见），重启实测自愈。若领导要求修：fit 改为 open 时现量现 fit（与动态文案现查同思路），但涉及生产代码改动超出 v14 界限，未动
- dev 与安装版共存：~~共 identifier + 单实例插件 ⇒ 互斥~~ → **v14.2 已根治（2026-08-16）**：单实例插件 `#[cfg(not(debug_assertions))]` 仅 release 注册，dev 可与安装版并行；注意两者热键仍互斥（同键只有先驻留者注册成功，第二份静默降级失败属预期）
- ~~【评审 P2-1】自启会放大 dev 互斥~~ → **已修（2026-08-16 管理者亲执，领导裁决「现在修」）**：同上 cfg 修复；实测安装版驻留下 debug 实例共存 4s 存活（修前自退）、release 双开仍自退；45 绿。安装版 1.0.1 无需重装（release 行为零变化）
- 【评审 P3-1】MSI→NSIS 迁移残留：曾以 0.1.0 MSI 装过的机器，NSIS 1.0.0 不卸旧版，双 exe 并存+Run 值指旧 exe 时会被单实例聚焦到旧实例（「升级了没变」假象）——自用机装 1.0.0 前先卸净旧版
- 【评审 P3-2】唤起为 best-effort：主窗经任务栏最小化后 show+focus 无 unminimize（与托盘左键路径同病）——若未来出现「最小化唤不回」反馈，托盘左键与单实例回调两处一起补
- 【评审 P3-3】无签名：异机运行触发 SmartScreen 属预期（任务书已声明），对外分发才需签名
- 【评审 P3-5 可选】单实例目前人工实测证据，可按 scripts/ 取证风格补 double-launch 冒烟脚本防插件升级回归（未做，无授权）
- 旧 0.1.0 setup（08-13 产物）留在 bundle/nsis/ 未清理（无清理授权）
- SmartScreen 首启「仍要运行」=无签名预期（任务书明示非缺陷，不做签名）
- 冒烟期间计时器三次外部 toggle：领导本人实测确认（安装版接管 Ctrl+Alt+O 全局键后，任何 ^%o 都会触发——本机多会话环境用该键自动化时需知悉）

## 13. evidence 块 Stop hook（evidence_guard.py）——方案待拍板，未注册未写脚本（2026-08-15）

**空白**：双通道存证里 `commit_guard.py` 挂在 `git commit` 上，本项目（及一切无 git 项目）通道一 guard **完全空转**——DEV_LOG evidence 块纯靠 AI 自觉（v13 实写了，但无机制兜底）。

**方案（影子模式起步，同 commit_guard 惯例）**：
- 脚本：`~/.claude/hooks/evidence_guard.py`（Python，<100ms，读 stdin hook JSON）
- 判定：①从通道二 `~/.claude/evidence/events-当日.jsonl` 过滤本项目 `src/**`、`src-tauri/src/**` 的 Edit/Write 事件（生产代码白名单口径，比宪法「非文档」更严——docs/*.md/scripts/memory/.claude 不计）②`grep` 本项目 `docs/DEV_LOG.md` 当日日期条目下有无 `verify_cmd:`（evidence 块特征行）③**有编辑无块** → stderr 警告 + exit 0（影子）；env `HARNESS_GUARD_ENFORCE=1` → exit 2 阻断提示补块
- 误报面（口径已内置豁免）：当日多轮微调一块即过（v13 两轮校准先例，不逐 Edit 对应）；纯文档会话直接过；events jsonl 缺损 → fail-open 记 WARN 跳过（与 evidence_log 定位一致：防偷懒不防对抗）；jsonl 无 session 维度 → 当日口径，同日跨项目混跑互相豁免（可接受）
- 注册（拍板后执行）：①备份 `settings.json` → `~/.claude/config-backups/settings-YYYYMMDD-evidence-guard.json`（元规矩）②`hooks.Stop` += `{"hooks":[{"type":"command","command":"<python.exe 绝对路径> C:\\Users\\Administrator\\.claude\\hooks\\evidence_guard.py"}]}`
- 转正：影子跑 ≥1 周零误报 → env 加 `HARNESS_GUARD_ENFORCE=1`（同 commit_guard）；届时 CLAUDE.md「已注册 guard hooks」段才补声明（元规则：注册行可指出才许写「已生效」）

## 12. v13 留档（2026-08-15，无待裁决项）

- 浅档液态配方两次校准定稿：白 sheen 落近白底不可见（ΔRGB 6~7）→「冷灰底霜 241 + 底部暗带 0.09 + 底影 0.2」；round-45 专项判「可辨与发脏间正确平衡点，不建议继续加」——后续若再想加强浅档，先读 round-44/45 档案
- 托盘页液态签名补位根因留档：托盘页覆盖段曾把 box-shadow 整棵杀成发丝边线（外阴影裁角教训），液态 inset 光影随之陪葬——inset 不裁角可保留，外阴影仍禁；两处规则注释声明同步维护
- 采集教训两条（已修进 scripts/v13-menus.ps1 注释）：①托盘菜单采集后 SetCursorPos 归位会钉 hover（改 mouse_event +1px 真微移重命中）②mouse_event dx 是 DWORD，负相对位移传 0xFFFFFFFF
- ghost 采集改进项（round-45 留档）：测试字形未整字压菜单中心，下轮 ghost 采集 glyph 对准菜单几何中心
- 领导配置实况：config.json material 在 v13 期间被外部两改（采集期=liquid_glass、hotkey Ctrl+O；收口时又翻回 **classic**）——旧记录 classic/Ctrl+P 早已过时；主题不持久化。v13 采集期三次开工 DOM 态互异（dark/classic→dark/liquid→light/classic），并行会话外部干预特征；终态已按「material=config 持久值」纪律对齐为 classic（DOM+托盘广播同步）

## 11. v12 留档（2026-08-15，无待裁决项）

- 右留白定档裁决：右 padding 12→**20**（+菜单内边距 4 ≈ 24 总量级）——round-42 评审实测左槽 27–30 vs 右 24–26 判平衡，建议维持；18 不建议（27:18≈1.5 超 macOS 原生比例带 1.2–1.4，右缘贴投影显局促）；round-43 独立复验支持。任务书「18–20 区间评审对 HIG 定稿」→ 定稿上限档 20
- 任务书现状一处与终码不符：「应用内菜单折行 textW 25/高 123」在终码未复现（实测修复前 96×89 单行，fixed shrink-to-fit 已贴内容）——nowrap 硬保证已加，不留折行口子
- 托盘菜单窗宽 160→104（tray_menu_fit 量真实内容宽）；窗口初始 inner_size(160,124)（lib.rs:629）维持不动——仅 hidden 启动一瞬，fit 后即正确
- 领导配置全程未动（classic/浅色/Ctrl+P/短休 5min；dataset 翻档采集后已还原，stats.json 未碰）

## 10. v11 留档（2026-08-15，无待裁决项）

- ~~迷你态右键无菜单~~ → v11 已按拍板执行：mini 右键无响应（菜单 89px vs 窗 76px 物理裁切无解），入口=展开态右键+托盘自绘菜单
- ~~托盘原生 muda 菜单观感~~ → v11 已换自绘小窗（复用 ctx-menu 苹果风、四边任务栏定位、失焦/Esc 关）
- 「再右键=关」toggle 不可测限制：溢出面板图标场景下重开面板的左键先触发失焦关（等效同为关），仅任务栏直显图标场景可达——已知限制，非缺陷
- 托盘菜单真实输入可达性：v11-realinput.ps1 REAL-INPUT PASS 已证（修复员一度存疑，实证推翻）；领导亲验时物理鼠标点一次菜单项作最终确认（建议项，非阻塞）
- 领导当前生效配置（2026-08-15 亲验中自设，勿动）：材质 classic、短休 5min、快捷键自录 Ctrl+P、自启开关以其注册表为准
- 教训已沉淀：docs/solutions/platform-integration/（WebView2 第二窗 browser args 一致性）+ workflow-issues/（自动化四陷阱）

## 9. v9 评审 P3 留档（2026-08-14 全部裁决不修，无待裁决项）

- ~~btn-switch「休息」glyph 锯齿~~ → 不修：`.switch-label` 13px vs 其他按钮 14px 的固有渲染差异，pre 帧（旧方案结束态）同样存在证明与变换无关，1x 物理尺寸正常
- ~~浅色 btn-reset hover 明暗 delta 偏小~~ → 不修：两轮评审均自评「不视为缺陷」，macOS 克制幅度，加大有闪烁感风险
- ~~浅色 collapse hover 静帧明度几乎不可辨~~ → 不修：同上口径，静止帧不可辨≠动态不可感知
- 环境教训（已入 DEV_LOG）：PowerShell `New-Object Type($w*$zoom,...)` 括号参数模式不做乘法求值 → GDI 构造一律 `::new()`

## ~~8. v6.1 液态玻璃残留~~（2026-08-14 v8 全部闭环）

- ~~进度条圆头 0% 凸出/读作旋钮~~ → 已修：进度 ≤2% 不渲染填充（含 min-width 圆头），round-32+ 面板空轨道为证
- ~~迷你暂停钮明暗构件规格差~~ → 已修：明暗统一同一四层配方（填充+顶高光+0.5px 描边+轻投影）
- ~~右键菜单过宽~~ → 已修：min-width 132→76 贴内容
- ~~深色迷你胶囊顶部高光偏亮~~ → 已修：.app.mini 深色专属降亮配方
- ~~外观组主题/材质分段宽度不一致~~ → 已修：统一 136px 等宽（rect 实测 136/136）
- ~~按钮行与「统计」标题间距~~ → 已修：本周标题 +8、周图 −4，节奏均衡
- ~~浅色右键菜单无边线~~ → 已修：深色发丝边线 0.5px（R34 加强至 0.16）
- 环境教训已沉淀：eval 一律 IIFE（见下「长效环境教训」）；flex-shrink 裁切陷阱已沉淀 docs/solutions/ui-bugs/settings-drawer-flex-shrink-clipping.md

## ~~7. v7 瞬态面板残留~~（2026-08-14 v8 全部闭环）

- ~~迷你胶囊无「可展开」affordance~~ → **不修关闭（领导拍板）**：迷你胶囊不加展开箭头，胶囊即按钮是既有语义
- ~~今日柱两段式误读~~ → 已修：有数据日去灰底纯红柱（.week-track.has-data）
- ~~深色未来日柱对比低~~ → 已修：空态短划（.week-track.empty）+ R32 深色短划提亮 0.36
- ~~进度圆头 ≈0 错位~~ → 已修：≤2% 不渲染（与 #8 圆头项同源同修）
- ~~浅色胶囊红色「专注」对比弱~~ → 已修：文字专用深红 --work-text #d70015
- ~~收起 chevron 与主操作区关联弱~~ → 已修：30px 圆钮→44×22 把手胶囊（sheet handle 语义），评审 round-34/35 两轮通过

## ~~0. 长休息功能残留~~（2026-08-14 v8 全部闭环）

- ~~循环圆点「当前个」无中间态~~ → 已做（领导拍板）：半填充（左半实填+工作色描边）；R32 评审要求形状级差异后定稿；数据仍全走 Rust snapshot
- ~~深色分段控件未选中对比+选中 pill 明度差~~ → 已修：未选中 #c7c7cc、pill #636366 + 发丝边线
- ~~迷你胶囊左右光学留白不对称~~ → 已修：左右 inset 统一 16px
- ~~圆点空心态权重偏低~~ → 已修：opacity 0.6→0.85

## 5. R20 残留 P2/P3 —— v8 处置（2026-08-14 全部闭环）

- ~~浅色 Switch 关态三层明度过近~~ → 已修：--input-bg 0.14→0.18（证据 round-32-toggle-off.png）
- ~~进度条刚起步孤立红圆点~~ → 已修：≤2% 不渲染
- ~~深色分段选中 pill 明度差小~~ → 已修（同 #0-2）
- ~~迷你态右键菜单压计时数字~~ → **不修关闭（领导拍板）**：76px 窗高物理不可避，瞬态
- ~~本周图未来日满高灰轨道误读~~ → 已修：空态短划（同 #7-3）
- ~~单行分组卡无组标题~~ → **不修关闭（领导拍板）**：R29 刚收紧 42px 高度预算，加标题行必再溢出（抽屉 368px 预算余量仅 ~9px）

## 长效环境教训（2026-08-15 单源化退役，细节移 docs/solutions/）

> 唯一权威源=**docs/solutions/**（`index.md` 检索）；skill（pomodoro-visual-review）坑速查只留高频七条+同源指针。本节不再堆细节——复制即快照，快照必过期；新增教训直接 ce-compound 进 solutions。对应关系：注入/hover/编码/P-Invoke/全屏置顶组合拳/圆角穿透 → `workflow-issues/transparent-webview2-automation-pitfalls.md`；托盘 UIA 取证 → `windows-tray-automation-uia-forensics.md`；并行会话干预与采集纪律 → `user-parallel-capture-discipline.md`；活体计时相位 → `living-timer-capture-phase-race.md`；采集恢复起始态（R33 事故）→ CONCEPTS.md「采集管线」条。

## 1. 系统通知「横幅」截图无法在本机捕获（功能已实现并 API 验证）

- 现象：阶段完成时 `app.notification().builder().show()` 返回 `Ok(())`（dev.log 两行 `[notify] show result: Ok(())` 为证），但屏幕右下/通知中心均无横幅。
- 根因：**系统级通知总开关关闭**——注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\PushNotifications` 的 `ToastEnabled = 0`；且任务栏铃铛显示免打扰。对照实验：用 PowerShell 自带 AUMID 发 WinRT toast 同样无横幅、不进通知中心（`docs/screenshots/ps-toast-test.png`），证明非应用问题。
- 未做处置：按「不动系统设置」的界限，没有替用户打开通知开关。
- 待裁决：领导在「设置 > 系统 > 通知」打开总开关后亲验一次横幅（应用内已实现：完成工作番茄→「专注完成 🍅」；连 4 个→「完成一组番茄 🎉」；休息结束→「休息结束」）。

## 2. 「苹果级」验收属半托项

- 评审循环收敛记录：round-9/10（修复前骨架）、round-12/13（2026-08-12 领导 4 项后）、**round-19/20（2026-08-13 领导亲验 4 项返工后，封顶轮）**均连续两轮独立评审「无 P1」（原文见 docs/review/round-*.md）。
- 机器判不了的最终质感，等领导亲验。

## 3. 顺手功能（按规矩不做的清单，留档备裁）——2026-08-14 v10 已做 4.5 项

- ~~系统托盘图标与托盘菜单（退出/暂停入口）~~ → v10 已做：TrayIconBuilder 三项菜单（开始/暂停动态文案 + 设置… + 退出真退进程），左键聚焦；证据 docs/screenshots/v10/（图标/聚焦 FOCUS PASS/明暗菜单/QUIT PASS）
- ~~开机自启动~~ → v10 已做：tauri-plugin-autostart 写 HKCU Run（值名「番茄钟」），默认关；窗口右键菜单勾选态跟随真实注册状态；正反向注册表证据齐
- ~~全局快捷键（开始/暂停）~~ → v10 已做：Ctrl+Alt+P，占用时降级 Ctrl+Alt+O（本机 P 被占，降级路径真实触发，SendKeys 实测 status 双向翻转）
- ~~周统计「个数/分钟」切换、历史月份视图~~ → v10 已做：单位切换同作用周柱+月格；月历热力格带 ‹ › 翻历史（› 当月禁用）；注入历史数据验翻页后已恢复真实 stats.json
- 多显示器/DPI 变动的拖拽边界：v10 已做模拟实测（出屏钳制/负坐标/边缘拖拽全 PASS，docs 证据 %TEMP%\pomo-shots\v10-multimon.json + scripts/v10-multimon.ps1）+ drag 模块立修（跨屏 DPI 每帧现算）。**留裁项：真双屏物理实测**（本机单屏，需设备条件）
- v10 让步记录：托盘图标位于任务栏溢出区系 Windows 默认行为（非缺陷）；月历数据格扁条造型维持（与周柱同语言，裁决不修）；R38 评审员 opus 配额 403 降级 sonnet

## 4. ~~切换键 ☕/▶ emoji 风格~~（2026-08-13 已裁决执行：方案 B 纯文字）

- 领导裁决「去图标纯文字」，已落地：切换键纯文字「休息/专注」，与「开始/重置」成体系；独立评审确认通过。此条撤销。

## ~~5. R20 残留 P2/P3 清单~~（2026-08-14 v8 全部闭环，逐项处置见文首「5. R20 残留 —— v8 处置」）

## 6. ~~图标 exe 嵌入~~（2026-08-13 已解决）

- Liquid Glass 新图标已生成全套（src-tauri/icons/，源 app-icon.svg，旧套备份 docs/review/icon-old2/），新旧 32/128 hash 均不同。
- dev 热重建链已自动全量重嵌（dev 日志：icon.ico 变更触发 rebuild + exe 重启），当前运行中的 dev 实例已是新图标。
- 备注：dev 后台托管进程已退出，但 vite（[::1]:1430）与 pomodoro-clock.exe（CDP 9223）均在运行，亲验可直接用；若要干净的受管实例，重开 `bun run tauri dev` 即可。
