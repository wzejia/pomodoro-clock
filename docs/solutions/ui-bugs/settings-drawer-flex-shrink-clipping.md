---
type: solution
title: 设置抽屉分组卡被 flex-shrink 压矮裁切控件（overflow:hidden 子项 min-height:auto=0 陷阱）
date: 2026-08-14
category: ui-bugs
module: settings-drawer
problem_type: ui_bug
component: frontend_stimulus
severity: high
symptoms:
  - "设置抽屉「循环」分组卡的 toggle 开关被拦腰裁断（36px 轨道只剩 14px，底部平切）"
  - "「时长」分组卡第三行只剩 8px 残条"
  - "明暗两主题在同一坐标复现"
  - "v6.1 给外观组新增「材质」行后开始显现（v6 起已隐性裁切行 padding）"
root_cause: logic_error
resolution_type: code_fix
related_components:
  - "src/styles.css (.drawer / .settings / .group-card / .setting-row)"
tags: [css, flexbox, flex-shrink, min-height-auto, overflow-hidden, settings-drawer, layout-clipping, tauri]
---

# 设置抽屉分组卡被 flex-shrink 压矮裁切控件

## Problem

固定尺寸悬浮窗（Tauri 2 + Vanilla TS，展开面板 340×456px）的设置抽屉里，「循环」分组卡中的 iOS 风格 toggle 开关被拦腰裁断，「时长」分组卡末行只剩一条残条。抽屉 `.drawer` 以 `inset: 0` 覆盖面板（src/styles.css:596-610），内容可用高度约 368px；抽屉内 `.settings` 是 flex 列容器（`flex: 1; justify-content: space-evenly`，src/styles.css:624-629），内含若干 `.group-card` 分组卡（`overflow: hidden`，src/styles.css:631-635，用于把行内 hairline 分隔线裁到圆角内）。

## Symptoms

- 视觉评审 round-29 截图实证：「循环」卡的 toggle 轨道（设计尺寸 36×22px）只剩 14px 高，底部被水平平切。
- 「时长」分组卡第三行（长休息）只剩约 8px 残条。
- 明暗两个主题在同一 y 坐标复现，排除主题变量问题。
- 实测每张卡 `getBoundingClientRect().bottom − 末行.bottom` 为负值（−13 / −8 / −9px），即行尾被卡片边界裁掉。
- 误导性阴性信号：`.settings` 容器 `scrollHeight === clientHeight`，看起来「无溢出」。

## What Didn't Work

**用 `scrollHeight === clientHeight` 判断「容器没有溢出所以内容没被裁」——这是错误结论。** 机制上：flex 子项默认 `flex-shrink: 1`，当内容自然高度超过容器时，浏览器会把子项**压矮**来避免容器 overflow，所以容器永远「不溢出」——但子项自身高度已小于其内容高度，配合 `overflow: hidden` 就把行尾裁掉了。容器级溢出检测对 shrink 型裁切完全失效，第一轮排查因此误判「布局没有高度问题」，走了弯路。

也没有选择这些修法：

- **给 `.group-card` 加 `flex-shrink: 0` 或 `min-height: min-content`**：能止住裁切，但总内容高（~440px）超过可用高度（368px）约 70px，硬禁止收缩只会让卡片溢出抽屉边界，把问题从「裁切」变成「溢出」，不治本。
- **给 `.settings` 加 `overflow-y: auto`**：固定 340×456 的悬浮窗里出现滚动条违背「一屏看完的设置页」设计目标，且 Apple 风格设置页在内容可预期时不该滚动。
- **压缩 toggle/字号等控件尺寸**：伤控件可用性，且只省几个 px，不够填 70px 的缺口。

历史背景 (session history)：抽屉从初建起就定位是「固定面板内的滑入覆盖层，永不加高窗口」——2026-08-12 初建会话明确「窗口尺寸恒为展开态固定值，抽屉与主内容互斥」，所以内容增长只能挤压自身纵向空间，没有「内容撑高窗口」这条退路。面板高度在生命周期内还收紧过（600→441→456），高度预算一直在减。而且此前已有一次同型事件：2026-08-13 长休息会话给抽屉加过两行，当时余量还够、靠截图评审兜底通过——抽屉布局从无自动化尺寸断言，尺寸问题全靠逐轮截图人眼捕捉 (session history)。

## Solution

只回收垂直预算、不动布局结构（不改 flex 策略、不加滚动、不动控件），四处纯数值调整（src/styles.css）：

```css
/* before */
.drawer {
  padding: 20px 18px;
  gap: 10px;
}
.drawer-head {
  margin-bottom: 10px;
}
.drawer .setting-row { padding: 12px 14px; }

/* after（现状，见 src/styles.css:603-604, 621, 639） */
.drawer {
  /* R29 P1：内容高超预算 ~30px → padding 20→14、gap 10→6 回收 18px */
  padding: 14px 18px;
  gap: 6px;
}
.drawer-head {
  /* R29 P1：高度预算回收（10→4） */
  margin-bottom: 4px;
}
.drawer .setting-row { padding: 9px 14px; }  /* 12→9，7 行省 ~42px */
```

