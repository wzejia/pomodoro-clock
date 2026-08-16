# Round 28 评审（v6.1 第二轮）

## 原始输出（独立评审子智能体，仅给八态截图 + Apple HIG 标准，已排除上轮留裁项）

**结论：本轮有 P1（2 条），未达收敛。**

- P1-1 panel-dark —— 「暂停」主按钮深色下失去红色填充，与重置/休息同为灰描边按钮，明暗不同构
- P1-2 drawer 两态 —— 「自动开始下一阶段」绿色开关右半被面板右边缘裁切
- P2-3 drawer 两态 —— 背后计时数字残影透过抽屉玻璃可读（深色更明显）
- P2-4 周图日标签 —— 「四」色阶比「一二三」浅一档（两态）
- P3-5 深色内容卡与面板玻璃亮度差小，层级分离弱于浅色
- P3-6 mini-dark 红色「专注」标签对比略沉
- 通过项：胶囊两态同构、高光克制、右键菜单层级、上轮修复保持、内容层实料原则执行到位

## 分诊

- P1-1 **成立，真 bug**：`[data-theme="dark"][data-material="liquid_glass"] .btn` 权重 (0,3,1) 盖过 `[data-material="liquid_glass"] .btn-primary` (0,2,1) → 深色 work 态主键失红（short/long_break 的 phase 变体权重 0,4,x 不受影响，故仅 work 态中招）。修：深色域补同权重 .btn-primary 规则
- P1-2 **误读**（crop + 几何实测双重实证）：展开+抽屉开同状态下 toggle right=308 < card right=322， knob right=306，完整无缺。观感根因=白旋钮融进白卡。顺手修：旋钮加 0.5px 发丝描边
- P2-3 成立 → 抽屉底 0.97→全实（rgb 不带 alpha），sheen 保留；3% 透字对大字号白色计时数字已可辨
- P2-4 **误读**（crop 实证）：一~四、六日同 --text-tertiary，五为 today 加粗，无第四档色阶
- P3-5 顺手修：深色内容卡 84/0.55 → 94/0.6 提亮一档
- P3-6 → BLOCKED（既有配色，--work 深色为系统红 #ff453a）

截图：round-28-*.png（八态）；复核裁剪图在 tmp（toggle-crop*.png、weeklabels-crop.png）
