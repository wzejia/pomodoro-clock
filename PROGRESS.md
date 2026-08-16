# PROGRESS

- 状态：**v14.2 dev×安装版互斥根治（2026-08-16，管理者亲执）**：单实例插件改 `#[cfg(not(debug_assertions))]` 仅 release 注册——实测 dev 与驻留安装版共存（4s 存活，修前自退）、release 双开仍自退；45 绿 0 ignored。安装版 1.0.1 在跑（D:\番茄钟，release 行为零变化无需重装）；dev 现可直接 `bun run tauri dev`（热键同键仍互斥属预期，见 BLOCKED #14）。
- 前情：v14.1 托盘化收口（2026-08-15）——任务栏关闭死态根治，1.0.1 实测全过；v14 打包 1.0.0（NSIS+单实例，明卷 6 条+评审 PASS 无 P1）。1.0.1 setup=`src-tauri\target\release\bundle\nsis\番茄钟_1.0.1_x64-setup.exe`。

## 2026-08-15 · v14 终验记录（明卷 6 条）
1. **cargo test ✅ 45 绿 0 failed 0 ignored**（任务 1 后与收口前各跑一次；单实例逻辑在官方 crate 内无可单测纯函数，以真实双开验证——DEV_LOG 已说明）
2. **NSIS 产物 ✅**：`src-tauri\target\release\bundle\nsis\番茄钟_1.0.0_x64-setup.exe` 2,326,828 B sha256 `769C4D55…31BB7`；裸 exe 10,684,928 B sha256 `6A7A3A4F…02685`；构建仅 makensis 无 WiX；三处版本 1.0.0 一致（tauri.conf.json:4 / Cargo.toml:3 / package.json:4）
3. **图标核对 ✅**：exe 内嵌 32×32 帧与 icons/32x32.png（v13 液态玻璃 B·计时环）逐字节全等（diff 0/4096），≠icon-old3 旧套；无历史 .res 复用（打包前核空）
4. **静默安装+卸载项 ✅**：setup /S exit 0；`HKCU\…\Uninstall\番茄钟` DisplayVersion=1.0.0；开始菜单+桌面快捷方式→D:\番茄钟\pomodoro-clock.exe；InstallLocation=D:\番茄钟（NSIS 记忆旧 0.1.0 路径，非 LocalAppData——per-user 可写不受影响，如实记）；installed exe 与 built exe 仅 3 字节差（bundle-type 标记落点，非损坏）
5. **单实例 ✅**：记事本抢前台后二启→Get-Process 恒=1（二启自退）+GetForegroundWindow pid=首实例（既有窗聚焦）双证据；dev/安装版共 identifier 互斥（BLOCKED #14 留档）
6. **冒烟全过 ✅**：UI 开（屏采 2.2s diff 703px 倒计时+⏸+红条）/停（托盘「暂停」点击→23:34 ▶ 定格帧）；托盘菜单弹出+三态文案 开始/暂停/继续 全实采；config.json 落安装目录（领导亲测 classic 持久化）；自启往返 Run 值=安装 exe 路径→还原关态
- 评审：无 UI 变更不进截图评审循环；Rust diff（单实例+版本）独立子智能体复核（opus，diff+测试输出、无自辩）——**PASS 无 P1**；P2-1 dev×安装版互斥被自启放大（自启驻留后 tauri dev 自退但 vite 占 1430）→ 界限内不改码，BLOCKED #14 留档二选一；P3-4 CLAUDE.md 死规矩「无安装包」过时+基线 35 过时 → 已当场补 v14 破例注记+基线 45（比照 v10 自启先例）；P3-1/2/3/5 记 BLOCKED #14
- 翻车/让步如实记：①PrintWindow 对 WebView2 抓缓存陈帧致「计时冻结」假象一轮（CopyFromScreen 屏采裁决，教训已记 DEV_LOG：release 验收禁用 PrintWindow 判活）②首启 tray_menu_fit 136 竞态两次实测（内容盒恒 104 正常+32px 透明区不可见+重启自愈，P3 不修→BLOCKED #14）③冒烟期计时器三次外部 toggle（领导本人实测，事后确认）④PowerShell 新进程 Add-Type 不存活复踩（已知坑）⑤单实例加不了单测（官方 crate 内部机制，DEV_LOG 说明）
- 无回滚项，结果不差于基线（45 绿=基线；v7-v13 功能安装版全通）
- dev 实例：已按任务书退净（pomodoro-clock 91820 + vite 树 61928），验后未重启（领导正用安装版；重启命令与互斥注意事项在 DEV_LOG/BLOCKED #14）

## 2026-08-15 · v14 开工回执（任务 0）
- 目标：NSIS 安装包 1.0.0 + 单实例保护（二启聚焦既有主窗后自退），安装级冒烟全过；dev 实例现存（vite 1430/CDP 9223），冒烟前退净、验完记 dev 重启命令
- 基线：cargo test 45 绿 0 ignored（v13 收口态，本轮重验）；版本三处现状 0.1.0 已核实一致（tauri.conf.json/Cargo.toml/package.json）；bundle targets 现状 "all"→改 ["nsis"]；single-instance 未接入（Cargo.toml 无此依赖，builder 链 lib.rs:576 起）
- BLOCKED 核实：#13 evidence hook/#1 通知横幅/#3 真双屏均与本任务无关，无冲突项
- 顺序：任务1 三处 1.0.0+tauri-plugin-single-instance（官方要求注册为**第一个**插件；回调=show+set_focus 主窗）→ cargo test → 任务2 bun run tauri build（防旧 .res 复用，图标核对）→ 任务3 静默安装+冒烟（快捷方式/卸载项 1.0.0/双开单实例/开停/config 落盘/自启注册表往返还原关态）→ 独立子智能体复核 Rust diff
- 最大风险：①NSIS 工具链首下走代理 7897，卡则设 HTTPS_PROXY 重试、败则记 BLOCKED ②dev 与安装版共 identifier——单实例插件会互相挡，冒烟前必须退净 dev ③本机 Ctrl+Alt+P 被占→安装版首启降级 Ctrl+Alt+O 属预期
- 单实例可测性预判：互斥/退出逻辑在官方 crate 内，我们仅回调三行（show/focus）无可单测纯函数→加不了单测，用冒烟真双开验证，DEV_LOG 说明

- 状态：**v13 收口 + 全会话知识回灌与流程改进五件套（2026-08-15）**：v13 液态菜单区分度已收口（详见下方 v13 终验记录）；同日完成 v11/v13 回灌审计（项目 solutions 8 档+索引+CONCEPTS 增「像素校准/相位归一」）与流程改进 1-5：①v13-menus.ps1 染料段 try/finally 保底还原+config 现读+8 张 4x zoom 评审预置图（验证跑 PASS，含一处双重编码 WARN 修复）②坑速查三要素立法（机制/正解/反例）+高频七条表③环境教训单源化（BLOCKED 教训节指针化退役→solutions 唯一权威源；pitfalls 补 §7§8）④`setx PYTHONUTF8 1` 用户级根治 GBK 坑（新进程生效）⑤evidence guard Stop hook 方案落 BLOCKED #13 待拍板（未注册）。dev 实例在跑（vite 1430 / CDP 9223 双页）。