合计回收约 60px+，使内容自然高度回落到 368px 预算内，`flex-shrink` 不再触发，裁切自然消失。

修后实测验证（非目测）：行高恢复 43/44px，toggle 完整 22px 高，卡内底部余量 9px；明暗两主题、两种 material 下同一断言复跑通过。

## Why This Works

根因链：v6.1 给外观组新增一行「材质」设置 → 抽屉内容自然高度（~440px）超过可用高度（368px）→ 按 CSS 规范，**`overflow` 非 `visible` 的 flex 子项，`min-height: auto` 计算为 0 而不是 min-content** → 默认 `flex-shrink: 1` 把 `.group-card` 压到内容高度以下 → 卡片自身的 `overflow: hidden` 裁掉行尾。

关键点在于这个隐性裁切**自 v6 就存在**：当时余量还够，被裁掉的只是末行的 padding，控件没受伤，连续两轮视觉评审都没发现；v6.1 加一行推过临界点，裁切第一次吃进控件本体（toggle 被腰斩）。所以这不是「新 bug」而是「存量隐患被一行新内容引爆」。

修复有效是因为它对因施策：问题不是「卡片不该收缩」（收缩是空间不足时的合理行为），而是「空间不该不足」。把垂直 padding/gap/margin 这些纯外观余量回收约 60px，内容高度回到预算内，shrink 条件不再成立，既不需要禁收缩（那会引入溢出），也不需要滚动（破坏设计）。同时四处改动全是数值、不动选择器结构，明暗主题与 liquid_glass material 的覆盖规则（src/styles.css:715+）全部自然继承，无主题分支风险。

## Prevention

**1. 固定高容器加一行内容前，先量预算再动手。** 对 `固定高容器 + flex 列 + overflow:hidden 子项` 这类结构，新增任何行之前用一条 console 命令量余量：

```js
// 在 devtools / CDP evaluate 中跑（本项目用 scripts/cdp.mjs eval）
const avail = document.querySelector('.drawer .settings').clientHeight;
const need = [...document.querySelectorAll('.drawer .group-card')]
  .reduce((s, c) => s + c.scrollHeight, 0);
console.log({ avail, need, headroom: avail - need });
```

`headroom < 新行高度` 就先回收预算（减 padding/gap）再加行，不要加了再说。

**2. 不要用 `scrollHeight === clientHeight` 排除 shrink 裁切。** flex 子项收缩恰恰是为了让容器不 overflow——容器「不溢出」与子项「内容被裁」可以同时成立。可靠的断言是逐卡比较卡片边界与末行边界：

```js
[...document.querySelectorAll('.drawer .group-card')].map(card => {
  const last = card.querySelector('.setting-row:last-child');
  return card.getBoundingClientRect().bottom - last.getBoundingClientRect().bottom;
});
// 任何负值 = 该行被裁；本次实测修复前为 [-13, -8, -9]，修复后全为正
```

这条断言应进视觉评审的常规检查清单（配合 capture-round.ps1 截图轮次一起跑），比人眼看截图更早发现「只裁到 padding」的潜伏期。

**3. 记住这条规范事实：** `overflow` 非 `visible` 的 flex 子项，`min-height: auto` 计算为 0。只要卡片带 `overflow: hidden`（裁 hairline 到圆角内的常见手法），它在 flex 列里就是「可被压到任意矮且安静裁内容」的——裁切无声无息，只有逐元素几何断言能可靠暴露。

**4. 潜伏期信号也要记录。** 当几何断言显示余量为正但很小（如 < 一行 padding）时，说明正处在「裁切只吃 padding」的潜伏期，下一行新内容就会引爆——此时应主动回收预算，而不是等 P1 截图。

## Related Issues

- `docs/review/round-29.md` — 本 bug 的评审原始档案（P1-1/P1-2 + 分诊），裁剪证据图 tmp/r29-toggle-zone.png
- `docs/review/round-28.md` — 前史反例：R28 P1-2「toggle 被右缘裁切」经 crop+几何实测证为误读（白旋钮融白卡视错觉）；横向误读与本轮纵向 shrink 裁切机制不同，同属抽屉裁切类 finding
- `BLOCKED.md` #8 — 同题一行摘要（环境条目），沉淀后指向本文档
- 抽屉高度预算演变史 (session history)：2026-08-12 初建（340×600、抽屉永不加高窗口的决策）→ 600→441→456 收紧 → 2026-08-13 长休加两行（首次同型事件）→ 2026-08-14 材质行引爆
