# Round 27 评审（v6.1 首轮 · 液态玻璃）

## 原始输出（独立评审子智能体，仅给八态截图 + Apple HIG 标准）

**结论：本轮有 P1（1 条），不达标。**

- P1-1 round-27-ctx.png / ctx-dark.png —— 右键菜单完全未出现（与 panel 截图像素级一致）；采集未触发或实机入口失效，需排查
- P2-2 drawer 浅色 —— 设置分组卡片边界不可见，与面板底零对比，明暗「分组感」不同构
- P2-3 drawer 两态 —— 节标题（循环/外观）与上方卡片组间距过近，层级归属歧义
- P3-4 mini 浅色 —— 暂停白圆片对比偏低（抽屉返回键同源）
- P3-5 panel 浅色 —— 重置/休息白按钮存在感弱于深色态
- P3-6 进度条下圆点语义不直观（建议拉开间距/已完成填红）
- 通过项：高光边缘光克制、明暗同构、颜色纪律、同心圆角、可读性全部达标

## 分诊

- P1-1：**采集事故，非产品 bug**。根因：cdp-seq 的 Runtime.evaluate 顶层 `const m` 在页面全局词法环境残留，第二次跑采集时报重声明 SyntaxError，被 Seq 的 `2>$null` 吞掉 → 菜单从未打开。实证：手动 IIFE eval 开菜单正常（opacity 1/visible/rect 正确）。修：Ctx 改 IIFE。本轮用首跑被覆盖后的截图，两态 ctx 均受影响
- P2-2 成立 → 立修：玻璃抽屉内 group-card 纯白 0.98 + 发丝描边（iOS 分组列表白卡浮灰底）
- P2-3 成立 → 立修：`[data-material="liquid_glass"] .drawer .section-caption { padding-top: 10px }`（玻璃作用域，不动经典）
- P3-4/5 同源 → 顺手修：浅色 chip 发丝描边 0.08→0.12
- P3-6：循环圆点语义为 v6 领导拍板设计（完成导向），「已完成填红」本就有（lit 态）；涉既有设计 → BLOCKED 留裁

截图：round-27-{mini,panel,drawer,ctx,panel-dark,drawer-dark,ctx-dark,mini-dark}.png + round-27-classic-*.png（经典对照八态，切经典回旧观感实证）