## 2026-08-15 · v13 终验记录（明卷 5 条）
1. **cargo test ✅ 45 绿 0 failed 0 ignored**（纯 CSS 改动，Rust 零动随套过）
2. **8 张并排对比截图+透字检验图 ✅**（docs/screenshots/v13/）：v13-ctx-{light,dark}-{classic,liquid}.png + v13-tray-{light,dark}-{classic,liquid}.png 八张单图；v13-pair-{ctx,tray}-{light,dark}.png 四张并排（左经典右液态 2x nearest）；v13-ghost-{light,dark}-{raw,stretch}.png 透字检验——两轮独立评审像素级裁定「一眼可辨」（浅档 ΔL≈20-25 结构性明暗 vs 经典平直 254；深档顶沿差近 30 L）；无字形鬼影（深档 diff avg1.77/max5、浅档仅贴边 12 像素 Δ=10 无结构、黑白衬底同行亮度差 ≤2 L）
3. **几何复测 ✅**（v13-geo.json 随采集落盘）：两菜单 offsetWidth=**104**、项高 **27**；明暗×两材质八组合 rect 逐字段全等（应用内 104×89@170,200 项 27×3／托盘 104×116 项 27×4）= v12 基线；v8 立法照守（材质段只动 background/box-shadow）
4. **v11-tray.ps1 -Step popup 重跑 PASS ✅**：104×116@(1545,773) 工作区内贴图标上方（inside-workarea=True above-icon=True）
5. **评审连续 2 轮无 P1 ✅**：round-44（opus，无 P1 无 P2；P3-1 浅档辨识度弱于深档→立修：底带 0.06→0.09、底影 0.16→0.2，摆幅 19→~25 拉平深档）→ round-45（opus 收敛复验，无 P1 无 P2 无新增 P3，P3-1 专项判「到位且未过火」、浅档末项对比度 13.5:1）；档案 docs/review/round-44.md、round-45.md
- 回归 ✅：mini 右键沉默（menuOpen:false）；托盘 hover 事故修复后四托盘图全净
- 翻车/让步如实记：①首版浅档白 sheen 配方实测不可见（ΔRGB 6~7），自验阶段即推翻改「冷灰底霜+底部暗重」——像素证据先行未进评审轮 ②托盘采集 SetCursorPos 归位反钉 hover 蓝底（四张废帧重采；改 mouse_event +1px 真微移重命中）③mouse_event 的 dx 是 DWORD 传 -1 中断一次（负位移应 0xFFFFFFFF）④三次采集开工 DOM 态互异（dark/classic→dark/liquid→light/classic），config material 期间被外部两改（liquid_glass→收口时又翻 classic）——并行会话外部干预特征；终态按「material=config 持久值」纪律对齐 classic 并广播托盘 ⑤popup 回归覆盖 v11 同名旧帧（证据卫生同 v12 口径记 DEV_LOG）
- 无回滚项，结果不差于基线（45 绿=基线，回归全过）；round-45 留档采集改进项：下轮 ghost 字形对准菜单中心

## 2026-08-15 · v13 开工回执（任务 0）
- 目标：液态玻璃菜单与经典档一眼可辨——只改 [data-material="liquid_glass"] 下 .ctx-menu/.ctx-item 的光影（background/box-shadow/border-color/backdrop-filter），移植液态面板的顶部高光带/底部内阴影/明暗梯度/深色分层提亮
- 基线已核：PROGRESS 终态 v12 收口（cargo test 45 绿 0 ignored；两菜单 offsetWidth=104、项高 27、四组合 rect 全等）；本轮明卷要求测试 ≥45 绿 skipped=0
- 底线照单：菜单底色 ≥0.95 防透字（styles.css 浅色段注释纪律）；几何铁律（104/27/圆角/padding 不变）；经典档零改动；同步链路不碰
- BLOCKED 核实：无与本任务冲突项；采集后还原 config 持久值（实测 material=liquid_glass；BLOCKED 旧记录「领导配置 classic」已过时——领导亲验期间自行切到液态并落盘，如实记）
- 顺序：读液态面板配方 → 改两段液态菜单 CSS → 8 张并排对比截图+透字检验 → 几何/popup/回归 → 评审 round-44 起（连续 2 轮无 P1，封顶 4 轮）

## 2026-08-15 · v12 终验记录（明卷 5 条）
1. **cargo test ✅ 45 绿 0 failed 0 ignored**（纯 CSS 改动；menu_origin 6 定位单测断言零改动随套过）
2. **任务 2 数值自证 ✅**（CDP 实测，明细在 DEV_LOG v12 条目）：两菜单 offsetWidth 相等=**104**；每项 offsetHeight **27** 单行（不再 38）；算术自洽 104=textW 52（开机自启@13px）+24 左勾选槽+20 右 padding+8 菜单内边距；明暗×两材质四组合 rect 全等（应用内 104×89 项 96×27／托盘 104×116 项 96×27）
3. **截图证据 ✅**：docs/screenshots/v12/ 六张（托盘弹出明暗 v12-tray-{light,dark}.png + 展开态右键明暗×两材质 v12-ctx-{light,dark}-{classic,liquid}.png），肉眼无折行无右侧死白；评审档案同内容 docs/review/round-42-*.png
4. **v11-tray.ps1 -Step popup 重跑 PASS ✅**：104×116@(1545,773) 工作区内贴图标上方（inside-workarea=True above-icon=True）
5. **评审连续 2 轮无 P1 ✅**：round-42（opus，无 P1 无 P2；P3 建议项=右留白定稿 20 采纳）→ round-43（opus 收敛复验，无 P1 无 P2 无 P3，像素级复测全过+周边波及扫描无异常）；档案 docs/review/round-42.md、round-43.md
- 右 padding 定稿裁决：20（评审实测左 27–30 vs 右 24–26 平衡；18 超 macOS 比例带不建议）——BLOCKED #11 留档
- 任务书现状一处与终码不符如实记：应用内菜单「折行 textW 25/高 123」终码未复现（修复前实测 96×89 单行）；nowrap 硬保证已加
- 回归 ✅：mini 右键沉默/展开态右键正常/Esc 整收/失焦 ~158ms（v61 四项全 PASS）；v8 几何 4/4 IDENTICAL×39 元素（[data-material] 零几何属性，立法照守）
- 翻车/让步如实记：v12-menus.ps1 首跑 ConvertFrom-Json 得裸串（cdp-seq eval return JSON.stringify 双重编码坑，v11 已录复踩）→ 改 return 对象单层，修进脚本注释；v11-tray popup 回归覆盖了 docs/screenshots/v11/traymenu-popup.png 旧帧（同名新尺寸，证据卫生声明记 DEV_LOG）；无回滚项，结果不差于基线（45 绿=基线，交互回归全过）

