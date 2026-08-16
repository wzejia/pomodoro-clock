# Liquid Glass · Web 实现参数规格（v6.1）

> 来源：苹果官方开发者文档 JSON API + WWDC25 Session 转录，2026-08-13 由两个独立子智能体抓取。
> 每条结论带出处；「官方未给」的项明确标注，禁止用「行业惯例」补。
> 抓取方法备注：HIG 页 JSON 正确前缀是 `/tutorials/data/design/human-interface-guidelines/<slug>.json`
> （`/tutorials/data/documentation/human-interface-guidelines/*` 为 404）；WWDC 转录经视频页 Transcript 标签 DOM 抓取。

## 1. 材质变体语义（regular / clear / identity）

| 变体 | 官方定义（原文） | Web 转译 |
|---|---|---|
| regular（默认） | "The regular variant blurs and adjusts the luminosity of background content to maintain legibility of text and other foreground elements." | `backdrop-filter: blur()` + 亮度/饱和调节层；**可读性优先于通透度** |
| clear | "The clear variant is highly translucent… Only use clear Liquid Glass for components that appear over visually rich backgrounds." | 极低填充透明度，仅用于媒体背景上；**本应用无媒体背景 → 不用 clear** |
| identity | "The identity variant of glass. When applied, your content remains unaffected as if no glass effect was applied." | 等价于关闭玻璃（我们的「经典」档语义相近） |

- **clear 在亮背景上的 35% 暗层**（HIG Materials 页正文唯一数值参数，反向验证点 #1）：
  > "If the underlying content is bright, consider adding a dark dimming layer of **35% opacity**."
  > "If the underlying content is sufficiently dark… you don't need to apply a dimming layer."
  （出处：https://developer.apple.com/tutorials/data/design/human-interface-guidelines/materials.json ）
  SwiftUI 代码示例用 `.black.opacity(0.3)`（示意值，非规范；出处 swiftui/glass/clear.json）。
- 默认形状：**Capsule**——"SwiftUI uses the regular variant by default along with a Capsule shape."
  （出处：https://developer.apple.com/tutorials/data/documentation/swiftui/view/glasseffect(_:in:).json ）

## 2. 克制原则：玻璃只属于功能层

- > "Liquid Glass forms a distinct functional layer for controls and navigation elements — like tab bars and sidebars — that floats above the content layer."
  （出处：materials.json，同上）
- > "**Don't use Liquid Glass in the content layer.**"
  （出处：materials.json，同上）
- > "Avoid overusing Liquid Glass effects… Limit these effects to the most important functional elements in your app."
  （出处：https://developer.apple.com/tutorials/data/documentation/technologyoverviews/adopting-liquid-glass.json ）
- > "Similarly, **always avoid glass on glass**. Stacking Liquid Glass elements on top of each other can quickly make the interface feel cluttered and confusing."
  （出处：WWDC25-219 转录 13:13，https://developer.apple.com/videos/play/wwdc2025/219/ ）

**本应用映射**：外壳（迷你胶囊/展开面板底）+ 右键菜单 + 抽屉 = 功能层 → 上玻璃；
统计卡、周图、设置分组卡 = 内容层 → **不上玻璃**，用更实的 standard material 保可读（official 四档：ultra-thin/thin/regular/thick，出处 materials.json Platform considerations）。

## 3. 光学特性官方描述（高光/折射/边缘光）

- 总定义：> "…a new dynamic material called Liquid Glass, which combines the optical properties of glass with a sense of fluidity."
  （出处：https://developer.apple.com/tutorials/data/documentation/technologyoverviews/liquid-glass.json ）
- 渲染行为：> "Liquid Glass is a material that blurs content behind it, **reflects color and light of surrounding content**, and reacts to touch and pointer interactions in real time."
  （出处：https://developer.apple.com/tutorials/data/documentation/swiftui/applying-liquid-glass-to-custom-views.json ）
- 高光来自虚拟环境光源：> "Light sources inside of this environment shine on the material producing highlights that respond to geometry just as you'd expect."
  （出处：WWDC25-219 转录 10:44）
- 环境光溢色：> "Light from colorful content nearby can subtly spill onto its surface… the light reflects, scatters, and bleeds into the shadow as well."
  （出处：WWDC25-219 转录 8:09）
- 透镜效应（lensing）：> "…highlighting the Liquid Glass material's lensing effect."（HIG accessibility.json 内卡片图 alt 文本）
- 阴影随内容自适应：> "The element is aware of what's behind it and increases the opacity of its shadow when it is over text."
  （出处：WWDC25-219 转录 11:47）

**注意**：specular highlight / edge light / bloom 的逐词规格在公开 JSON 文档中**不存在**（正文零出现）——
Web 端只能按「reflects color and light」原则拟态：顶部镜面高光 + 内侧边缘光 + 底部内阴影纵深。
"reflection, refraction, highlights" 仅出现在 **App 图标**语境（adopting-liquid-glass.json App icons 节），不可当界面材质规格引用。

## 4. 同心圆角（concentricity）

三种形状类型（WWDC25-356 转录 3:21–3:49，https://developer.apple.com/videos/play/wwdc2025/356/ ）：
> "Fixed shapes have a constant corner radius. **Capsules use a radius that's half the height of the container. And concentric shapes calculate their radius by subtracting padding from the parent's.**"

