---
type: solution
title: box-shadow 简写覆盖会整棵替换：杀外阴影时把液态 inset 光影一并陪葬
date: 2026-08-15
category: ui-bugs
module: styles
problem_type: css_pitfall
component: css-material
severity: low
symptoms:
  - "托盘菜单页液态档与经典档肉眼无差（v13 任务起点「菜单看不出材质变化」的成因之一）"
  - "主窗与托盘页同一组件不同环境观感不一致（主窗有液态光影、托盘页没有）"
root_cause: css_specificity_override
resolution_type: code_fix
related_components:
  - "src/styles.css"
  - "tray-menu.html"
tags: [css, box-shadow, shorthand, specificity, override, liquid-glass, tray-menu]
---

# box-shadow 简写整棵覆盖：一处覆盖段废掉整棵液态签名

## Problem

`.ctx-menu` 液态档在主窗有完整玻璃光影（顶高光/底厚度/描边，四层 inset），托盘菜单页
同一组件却毫无液态笔触——两菜单共用 `.ctx-menu` 类，观感却不一致，v13 起点「切材质菜单
看不出变化」的部分成因即此。

## Root Cause（CSS 层叠语义 + v11 历史决策的合谋）

`box-shadow` 是**简写属性**：任何一处更高特异性/更靠后的 `box-shadow` 声明都是**整棵替换**，
不是逐层合并。v11 建托盘菜单页时，为解决「窗口与菜单同尺寸、外阴影无处渲染会被裁成角部
黑边」（styles.css 托盘页覆盖段注释），写了：

```css
.tray-menu-page .ctx-menu, ... { box-shadow: inset 0 0 0 0.5px var(--hairline); }
```

这一刀切掉外阴影（当时正确），但整棵替换的语义把 v6.1 起液态档 `.ctx-menu` 的全部 inset
光影层也一并抹掉——**被禁的只是外阴影（溢出窗外渲染），inset 光影在窗内渲染、根本不裁角**。
坑埋于 v11（当时液态菜单只有两层弱 inset，损失不可见），现形于 v13（液态签名加强后差异
变成「一眼无玻璃」）。

## Solution（v13 补位规则）

覆盖段只声明**确实要改的层**做不到（简写无逐层增删），所以补两条**液态专属同覆盖位规则**，
把 inset 光影按主规则同款写回，外阴影仍不出现：

```css
[data-material="liquid_glass"] .tray-menu-page .ctx-menu {
  box-shadow: inset 0 1px 0 rgba(255,255,255,.9), inset 0 -1.5px 2.5px rgba(0,0,0,.2), ...;
}
```

并在**主规则与覆盖段两处注释互相声明「同步维护」**——配方再调时必须同改两处。

## Prevention

- 写 `box-shadow` 覆盖规则前先问：我要去掉的是**哪一层**？简写做不到「只删一层」，
  被保留下来的层必须显式重写进覆盖规则。
- 判定「禁某层」的物理原因是否同样适用于其他层（v11 案例：外阴影裁角 ✓ 该禁；
  inset 窗内渲染 ✗ 不该连带）。
- 同组件跨环境（主窗/独立窗/打印）共用样式时，覆盖段改动后**两个环境都截图对比**——
  本坑潜伏两版（v11→v12）无人在托盘页切过液态材质细看，直到 v13 才现形。

## 证据

- 修复：src/styles.css 托盘页覆盖段两条液态专属规则（v13，含同步维护注释）
- 复验：docs/screenshots/v13/v13-pair-tray-{light,dark}.png（修复后托盘页液态签名在场，
  round-44/45 评审像素级确认与主窗菜单同语言）