## 2026-08-15 · v12 开工回执（任务 0）
- 目标：两套右键菜单留白统一——①.ctx-item 加 white-space:nowrap（macOS 铁律永不折行）②两菜单同规则贴内容（.ctx-menu width:max-content；宽=最长项 textW+左 24 勾选槽+右 padding+菜单内边距 8；min-width:76 下限保留兜单项）③右 padding 12→20（18–20 区间，评审对 HIG 定稿）与左槽视觉平衡；明确不做：菜单项增删/分隔线/图标/副文本
- 现状核实与任务书一致：托盘窗 lib.rs:629 inner_size(160.0,124.0) 写死+托盘页 .ctx-menu position:static 撑视口（styles.css:1116-1126）→tray_menu_fit 回告视口宽自证循环永 160；应用内 min-width:76（:704）压窄折行；共用 .ctx-item padding 5px 12px 5px 24px（:727）✓槽 left:8（:742）；tray_menu_fit=纯 set_size(LogicalSize)（lib.rs:279-281）；menu_origin 6 单测 mw/mh 全参数化与实际尺寸解耦→断言零改动
- 修法：styles.css 两处（.ctx-menu 加 width:max-content / .ctx-item nowrap+右 padding 20），托盘页回告链（tray-menu.ts getBoundingClientRect→tray_menu_fit）零改动即通；窗口初始 160x124 仅 hidden 一瞬不动
- 基线待验：cargo test ≥45 绿 skipped=0；dev 实例在跑（CDP 9223 双页齐，vite 1430 沙箱 502 为已知假象）；BLOCKED 无与本任务冲突项（领导配置 classic/Ctrl+P/短休5 勿动，采集备份-恢复纪律照守）
- 顺序：CSS 三改 → CDP 数值自证（同宽/单行 ~27-28/算术自洽/明暗×两材质四组合 rect 全等）→ cargo test + v11-tray.ps1 -Step popup 回归 → 评审 round-42 起（连续 2 轮无 P1，封顶 4 轮）
- 最大风险：max-content 与 box-sizing:border-box 的交互（实测算术自洽兜底）；fit 后窗宽变化影响 menu_origin 钳制（popup 回归兜底）；材质段零几何属性的 v8 立法照守

## 2026-08-15 · v11 终验记录（明卷 7 条）
1. **cargo test ✅ 45 绿 0 failed 0 ignored**（35 基线 +4 改键 serde/校验 +6 托盘定位纯函数；tsc clean；bun run build 双页产物齐）
2. **mini 右键双证据 ✅**：终码复验 mini dispatch contextmenu→menuOpen:false；展开态→open at 170,200（任务1 首轮证据+收口复验一致）
3. **托盘自绘菜单明卷全证 ✅**（scripts/v11-tray.ps1，docs/screenshots/v11/）：POPUP PASS（160x116@(1517,773) 工作区内贴图标上方，traymenu-popup.png/traymenu-popup-dark.png 明暗）；ITEMS PASS（四项+running→「暂停」动态文案现查 snapshot）；CHECK PASS（开机自启✓ 跟随注册表 OFF→ON→aria→OFF 双向，traymenu-autostart-on.png）；BLUR/ESC PASS；QUIT PASS（真实点击退出，Get-Process 查无）；**REAL-INPUT PASS**（scripts/v11-realinput.ps1：mouse_event 相对微移真 hover 命中、真实点击「设置…」→主窗展开开抽屉，推翻「透明菜单窗收不到真实输入」的存疑）
4. **改键全链证据 ✅**：UI 真实路径录 Ctrl+Alt+U→config.json 落 hotkey 键（material 等键不丢）→SendKeys ^%u idle→running→paused 双向、^%o 旧键已卸不动；录 Ctrl+Alt+P（本机被占）→红帽「被占用，已保留原键」（hotkey-occupied-error.png）+配置仍 U；× 关闭→effective:null、^%u 死、enabled:false 留记忆；重启→禁用保持+^%u 仍死（重启保持）；删 hotkey 键重启→默认 Ctrl+Alt+P→降级 O 生效、^%o 翻转（v10 行为保住）
5. **进度条三态截图 ✅**（docs/screenshots/v11/progress-{work-mid,break-mid,paused}.png + 4x zoom）：bar 与 snapshot done 逐 tick 数值一致（36.7%↔36.7%、26.6%↔26.6%）、暂停两次采样 20.1% 冻结；液态/经典×明暗四档在 round-40 采集中齐证
6. **PANEL_H 467 morph/几何 ✅**：456→467（+11 ≤ 任务书 +16 预算，实测不动点 467→末卡底 451.1→公式自洽）；v8-geometry 4 态×39 元素两档材质 rect 全等（4/4 IDENTICAL），抽屉四卡底−末行底全 0 不裁
7. **评审连续 2 轮无 P1 ✅**：round-40（opus，无 P1；P2-1 托盘首项 hover 描边环→立修）→ round-41（opus，无 P1 无 P2，P2-1 复验通过+全量复核通过；P3-1 证据集卫生=旧帧重采已做）；档案 docs/review/round-40.md、round-41.md
- 翻车/让步如实记：①任务4 采集首跑 IIFE 缺 async 全灭+接管时残留计时恰好自然翻相位致两态串相→相位归一+断言硬化（scripts/v11-progress.ps1）②v11-tray.ps1 自踩三坑：EnumWindows 泛型委托不可封送、GetWindowTextW 缺 CharSet.Unicode 中文乱码、cdp-seq 无 click 命令（首跑 QUIT FAIL 系脚本 bug 非产品）③「再右键=关」toggle 路径在溢出面板图标场景不可测（重开面板的左键先触发失焦关，等效同为关）——任务栏直显图标场景才可达，记此限制④P2-1 根因=UA :focus-visible 环+CDP 合成 hover 残留双因（长命实例自动化污染），修复=outline:none+opened blur 残留焦点 ⑤领导 8-14 晚~8-15 午持续亲验：自启/材质 classic/短休 5 分钟/自录 Ctrl+P 均为领导真实选择，全程未动其配置（采集脚本备份-恢复纪律+stats 哈希比对）
- 生成器根因级发现已立法注释+沉淀：**WebView2 同 user data folder 第二窗口 additional_browser_args 必须与主窗逐字一致**（否则 ERROR_INVALID_STATE 被吞、窗永不创建）→ docs/solutions/platform-integration/；自动化四陷阱（SetCursorPos 无 WM_MOUSEMOVE 等）→ docs/solutions/workflow-issues/
- 无回滚项，结果不差于基线（45 绿 > 35 绿基线，v7-v10 交互回归全过）
- 分工记录（领导指令）：任务3/任务2 由两个生成器子智能体先后实现（串行避 lib.rs 冲突），评审由独立 opus 子智能体两轮执行，主会话统筹规格/验货/证据/三件套

