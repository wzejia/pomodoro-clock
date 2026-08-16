# Round 36 评审（v9 第 1 轮：按钮 hover/active 去缩放化）

评审方式：独立子智能体（opus），只给 25 张 4x/5x 放大裁剪截图 + 修复前对照 + Apple HIG 标准，无实现上下文。

## 采集说明

- 管线：tmp/v9-hover.mjs（单 WS 会话 Input.dispatchMouseEvent 真实悬停/按压，+40/+120/+700ms 定时帧；按压帧移开再松手不触发点击）+ tmp/v9-capture.ps1（驱动+裁切放大）
- 命名：round-36-r36-{btn}-{theme}-{moment}-xN.png；修复前基线 round-36-pre-*（Tag=pre）
- 按压证据不改计时状态（release 在按钮外，无 click 触发）

## 评审原文（结论摘要）

- 基线确认：round-36-pre-btn-primary-mid1「开始」边缘明显发毛有重影（位图重采样特征），end 帧锐利——历史问题属实复现。
- **判据 A（文字全程锐利）：通过**——全部按钮组 mid1/mid2/end 三帧锐利度一致，无发毛/重影；mini 播放三角、step-btn「−」、collapse chevron 边缘干净。
- **判据 B（反馈可感知且克制）：通过**——浅色 hover 微沉、深色提亮，方向符合 Apple 惯例；按压红色明显变暗、按压感明确；幅度克制不分散注意力。
- 顺查：圆角/描边/高光/glyph 无异常、无色带、无变色。
- **P1：无。P2：无。**
- P3-1：btn-switch「休息」glyph 阶梯状锯齿，与「重置」渲染质量不一致（修复前 pre 帧同样存在，既存问题）。
- P3-2：浅色 btn-reset hover 明暗 delta 处静帧可辨识下限（评审自评「当前不视为缺陷」）。
- 总体：无 P1 无 P2，改动达标。

## 分诊与处置

- P3-1 不修关闭：btn-switch 标签 `.switch-label` font-size 13px vs 其他按钮 14px——13px 汉字笔画像素更少，4x 最近邻放大下天然更显锯齿；pre 帧（旧 scale 方案结束态）同样存在证明与变换无关，1x 物理尺寸两按钮渲染均正常。非本次改动范围，不扩 scope。
- P3-2 不修：评审自评不视为缺陷；hover 0.96/1.15 幅度对齐 macOS 惯例，加大有「闪烁感」风险。

→ 进 round-37 独立复审（同一改动零代码变更，重新采集 r37 全套截图，新评审员），再无 P1 即收敛。