- 规则（反向验证点 #2）：**R_inner = R_outer − padding**（concentric）；capsule：R = height/2（CSS `999px`）。
- 嵌套发紧/外扩 = 未同心：> "If something feels off… its shape probably needs to be concentric."（356 转录 5:29）
- 玻璃控件与窗口圆角同心：> "Glass controls nest perfectly into the rounded corners of windows, maintaining concentricity throughout the UI."（219 转录 7:53）
- API 佐证：`ConcentricRectangle` / `UICornerConfiguration`（adopting-liquid-glass.json Controls 节）。

**本应用映射**：迷你胶囊 = capsule（R=38=76/2 已满足）；展开面板 R=22、内卡 R=14、面板内边距 18 →
同心值应为 22−18=4（过小，观感差）→ 说明面板-卡片间距本就不是同心嵌套场景（卡片不贴面板角），
同心规则适用于**抽屉滑入层**（贴面板边缘）与**右键菜单项**（菜单 R=10、项 R=6、padding 4 → 6=10−4 ✓ 已满足）。

## 5. 动效原则

- 出现/消失靠调光不透明度以外的「物质化」：> "Instead of fading, Liquid Glass objects materialize in and out by gradually modulating the light bending and lensing."（219 转录 2:55）
  → Web 近似：玻璃层的 backdrop/highlight 渐变优先于纯 opacity 淡入淡出（我们已有交叉淡入，保留即可——CSS 无 lensing 可调）。
- 交互即时反馈：> "Liquid Glass responds to interaction by instantly flexing and energizing with light."（219 转录 3:38）
  → 已有 :hover scale(1.08)/:active scale(0.92) + 弹簧，符合。
- 凝胶弹性随手势：> "an inherent gel-like flexibility… as it moves in tandem with your interaction"（219 转录 3:51）
  → 拖拽跟手已有（Rust 8ms 线程）。
- 跨状态 morph：> "Liquid Glass dynamically morphs between the controls in each context… the bubble simply pops open."（219 转录 4:32）
  → 我们 340ms WAAPI 弹簧 morph 正合此意，**时长/曲线官方未给数值**（全部来源零毫秒数，勿编造）。
- HIG Motion 总则：> "Don't add motion for the sake of adding motion." / "don't make people wait for an animation to complete"（motion.json）
  仅有数字：帧率 30–60fps（motion.json）；避免 ~0.2Hz 振荡（visionOS 节）。

## 6. 可访问性退化

官方退化行为（WWDC25-219 转录 18:22，逐字）：
> "Reduced Motion decreases the intensity of some effects and disables any elastic properties for the material."
> "**Reduced Transparency, makes Liquid Glass frostier and obscures more of the content behind it.** Increased contrast, makes elements predominantly black or white and highlights them with a contrasting border."

- HIG 自定义动效退化清单：收紧弹簧减回弹、位移改 fade、禁 z 轴深度动画、禁 blur 淡入淡出
  （出处：https://developer.apple.com/tutorials/data/design/human-interface-guidelines/accessibility.json ）
- **Web 对应物**：
  - `@media (prefers-reduced-transparency: reduce)` → 玻璃层 alpha 提到近不透明（"frostier"），去 backdrop-filter；
  - `@media (prefers-contrast: more)` → 元素近纯黑/白 + 对比描边；
  - `@media (prefers-reduced-motion: reduce)` → 弹簧改线性短 fade（morph 340ms 降为 opacity 交叉）。

## 7. 明暗主题行为

- 材质**连续响应背景亮度**，非两套固定配方：> "the regular variant of Liquid Glass… appears darker when there is a dark background beneath it / lighter when there is a light background beneath it."（materials.json references 图注）
- visionOS 佐证：> "glass automatically adapts to the luminance of the objects and colors behind it."（materials.json visionOS 节）
- 内容滚过时动态调节：> "The amount of tint and the dynamic range shift to always ensure buttons remain legible."（219 转录 6:42）

**本应用映射**：透明 WebView2 糊不到桌面（R18 实证），「背景亮度」不可知 →
明暗两套配方按主题固定（合理偏离，已在§9 声明），alpha 取值保守偏高保可读。

## 8. 数值参数总表（官方仅这些，逐字引用）

| 参数 | 值 | 出处 |
|---|---|---|
| clear 变体亮背景暗层 | 35% opacity 黑色 | materials.json（HIG 正文唯一数值） |
| SwiftUI clear 示例暗层 | black.opacity(0.3)（示意非规范） | swiftui/glass/clear.json |
| 默认形状 | Capsule（R=height/2） | swiftui/view/glasseffect(_:in:).json；356 转录 3:49 |
| 同心圆角 | R_inner = R_outer − padding | 356 转录 3:21–3:49 |
| 示例圆角 | 16.0（代码示例） | applying-liquid-glass-to-custom-views.json |
| 容器融合间距示例 | 40.0（GlassEffectContainer） | applying-liquid-glass-to-custom-views.json |
| 动效帧率 | 30–60 fps | motion.json |
| 动效时长/弹簧参数 | **官方未给** | 全来源零出现 |

## 9. 偏离声明（Web 拟态的必然妥协）

1. 透明 WebView2 的 backdrop-filter 只糊页内内容、糊不到桌面（R18 实证）→ 玻璃的「模糊背景」
   用分层渐变 + 半透明底色**拟态**，alpha 取 0.55–0.85 区间保文字可读（官方 regular 变体
   「maintain legibility」原则优先于通透度）。
2. 明暗行为：官方是连续响应背景亮度；我们背景不可知 → 按 data-theme 两套固定配方。
3. 高光/边缘光强度无官方数值 → 自校准，评审循环把关。
4. 「materialize by modulating lensing」无 CSS 对应物 → 保留既有交叉淡入 + 340ms 弹簧 morph。