## 2026-08-14 · v11 开工回执（任务 0）
- 目标：领导亲验四项一次做完——①mini 态右键取消（菜单 89px vs 窗 76px 物理裁切无解，入口=展开态右键+托盘）②托盘菜单自绘（弃 muda 原生，无边框小窗贴图标弹出，苹果风四项，失焦/Esc 关，四边任务栏钳制）③快捷键完整改键（抽屉加行：显示/按下即录/占用回退/可关闭，config.json 持久化）④胶囊底部 2px 进度条（snapshot 驱动，专注红/短休绿/长休蓝，两材质两主题）
- 明卷 7 条：cargo test ≥35 绿 skipped=0；mini 右键无响应+展开态正常双证据；托盘弹层明暗截图/四项/勾选态/失焦关/退出无进程；改键全链（录键→落盘→SendKeys 翻转→重启保持；占用回退；关闭无效）；进度条三态截图；PANEL_H 若变 morph 几何 rect 全等；评审 round-40 起连续 2 轮无 P1（封顶 6）
- 基线已核：cargo test 35 绿 0 ignored；dev 实例在跑（CDP 9223 页面=番茄钟@1430）；BLOCKED 无未决冲突项（#1 通知/#3 真双屏均设备条件留裁，与本任务无涉）
- 现状核实与任务书一致：托盘 lib.rs:437-455 菜单/456-466 事件/467-479 左键/510-521 动态文案；快捷键 397-406 handler/485-496 注册降级；mini 右键 main.ts:205-217；PANEL_H=456 main.ts:14；AppConfig 仅 timer+material
- 顺序：任务1（纯前端一行级）→ 任务4 进度条（前端+CSS）→ 任务3 改键（Rust 字段+测试+抽屉行，PANEL_H 重测）→ 任务2 托盘弹层（最重，新窗口方案）→ 证据采集 → 评审循环
- 最大风险：托盘自绘弹层方案选型（独立小窗 vs 主窗内弹层——主窗 220x76/340x456 均贴不到托盘，倾向新建无边框置顶小窗）；改键录制 UX 与全局热键 unregister/register 运行时切换
- 环境坑照单遵守：CDP eval 一律 IIFE；SendInput 被拦用 WScript SendKeys；并行会话干预先排除；采集后恢复窗口起始态；改 stats.json/config.json 先备份验完恢复
- 明确不做：每日目标/勿扰联动/音效/长周期趋势

## 2026-08-14 · v10 终验记录（明卷 8 条）
1. **cargo test ✅ 35 绿 0 failed 0 ignored**（基线 30 → +5 月聚合新测试：跨月不漏/空月全零/分钟换算/月初星期偏移锚定 2026-08-01=周六=5、2026-06-01=周一=0/天数 28·29·30·越界钳）
2. **托盘三证据 ✅**（docs/screenshots/v10/）：tray-icon.png 溢出面板图标 + FOCUS PASS（左键 fg→本窗）；tray-menu.png（idle「开始」）/tray-menu-running.png（running「暂停」）动态文案；tray-menu-before-quit.png + QUIT PASS（Get-Process 查无 pomodoro-clock，真退进程）
3. **自启注册表正反向 ✅**：tauri-plugin-autostart 写 `HKCU\...\Run` 值名「番茄钟」（productName），开启/关闭两次 reg query 输出齐；窗口右键菜单勾选态每次开菜单现查真实注册状态
4. **快捷键翻转 ✅**：Ctrl+Alt+P 被占（dev.log「HotKey already registered id: 589858」）→ 降级 Ctrl+Alt+O 真实触发；WScript SendKeys ^%o 实测 status idle→running→idle 双向翻转
5. **统计四截图 ✅**：round-39-stat-week-count/min.png（个数/分钟周柱）+ stat-month-cur/prev.png（当月/上月月历热力格，‹ › 翻页 ›当月禁用）；注入历史数据验翻页后已恢复真实 stats.json（.bak-v10 回滚）
6. **多屏审查+模拟实测 ✅**：drag 立修（GRAB 改存 CSS 坐标 bits，跟随线程每帧 GetDpiForWindow 现算物理偏移，跨屏 DPI 不漂移）；morph 审查健壮（MonitorFromWindow DEFAULTTONEAREST + i32 负坐标 + clamp hi<lo 兜底）。MULTIMON PASS（%TEMP%\pomo-shots\v10-multimon.json + scripts/v10-multimon.ps1）：半出右缘/负坐标/完全出屏(3000) 三态展开全钳回工作区（R≤1920/L≥0）且 CDP 应答；边缘拖拽精确跟到 OS 钳位光标（L=1779=1919-140 分毫不差）、up 即停跟
7. **评审连续 2 轮无 P1 ✅**：round-38（sonnet，无 P1，P2-1 占位格不可见已修）→ round-39（haiku，无 P1 无 P2，复验通过+今天描边/梯度/数据结构像素级自洽）；P3-1 垫格对比立修（0.16→0.35，实测 RGB 248→245，与 level-0 保 4 级距），P3-2/3/4 裁决不修记理由；档案 docs/review/round-38.md、round-39.md
8. **回归抽验 ✅**：失焦即收 ~159ms PASS（v9-blur.ps1）；材质几何不变 4/4 IDENTICAL（v8-geometry.ps1：panel/drawer/ctx/mini 两档 rect 全等）
- 翻车/让步如实记：①评审员 opus→sonnet→haiku 两次 403 配额降级（功能结论不受影响，独立性保住）；②托盘取证踩 UIA 同名任务栏按钮（按宽度 ≤60 过滤）与「^」toggle（先查再点，3 轮循环兜底）两坑，已修进 scripts/v10-tray.ps1；③UIA 枚举不到 Tauri 托盘菜单项 → 键盘赛道（右键点 UIA 确证后非盲注）；④一次右键误落桌面弹出桌面菜单（未碰我们菜单，timer 仍 running 为证）；⑤月历扁条造型维持（已裁决）；⑥真双屏物理实测留 BLOCKED（需设备条件）；⑦P3-1 首修 0.20 无效（--track 自身 alpha 0.16 双重衰减），实测后才到 0.35——像素证据不靠拍脑袋
- 无回滚项，结果不差于基线（35 绿 > 30 绿基线，v7-v9 交互抽验全过）

## 2026-08-14 · v10 开工回执（任务 0）
- 目标：顺手功能五件套一次做完——托盘（左键聚焦/右键三菜单）、自启（HKCU Run，默认关）、全局快捷键（Ctrl+Alt+P 切换，占用降级 Ctrl+Alt+O）、统计升级（个数/分钟 × 周/月，月历热力格+翻页）、多屏/DPI 审查（能审则审能模拟则模拟，真双屏留 BLOCKED）
- 明卷 8 条：cargo test ≥34 绿 skipped=0（月聚合 ≥4 新测试）；托盘三证据；自启注册表正反向输出；快捷键 status 翻转证据；统计四截图；多屏审查结论+模拟实测；评审 round-38 起连续 2 轮无 P1（封顶 6）；回归抽验失焦即收+材质几何不变各一条
- 基线：cargo test 30 绿 skipped=0（v9 收口）；BLOCKED #3 即本任务清单，完成后勾销
- 顺序：三依赖同批进 Cargo.toml 一次构建 → 任务1 托盘 → 任务2 自启 → 任务3 快捷键 → 任务4 月聚合 Rust+测试 → 前端统计 UI → 任务5 多屏审查 → 证据采集 → 评审循环
- 最大风险：三新依赖首构拉 crates 量大；托盘菜单文案需随 snapshot 动态；快捷键占用降级路径要真验
- 界限：既有断言不动总数只增；v7-v9 交互全不回归；失败路径只记日志+BLOCKED 不弹窗不崩

- 状态：**v9 按钮悬停文字发毛根治收口（2026-08-14）**：五处（六条规则）transform 缩放病灶全去，hover/active 改 filter brightness 明暗反馈；修复前后对照帧证文字从「中途发毛」变「全程锐利」；评审 round-36/37 连续两轮无 P1 收敛（封顶 4 轮用 2 轮）；cargo test 30 绿 skipped=0；失焦即收 ~164ms PASS。dev 实例在跑（vite 1430 / CDP 9223），领导可亲验：鼠标放上「开始/重置/休息」文字不再先糊后清。

