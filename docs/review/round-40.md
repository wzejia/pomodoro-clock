# round-40 评审档案（v11 首轮）

- 评审员：opus（独立子智能体，只给截图+判据+HIG 口径，未给实现细节）
- 采集：scripts/v11-capture.ps1 -Round 40（九态：液态八态+经典迷你；1 分钟临时配置让胶囊进度条可见，采集后恢复原 config/stats 哈希未变）
- 素材：round-40-{mini,panel,drawer,ctx}{,-dark}.png、round-40-classic-mini.png + docs/screenshots/v11/traymenu-popup{,-dark}.png、traymenu-autostart-on.png、progress-{work-mid,break-mid,paused}-zoom.png、hotkey-occupied-error.png

## 评审结论（原文摘要）

- 判据 1 托盘弹层一致性：同族成立（圆角/行高/勾选槽/hover 语言），边缘干净（四角 6x 放大无裁切无 AA 瑕疵），深色档成立；唯一例外=首项 hover 描边环 → P2-1
- 判据 2 抽屉快捷键行：成立（行密度与既有一致、与 round-39 比对旧行零位移、键帽 chip 质感达标、错误态克制、× 可达）
- 判据 3 胶囊 2px 进度条：成立（贴底边内缩对称、不破坏圆角高光、红绿语义清晰、四组合可见、暂停定格无残缺；AA 阶梯 4x 下正常 1x 不可见，主动不报）
- 判据 4 常规扫描：通过（无发毛/裁切/失对比；ctx 遮统计卡数字属浮层正常遮挡）
- **P1：无。P2×1。P3：无。总体：可发布。**

## P2-1（已立修闭环）

- 现象：traymenu-popup{,-dark}.png 首项 hover 蓝底外紧贴 ~2px 描边环（浅色黑/深色白，6x 最近邻确认），像 Windows focus visual，苹果语境显噪。
- 诊断（修复子智能体，全实测）：环=UA `:focus-visible` 焦点环（DOM 焦点跨 hide/show 残留在首项，可信键盘输入翻转键盘模态后残留焦点命中 :focus-visible）；蓝底=残留合成 `:hover`（托盘页常驻不 reload，CDP 合成输入留下的渲染器内鼠标位置跨 hide/show 存活）。干净态弹出本无环无蓝底——评审截图是长命 dev 实例被自动化污染的残留态。
- 修复（2 处最小改动）：styles.css `.tray-menu-page` 段追加 `.ctx-item:focus/:focus-visible { outline: none }`（visual-only）；tray-menu.ts `tray-menu-opened` 首行 blur 残留 activeElement。
- 验证：重采 popup/dark 两档无环无假蓝底；弹出时 activeElement=BODY、首项 :hover/:focus 全 false；合成 move 真悬停仍出 accent 蓝（hover 语言保留）；人为污染→重开→blur 生效；可信 Tab 键盘聚焦也无环（outline:none）。tsc clean。
- **附带实证（修复员遗留疑问的裁决）**：「透明菜单窗收不到真实 OS 鼠标输入」不成立——scripts/v11-realinput.ps1：SetCursorPos+mouse_event 相对 ±1px 微移 → 真实 :hover 命中「设置…」（anyHover [false,true,false,false]）；真实左键点击「设置…」→ 主窗展开+抽屉开+菜单自关（open-settings 全链）→ REAL-INPUT PASS。修复员观察到的零命中系 SetCursorPos 本身不产生 WM_MOUSEMOVE 所致（Windows 行为，非输入死亡）。教训：菜单页证据采集前须 `move 1 1` 归位或重启实例清残留合成 hover。
- 修复后进 round-41 复审（round-40 已无 P1，round-41 再无 P1 即连续两轮收敛）。

## 分诊记录

- P1 无。P2-1 立修（改动小收益直接，且是托盘弹层唯一「不苹果」笔触感来源）。P3 无。
- 评审员主动放弃项：✓ 灰色（克制一致）、进度条端点 AA（1x 不可见）——记录免重复报。