## 2026-08-14 · v9 终验记录（明卷 4 条）
1. **代码断言 ✅**：grep 断言 `.btn`/`.icon-btn`/`.collapse-btn`/`.step-btn` 的 hover/active 规则零 scale/transform（含任务书未列的同款病灶 `.icon-btn:active scale(0.92)`，按「按压同治」拍板一并治）；transition 清单同步去 transform。保护项未碰：ctx-menu 入场 scale、抽屉 translateX、toggle 旋钮、morph
2. **真实悬停证据 ✅**：CDP Input.dispatchMouseEvent 真实悬停三按钮，+40/+120/+700ms 定时帧 4x 放大——修复前 round-36-pre-btn-primary-mid1「开始」发毛有重影 vs end 锐利（问题复现）；修复后 round-36-r36/r37 全部按钮三帧同等锐利（明暗两主题 × 开始/重置/休息 + 收起把手 + mini 播放钮 + 步进钮按压）
3. **评审收敛 ✅**：round-36/37 连续两轮无 P1 无 P2（封顶 4 轮用 2 轮）；P3×3 全部裁决不修记理由（见 BLOCKED「9. v9 评审 P3 留档」）
4. **回归 ✅**：cargo test **30 绿 0 failed 0 ignored**（纯 CSS 改动 Rust 零动）；失焦即收抽验 PASS（~164ms 收至 220x76，tmp/v9-blur.ps1）
- 方案要点：`filter: brightness()` 不缩放合成层→文字零重采样，且天然兼容液态档渐变背景免逐材质覆盖；浅色 hover 微沉 0.96/0.94、深色提亮 1.15~1.22、按压快速变暗 0.82~0.88；`.step-btn` 按压用 --input-bg 自适应明暗
- 翻车/让步如实记：DEV_LOG 编辑一度吃掉 v8 标题行（已补回，grep 章节结构验证）；采集脚本踩 PowerShell `New-Object Type(乘法表达式)` 参数模式不求值坑（→ `::new()`，教训已记 DEV_LOG）；无回滚项，结果不差于基线

## 2026-08-14 · v9 开工回执（任务 0）
目标：五个选择器 hover/active 去 scale/transform（grep 断言）→ 明暗/高光反馈；真实悬停中途+结束截帧证文字全程锐利；评审 round-36 起连续 2 轮无 P1（封顶 4）；cargo test ≥30 绿 + 失焦即收抽验。根因已核对：styles.css:188/189/303/307/308/487/490/574/576 九行命中（含 .icon-btn:active scale(0.92)，任务书未列但属「按压同治」拍板范围，一并治）。方案：filter: brightness()（不缩放合成层、文字不重采样、天然兼容液态档渐变背景，免逐材质覆盖）；拍板 3 保护项（ctx-menu 入场 scale/抽屉 translateX/toggle 旋钮/morph）不碰。

## 2026-08-14 · v8 终验记录（明卷 5 条）
1. cargo test **30 绿 0 failed 0 ignored**；tsc clean
2. **几何不变性证据**（tmp/v8-geometry.ps1，CDP 实测 getBoundingClientRect，容差 0.5px）：面板/抽屉/菜单/迷你 4 态 × 37 元素，两档材质 **0 差异**。关键行（liquid vs classic，格式 x,y,w,h）：
   | 元素 | liquid_glass | classic |
   |---|---|---|
   | .collapse-btn | 148,416.19,44,22 | 148,416.19,44,22 |
   | .drawer .group-card#0 | 18,81.7,304,129 | 同左 |
   | .drawer .group-card#2（外观） | 18,353.13,304,88 | 同左 |
   | .drawer .toggle | 272,292.42,36,22 | 同左 |
   | .drawer .theme-switch#0/#1 | 172,362.13,136,26 / 172,406.13,136,26 | 同左 |
   | .drawer .stepper#0~3 | 214,*,94,25 | 同左 |
   | #btn-toggle-run（mini） | 176,24,28,28 | 同左 |
   抽屉三卡「卡片底−末行底」两档均 0（齐平不裁），卡2底 441.13 < 抽屉底 442；面板底部留白 17.8px（把手化后）
3. **清单 15 项逐项闭环**：全部「已修+截图证据」，无漏项——证据=round-32~35 八态截图（修复后）对照 round-31（修复前）+ round-32-toggle-off.png（Switch 关态）+ 本表第 2 条几何实测。明细：P2①圆头≤2%不渲染 ②暂停钮明暗同配方 ③专注红字--work-text ④Switch--input-bg 0.18 ⑤深色分段文字+pill+边线 ⑥未来日短划 ⑦今日柱去灰底；P3⑧菜单 76px ⑨深色 mini 高光降亮 ⑩分段 136px 等宽 ⑪本周标题节奏 ⑫浅色菜单发丝边线 ⑬胶囊 16/16 对称 ⑭圆点空心 0.85 ⑮收起钮把手化
4. 评审 **round-34/35 连续两轮无 P1** 收敛（封顶 6 轮用 4 轮；R33 P1=采集事故非产品缺陷，补采+根因修复；档案 docs/review/round-32~35.md）
5. v7 回归四 PASS（宽容点击 8px 展开/抽屉开态 Esc 整收 220x76/失焦 ~176ms/拖拽 leave 后续跟 100px）；材质真实点击往返 classic↔liquid_glass 落盘正确，**重启 dev 后 DOM/config 均 liquid_glass**
- 拍板项 ✅：圆点「进行中」=半填充（左半实填+工作色描边，R32 评审要求形状级差异后定稿）；数据仍全走 Rust snapshot。不修关闭 ✅：#7-1（胶囊即按钮）/#5-4（76px 物理不可避）/#5-6（高度预算刚收紧，加行再溢出），理由已记 BLOCKED
- 环境新教训：证据补拍脚本必须恢复窗口起始状态（toggle-off 脚本留展开态 → R33 首张 mini 拍成面板，P1 采集事故）
- 让步记录：无（本轮全在既定界限内；P3 深色红字维持 Apple 官方深色 systemRed #FF453A，评审目测 3.5:1 但官方取值优先）

## 2026-08-14 · v8 开工回执（任务 0）
- 目标：①P0 两档材质几何统一并立法（材质规则只许 background/box-shadow/color/border-color，禁 padding/margin/width/height/gap/font-size），CDP rect 全等对比表贴本文件；②清单 15 项逐项闭环（修复前后截图 or 不修+理由）；③循环圆点加「进行中」中间态（数据仍走 Rust snapshot）；④#7-1/#5-4/#5-6 按拍板「不修」记理由关闭；⑤评审 round-32 起连续 2 轮无 P1，收敛后勾销 BLOCKED #0/#5/#7/#8
- 根因已核对：styles.css:777 液态玻璃 `.drawer .section-caption` padding-top 6px vs 经典 :409 的 2px，材质规则改几何 + 抽屉 space-evenly 均分 → 全控件重排
- 顺序：环境核查（dev 实例/基线测试）→ P0 几何统一+立法 → 清单 15 项 → 圆点中间态 → cargo test → 八态采集 round-32 评审循环 → v7 回归+材质重启保持 → BLOCKED 勾销
- 最大风险：几何统一可能再触 R29 高度预算临界点（抽屉 368px 预算，余量仅 9px）——每处几何改动后必跑「卡片底−末行底」断言
- 基线待验：cargo test 须 ≥30 绿 skipped=0；界限：timer/stats 引擎、v7 交互、1430/9223 不碰

- 状态：**v6.1 液态玻璃改版收口（2026-08-14）**：官方 spec → 四界面×明暗落地 → 材质开关持久化，评审 round-30/31 连续两轮无 P1 收敛（封顶 6 轮用 5 轮），cargo test 30 绿 skipped=0，v7 交互回归四项全 PASS。明卷 5 条完成条件全满足。残留 P2/P3 留裁 → BLOCKED #8；dev 实例在跑（vite 1430 / CDP 9223，任务 bjbfjjf67），领导可亲验：设置抽屉「材质」分段切经典/液态玻璃。

## 2026-08-14 · v6.1 终验记录（明卷 5 条）
1. cargo test **30 绿 0 failed 0 ignored**（26 基线 + appconfig 4 新）；tsc clean
2. docs/liquid-glass-spec.md ✅ 每条带官方出处；反验点：35% 暗层→materials.json 原文（§1）、同心圆角 R内=R外−padding→WWDC25-356 转录（§4）
3. 截图 docs/review/round-27~31（每轮八态：迷你/面板/抽屉/菜单×明暗）+ round-31-classic-* 经典对照；评审 round-30/31 连续两轮无 P1
4. 材质切换实测 ✅：真实按钮路径 经典↔液态玻璃 往返 + config.json 落盘（set_config 不丢材质键）；切经典八态回旧观感（round-31-classic-*）；**重启 dev 后 DOM/config 均 liquid_glass 保持**
5. v7 回归四项全 PASS（tmp/v61-regression.ps1）：宽容点击 8px 抖动展开；抽屉开态真实 Esc 整收 220x76；失焦即收 ~156ms；快速拖拽 leave 前跟 112px / leave 后续跟 100px 不冻结
- 环境新教训：透明窗圆角外像素点击穿透——合成鼠标终点落在胶囊圆角外（y<38 的端部区）会触发 mouseleave 杀拖拽判定；脚本终点须落在胶囊中线
- 评审 P1 修复史：R27 菜单未采到（eval 顶层 const 全局残留，IIFE 修）/ R28 深色主键失红（选择器权重 0,3,0>0,2,0 且源码序靠后）/ R29 抽屉分组卡裁行（overflow:hidden 的 flex 子项 min-height:auto=0 被 shrink；行 padding 12→9 + 抽屉 padding 回收 64px）
- 让步记录：①玻璃明暗按 data-theme 两套固定配方（官方为连续响应背景亮度，透明 WebView2 背景不可知）②抽屉/右键菜单用全实底+sheen（0.97 alpha 仍透大字号白字残影，可读性优先于通透度，spec §1 regular 语义）③无官方数值的参数（模糊半径/高光强度）自校准、评审把关，spec §9 已声明

## 2026-08-13 · v6.1 开工回执（任务 0）
- 目标：全 UI（胶囊/面板/右键菜单/抽屉）×明暗换拟态液态玻璃；设置加材质分段开关（经典/液态玻璃，config.json 持久化，默认液态玻璃）
- 顺序：任务1 子智能体读苹果官方 JSON 文档 → docs/liquid-glass-spec.md（每条带出处）→ 任务2 落地四界面 → 材质切换+持久化 → 评审循环 round-27 起（连续 2 轮无 P1，封顶 6 轮）→ v7 交互回归实测
- 最大风险：透明 WebView2 糊不到桌面 → 液态玻璃只能页内拟态（分层渐变+高光+边缘光），参数无官方 Web 映射需从 spec 转译；Material 枚举进 config.rs 涉及 Rust 序列化默认值，测试不得 <26 绿
- 任务1 ✅ docs/liquid-glass-spec.md（两子智能体：官方 JSON + WWDC25-219/356 转录；HIG 正确前缀 /tutorials/data/design/...；35% 暗层与同心圆角为可反验数值，模糊半径等标注「官方未给」）
- 任务2a ✅ 材质管道：appconfig.rs（Material 枚举+AppConfig flatten 接管 config.json，修掉 TimerConfig::save 直写丢键隐患）+ set_material 命令 + 前端分段控件；cargo test 30 绿（+4）tsc clean；CDP 实测 set_config 后材质键保留、legacy config 回默认 liquid_glass
- 任务2b 首版 CSS 落地（[data-material="liquid_glass"] 作用域，经典零改动）；自验翻车一处：抽屉/菜单 0.88 alpha 盖面板透字 → 近不透明 0.97/0.95；明暗层级拉开（内容卡比外壳更白/更亮）
- 环境：旧 dev 后台任务跨日死亡，vite 1430 孤儿整树杀后重启（bgdf8fl0c）；新采集管线 tmp/v61-capture.ps1（四界面×明暗×材质 8 态/轮）

- 状态：**v7 瞬态面板收口（2026-08-13）**：失焦即收/Esc/宽容点击三项落地并全量验收通过，评审 round-25/26 连续两轮无 P1 收敛，cargo test 26 绿 skipped=0。明卷 5 条完成条件全满足。残留 P2/P3 留裁 → BLOCKED #7；dev 实例在跑（vite 1430 / CDP 9223），领导可亲验：点胶囊展开（手抖 8px 内没事）、点面板外任意处或 Alt+Tab 即收、Esc 整收。

## 2026-08-13 · v7 评审循环（任务 3）
- round-25：无 P1（P2×3 涉既有设计→BLOCKED #7；P3×3 同）
- round-26：无 P1（新 P2×2=进度圆头 0% 错位/胶囊标签对比，均既有元素 v7 未触碰→BLOCKED #7）→ 连续两轮无 P1 收敛（封顶 6 轮内用 2 轮）
- 评审采集管线变更：本机双 WT 全屏置顶使 CopyFromScreen 截到的是终端 → 改 CDP Page.captureScreenshot 离屏渲染 + System.Drawing 底色合成（tmp\v7-capture.ps1）；后续评审轮沿用此管线直到置顶遮挡消失

## 2026-08-13 · v7 任务1/2 验收记录
- 实现：①main.ts attachThresholdDrag 重写——拖拽阈值 6→12px、松手位移<10px 且按下<500ms 一律算点击（去掉 !wasDragging）；②Esc/失焦整收走 collapsePanel（关菜单+复位抽屉+setExpanded(false)）；③lib.rs on_window_event(Focused(false))→emit window-blurred；bridge 加 onBlur
- 顺手修掉的拖拽冻结 Bug（领导实测发现）：旧代码 mouseleave→endNativeDrag，快拖时光标跑出窗口即杀拖拽线程→窗口冻结。修法：mouseleave 只清点击判定不杀线程；Rust 拖拽线程加 GetAsyncKeyState 兜底自终止（1500ms 宽限防误杀 CDP 合成拖拽）；mousedown 重置陈旧 dragging 态
- 验收（全 PASS）：手抖点击 10/10 展开（≤8px 抖动）；30px 拖拽精确跟 30px 不展开；leave 后续拖 200px 不冻结；快速交替 10 次终态 220x76 干净；切窗/Alt+Tab 失焦 1s 内收起；mini 失焦无变化；抽屉失焦收起且再展开无残留；morph 中途失焦无残影；真实键盘 Esc（WScript SendKeys 通道）收起+抽屉整收
- 环境教训：①本机双 Windows Terminal 全屏置顶盖住一切→真实点击全落空，验证改 CDP dispatch + SetCursorPos 合成；②SendInput 键盘注入被拦（返回 0），真实键盘用 WScript SendKeys；③SetForegroundWindow 前台权不稳定，AttachThreadInput + 重试才稳；④PowerShell `@($a + 1, $b)` 逗号优先级陷阱
- 环境偏离声明：明卷「点桌面/别的窗口」因全屏置顶终端无法真实点击，用 SetForegroundWindow 焦点切换等价验证（同一 Focused(false) 链路）
- 证据：docs/screenshots/v7/（rapid-final、blur-*、blur-midmorph-after）

## 2026-08-13 · v7 开工回执（任务 0）
- 三点核对 ✅：①main.ts:154 位移>6px 即 startNativeDrag（自研 drag_begin 线程，非 start_dragging）、:165 moved<6 && !wasDragging 才展开——手抖即失效；②main.ts:173 收起仅 btn-collapse；③全工程无 Focused/blur 监听（lib.rs 无 on_window_event）
- 基线：cargo test 26 绿 skipped=0；config.json 即时落盘已有（set_config→save），满足「失焦收不丢设置」前提
- 顺序：任务2（宽容点击+Esc，纯前端）→ 任务1（Rust on_window_event→emit→setExpanded(false)）→ 重启 dev 真实鼠标验收 → 评审循环 round-25 起
- 最大风险：真实失焦验收需真实鼠标（CDP 抢焦无效），SetCursorPos+mouse_event 注入；morph 进行中失焦打断的残影风险由 morphGen 代际机制兜底

- 状态：**长休息功能已暴露并收口（2026-08-13）**：设置抽屉+循环圆点指示器落地，评审 round-23/24 连续两轮无 P1 收敛（封顶轮内），cargo test 26 绿 skipped=0。详见末节。

## 2026-08-13 · 长休息暴露任务 · 收口记录
- 任务1 ✅：set_config 加 long_break_min/long_break_every（every 钳 ≥1 防取模 panic）；新增 config.json 持久化（exe 同目录，仿 stats.rs，serde(default) 缺字段补默认，损坏回默认不崩）；抽屉加「长休息(1-60分钟)」+「长休间隔(2-10个)」步进器；mock bridge 补字段（注意前端发 camelCase）
- 任务2 ✅：TimerSnapshot 加 long_break_every，圆点=completed_work_count % every（长休全亮），前端零自计数；三状态两主题截图在 docs/screenshots/cycle-{0,2,all}-{light,dark}.png
- 任务3 ✅：round-22（P1=采集 artifact、P2 分组语义+圆点对比立修）→ round-23 无 P1（P2=误读，crop 实证开关开态本有绿色）→ round-24 无 P1，连续两轮收敛
- 实测证据：间隔改 2 连打 2 番茄自动进长休（15:41:39 short c1 → 15:41:59 work c1 → 15:43:00 long_break c2，60s 整链）；重启后 30/10/20/3 保持（IPC+UI 双路核实）
- 环境教训：①本机有并行 Claude 会话会干预运行中 app（15:16 blip 实为外部 set_config+start，干净复测排除 app bug）②vite/bun 进程树要整树杀（taskkill /T）否则 1430 残留 ③PANEL_H 441→456（圆点行 +16px，CDP 实测）
- 残留：P2 圆点「当前个」中间态（涉既定设计，留裁决）+ P3 清单 → BLOCKED #0；stats.json 测试污染已清（备份 stats.json.bak-20260813）；dev 实例已于收口后被外部停止，亲验需重开 `bun run tauri dev`

## 2026-08-13 · 长休息暴露任务（执行会话）
- 目标理解：长休引擎已在 Rust 但用户摸不到 → 设置抽屉加「长休息时长（默认15)+长休间隔（默认4)」两项步进器 + 面板计时区下方循环圆点指示器（completed_work_count % long_break_every，长休全亮）；评审 round-24 封顶，连续 2 轮无 P1 收敛
- 任务0核实：①lib.rs:88 set_config 只收 work_min/short_break_min/auto_start_next，长休两参数不透传 → 需改签名；②**config 无任何持久化**——启动恒 TimerConfig::default()，只有 stats.json 落盘；goal 验收要求「重启后设置保持」→ 新增 config.json（exe 同目录，仿 stats.rs load/save，损坏归零不崩）
- 让步记录：goal 假设「现有两项已持久化、长休走同一条路」与实际不符（现有也不持久化）；按验收条件第 1 条「重启 dev 后设置值保持」优先，为四项设置统一补持久化
- 基线：cargo test 22 绿 skipped=0；round-21 四态基线截图已采（docs/review/round-21*.png）；dev 实例在跑（vite 1430 / CDP 9223，但非受管进程，Rust 改动需重启 tauri dev）
- 顺序：Rust set_config 加参+config.json 持久化+补测试 → 前端两步进器+mock 补字段 → 指示器圆点 → 重启 dev 实测（间隔2连打2番茄进长休+重启保持）→ 评审循环 22-24

- 状态：**图标已重设计为 B·计时环（2026-08-13 领导从 4 候选中选定）**，全套图标已重生成并嵌入 exe；玻璃版备份 `docs/review/icon-old3-glass/`。其余同下：

- 状态：**领导 4 项返工全部收口（2026-08-13）**：4 项独立验收全通过、评审循环 round-19/20 连续两轮无 P1 收敛（封顶线）、cargo test 22 绿 skipped=0。接续读 `docs/HANDOFF-2026-08-12.md` + `CLAUDE.md`。

## 2026-08-13 · 任务0 核对与基线（执行会话）
- 三嫌疑全部对上：①main.ts:159-166 先切 class 再 animate_size——collapse-f00 实测大圆球（radius 38→窗口仍 ~400px 宽）；②sizes.txt 帧间隔 ~20ms≈50fps；③lib.rs:240 SWP_NOMOVE 锚左上角向右下机械拉伸
- 4 问题理解：①动画=class 先行+50fps+锚角拉伸，收起尤甚 ②切换键 ☕/▶ emoji 突兀→纯文字 ③齿轮→右键菜单（自绘苹果风，原生 Menu 对无边框 WebView 窗口支持不可靠）④图标扁平→Liquid Glass 重绘
- 修复顺序：1 动画 → 2 emoji → 3 右键菜单 → 4 图标 → 评审循环（round 15 起）
- 动画方案：窗口瞬时 set_size + 纯 CSS 卡片 morph（合成器 60fps），生长方向朝屏幕内防出屏
- 基线：round-14 四态截图 + anim14/ 逐帧（collapse-f00 大圆球=拉伸实锤）；连点 10 次终态 mini 220x76 干净
- 最大风险：自绘右键菜单在透明无边框窗口内的定位/出屏处理；图标 SVG→PNG 光栅化仍走 CDP 截图管线
- dev 沿用上一会话进程（vite 1430 / CDP 9223 已验证是本应用）
- 任务1 ✅ 动画重做：窗口瞬时 set_size + 卡片 WAAPI morph（width/height/radius/translate 同时间线 340ms spring），内容固定自然尺寸+交叉淡入；Rust anim 逐帧模块删除，换 morph_begin（防出屏钳制+返回锚点偏移）/morph_commit。证据 docs/review/anim15/：rAF 采样 60fps 双向单调、PNG 帧无拉伸无圆球、连点10次终态 220x76、cargo test 22 绿
- 任务2 ✅ ☕/▶ emoji 删除，纯文字「休息/专注」
- 任务3 ✅ 齿轮删除+底栏收起居中；自绘右键菜单（苹果风、主题跟随、防出屏钳制、Esc/外点关闭）→「设置…」自动展开+滑入抽屉；阈值拖拽限定左键。证据 docs/review/ctx/（真实右键截图）；cargo test 22 绿
- 注意：morph 期间（340ms）窗口大于卡片的透明环吞点击——瞬时、可接受，已知情
- 任务4 ✅ 图标 Liquid Glass 重绘：候选 A（边缘光）/B（分层纵深）手写 SVG → 独立评审盲选 B（A 高光洗掉品牌红）→ B 按评审补强（极浅 sheen+内侧边缘光+底部纵深）→ app-icon.svg 替换 + tauri icon 全套重生成；旧套备份 docs/review/icon-old2/；新旧 32/128 hash 均不同；cargo test 22 绿。注意：运行中 exe 图标需下次构建才嵌入
- 评审教训沉淀：SVG objectBoundingBox 渐变用于零高度 line 会整根消失 → glyph 渐变用 userSpaceOnUse
- 评审循环 round 15-20：R15 两 P1（按钮底色=误读、收起键对比=成立）→ R16 无 P1 → R17 无 P1（材质渗透 P2 立修）→ R18 一 P1（透底：查明 backdrop-filter 在透明 WebView2 糊不到桌面，材料改实心 alpha 1.0）→ R19 无 P1（开关 P2=误读，crop 实证）→ R20 无 P1，连续两轮收敛（封顶）
- 任务5 ✅ 独立验收子智能体逐项确认：动画/emoji/右键菜单/图标 4 项全部通过
- 评审方法论教训：整图缩略会造成误读（按钮底色、开关态、省略号三次），评审提示词已要求放大局部再下结论；可疑项先用 crop-tool 实测再改
- 平台限制定论：透明 WebView2 backdrop-filter 只合成页内内容；真模糊需 DWM Acrylic，但会把圆角外透明角填成方角——故材料走实心，圆角优先
- 终态：cargo test 22 绿 skipped=0；评审档案 docs/review/round-15..20.md；残留 P2/P3 已记 BLOCKED

## 2026-08-12 晚 · 领导 4 项修复会话
- 基线核对 ✅：cargo test 22 绿 skipped=0；Round 11 基线截图已采（覆盖旧无评审同名图）；4 问题亲眼确认（底部大面积留白/齿轮压计时区上方/图标扁平/动画双层=Rust 220ms ease-out + CSS 0.5s spring）
- 轮次裁决：goal 说 round-10 起，但 round-10/11 已被上会话占用 → 基线=Round 11，新评审轮 12–15
- 4 问题理解：①PANEL_H=600 写死 vs 内容实测偏矮 ②Rust SetWindowPos 动画与 CSS transform spring 双源打架 ③齿轮绝对定位 panel-head 右上压计时区 ④图标扁平红底无光影层次
- 修复顺序：4（留白）→1（动画）→3（齿轮）→2（图标），先易后难
- 动画方案裁决：选方案 B（纯 Rust 窗口动画 340ms 单调弹簧曲线 + 内容纯淡入淡出），弃方案 A——窗口瞬时 set_size 会让动画期透明区露桌面且吞点击，CSS 弹簧尺寸与窗口矩形必有缝
- 最大风险：tauri icon 光栅化管线（SVG→PNG 无全局工具，拟用 CDP 截图）
- 环境坑新发现：`powershell`(5.1) 误读无 BOM 的 win.ps1 中文注释报解析错 → 用 pwsh 7 跑 capture 脚本即正常，不改脚本；沙箱内 localhost 会 502/拒绝，CDP 类命令需关沙箱
- 修复 4 ✅：PANEL_H 600→441（CDP 实测内容底 426.2+padding 14），底部留白 14px；mock-preview 同步
- 修复 1 ✅：纯 Rust 窗口动画 340ms ease-out quart（单调无回跳），CSS 删 transform spring 只留 opacity 淡入淡出，border-radius 同步 340ms 同曲线；逐帧证据 docs/review/anim/sizes.txt（expand/collapse 双向 14+ 帧单调）、连点 10 次终态 220x76 无残影（rapid-final.png）
- 修复 3 ✅：齿轮移入 .panel-foot 底部居左（收起居中、同宽 spacer 平衡），移出 panel-head 不再压计时区/不扰拖拽
- 修复 2 ✅：3 候选手写 SVG（A 玻璃高光/B 果实纵深/C 极简徽章）→ 独立评审盲选 A（B 误读成对勾、C 像斜杠）；高光降 0.26 后替换 app-icon.svg，tauri icon 全套重生成；旧图标备份 docs/review/icon-old/
- 4 项修完，cargo test 22 绿 skipped=0；进入评审循环 round 12
- 评审循环 ✅ 收敛：round-12 无 P1（P2-1 胶囊底色=评审误读；进度轨道对比度立修 0.12→0.16/0.16→0.20）→ round-13 无 P1，连续 2 轮达标（round-15 封顶内，实际用 2 轮）
- 最终状态：cargo test 22 绿 skipped=0；4 项前后对比=round-11（修复前）vs round-13（修复后）；动画证据 docs/review/anim/；图标对比 docs/review/icon-candidates/{old,new}-*.png + icon-old/ 备份
- 遗留裁决项已记 BLOCKED #4（休息键 emoji 风格争议）；dev 进程本会话退出即停，亲验需 `bun run tauri dev`
- 目标理解：Tauri 2 悬浮番茄钟，「假如苹果做番茄钟」质感；计时/统计唯一真相在 Rust，UI 只显示+转发；无安装包/自启/云同步/任务管理/白噪音。
- 环境基线 ✅：rustc 1.96.0 / node v24.18.0 / bun 1.3.14；vite 端口 1430（1420 被占）；CDP 9223
- 原任务 0-4 ✅（2026-08-12 上午）：Rust 状态机+悬浮窗+统计，详见旧 HANDOFF 档
- UI 优化 1-7 ✅（2026-08-12 下午）：
  - 自动切换开关：`TimerConfig.auto_start_next` 默认 false，cargo test 22 绿
  - Rust 逐帧窗口动画：ease-out cubic SetWindowPos，~220ms
  - 齿轮抽屉：设置区移入 drawer，覆盖式滑入
  - 展开页拖拽：attachThresholdDrag 绑 panel-head
  - 控件居中：隐藏原生 spinner，text-align:center
  - 跳过→切换：多功能键「☕休息/▶专注」随阶段动态
  - 应用图标：番茄红圆角矩形+白色时钟指针，tauri icon 全套生成
- UI 优化 #8 视觉评审 ⏳：截图已采集，评审子智能体运行中
- 待办：视觉评审修 P1 → 连续 2 轮无 P1 达标
- 关键技术资产：`scripts/cdp.mjs`（WebView2 CDP，:9223）、`scripts/win.ps1`（窗口/输入注入）、`scripts/capture-round.ps1`（四态截图采集）；Rust 拖拽线程（drag）+ 动画线程（anim，代际令牌防重入）；自研阈值拖拽（6px 判定）
- 改动记录：`docs/DEV_LOG.md`