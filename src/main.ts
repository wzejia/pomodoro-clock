import { bridge, TimerSnapshot, StatsSummary, MonthSummary, TimerConfig } from "./bridge";

// 调试：把页面错误回传 Rust 侧 dev.log
window.addEventListener("error", (e) => {
  try { (window as any).__TAURI__?.core?.invoke("log_js_error", { message: `${e.message} @ ${e.filename}:${e.lineno}` }); } catch {}
});
window.addEventListener("unhandledrejection", (e: PromiseRejectionEvent) => {
  try { (window as any).__TAURI__?.core?.invoke("log_js_error", { message: `unhandledrejection: ${e.reason}` }); } catch {}
});

const MINI_W = 220, MINI_H = 76;
// PANEL_H=467（2026-08-14 v11 CDP 实测：加快捷键卡后抽屉自然内容 386px=78 标题+308 卡；
// 456 下 space-evenly 缝隙仅 0.5px 近乎相贴，放大后 2px 档；v8 公式逐点复核：
// 467→末卡底 451.1→ceil(+14+1)=467 自洽不动点，面板底部留白 +11px 在任务书 +16 预算内）。
// 内容固定（统计/周图高度恒定），内容变动需重测
const PANEL_W = 340, PANEL_H = 467;

const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;

const app = $("app");
const pillTime = $("pill-time");
const panelTime = $("panel-time");
const panelPhase = $("panel-phase");
const progressBar = $("progress-bar");
const btnToggleRun = $("btn-toggle-run") as HTMLButtonElement;
const iconPlay = $("icon-play") as unknown as SVGElement;
const iconPause = $("icon-pause") as unknown as SVGElement;
const btnPrimary = $("btn-primary") as HTMLButtonElement;
const btnSwitch = $("btn-switch") as HTMLButtonElement;
const switchLabel = btnSwitch.querySelector(".switch-label") as HTMLElement;
const cycleDots = $("cycle-dots");
// v11：胶囊底部 2px 进度条（与面板 progress-bar 同一 snapshot 数据源）
const pillProgressBar = $("pill-progress-bar");
// v11：快捷键改键（键帽=录制入口/当前生效键展示，×=一键关闭）
const hotkeyCap = $("hotkey-cap") as HTMLButtonElement;
const hotkeyOff = $("hotkey-off") as HTMLButtonElement;

const PHASE_LABEL: Record<string, string> = {
  work: "专注",
  short_break: "短休息",
  long_break: "长休息",
};

let snapshot: TimerSnapshot | null = null;
let expanded = false;
let notifiedPermission = false;

/* mock 预览模式：给个桌面感背景 */
if (bridge.isMock) document.body.classList.add("mock-preview");

function fmt(ms: number): string {
  const total = Math.ceil(ms / 1000);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

function render(s: TimerSnapshot) {
  snapshot = s;
  const time = fmt(s.remaining_ms);
  pillTime.textContent = time;
  $("pill-phase").textContent = PHASE_LABEL[s.phase] ?? s.phase;
  panelTime.textContent = time;
  panelPhase.textContent = PHASE_LABEL[s.phase] ?? s.phase;
  app.dataset.phase = s.phase;
  app.dataset.status = s.status;
  const done = s.total_ms > 0 ? 1 - s.remaining_ms / s.total_ms : 0;
  // v8①：进度 ≤2% 不渲染填充（含 min-width 圆头）——刚起步的孤立红圆点语义弱、
  // 且 5px 圆头与轨道左端头像素级错位被读作「可拖拽旋钮/凸出轨道」；空轨道才是干净的未开始态
  const showBar = done > 0.02;
  progressBar.style.width = showBar ? `${(done * 100).toFixed(1)}%` : "0";
  progressBar.style.minWidth = showBar ? "" : "0";
  // v11 胶囊底部 2px 条：同一 done（snapshot remaining/total），暂停时 remaining 冻结即停；
  // 无 min-width 圆头问题，不设 2% 阈值——2px 高度的短段只读作「刚起步」
  pillProgressBar.style.width = `${(done * 100).toFixed(1)}%`;

  const running = s.status === "running";
  iconPlay.style.display = running ? "none" : "";
  iconPause.style.display = running ? "" : "none";
  btnPrimary.textContent = running ? "暂停" : s.status === "paused" ? "继续" : "开始";
  // 切换键随阶段动态：专注态->去休息，休息态->去专注（纯文字，与「重置」同风格）
  const toBreak = s.phase === "work";
  switchLabel.textContent = toBreak ? "休息" : "专注";

  // 循环指示器：数据源=Rust snapshot（completed_work_count % long_break_every），
  // 前端不自计数；长休阶段全亮；间隔改了圆点数跟着变
  const every = Math.max(1, s.long_break_every);
  if (cycleDots.childElementCount !== every) {
    cycleDots.innerHTML = "";
    for (let i = 0; i < every; i++) {
      const d = document.createElement("span");
      d.className = "cycle-dot";
      cycleDots.append(d);
    }
  }
  const lit = s.phase === "long_break" ? every : s.completed_work_count % every;
  // v8：进行中第 N 个圆点中间态（领导拍板）——工作阶段的当前个 = completed % every，
  // 休息阶段无当前个；长休全亮不变。索引全部由 Rust snapshot 推导，前端不自计数
  const current = s.phase === "work" ? s.completed_work_count % every : -1;
  cycleDots.querySelectorAll(".cycle-dot").forEach((d, i) => {
    d.classList.toggle("lit", i < lit);
    d.classList.toggle("current", i === current);
  });
}

/* ---------- 运行控制 ---------- */

async function toggleRun() {
  if (!snapshot) return;
  if (snapshot.status === "running") await bridge.invoke("timer_pause");
  else if (snapshot.status === "paused") await bridge.invoke("timer_resume");
  else {
    if (!notifiedPermission) {
      notifiedPermission = true;
      await bridge.requestNotifyPermission();
    }
    await bridge.invoke("timer_start");
  }
  render((await bridge.invoke("timer_snapshot")) as TimerSnapshot);
}

btnToggleRun.addEventListener("click", (e) => { e.stopPropagation(); toggleRun(); });
btnPrimary.addEventListener("click", () => toggleRun());
$("btn-reset").addEventListener("click", async () => {
  await bridge.invoke("timer_reset");
  render((await bridge.invoke("timer_snapshot")) as TimerSnapshot);
});
btnSwitch.addEventListener("click", async (e) => {
  e.stopPropagation();
  await bridge.invoke("timer_skip");
  render((await bridge.invoke("timer_snapshot")) as TimerSnapshot);
  refreshStats();
});

/* ---------- 展开 / 收起 + 自定义拖拽（宽容点击：松手位移<10px 且 <500ms 一律算点击） ---------- */

let downPos: { x: number; y: number } | null = null;
let downTime = 0;
let dragging = false;
// 自动化验证探针
const dbg = { moves: 0, dragCalls: 0 };
(window as any).__dbg = dbg;
const pill = $("pill");

async function startNativeDrag(e: MouseEvent) {
  if (dragging || bridge.isMock) return;
  dragging = true;
  dbg.dragCalls++;
  try {
    await bridge.invoke("drag_begin", { grabX: e.clientX, grabY: e.clientY });
  } catch { dragging = false; }
}

async function endNativeDrag() {
  if (!dragging) return;
  dragging = false;
  try { await bridge.invoke("drag_end"); } catch {}
}

/// 绑定阈值拖拽：位移 >12px 启动拖拽窗口；松手时位移 <10px 且按下 <500ms 一律算点击
/// （仅 pill 触发展开，即便中途触发过拖拽——接受「微拖想挪窗」误展开的取舍）
function attachThresholdDrag(el: HTMLElement) {
  el.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return; // 右键留给上下文菜单，不触发拖拽
    if ((e.target as HTMLElement).closest("button")) return;
    dragging = false; // 上一次 mouseup 若丢在窗外，按下即重置陈旧拖拽态
    downPos = { x: e.screenX, y: e.screenY };
    downTime = performance.now();
  });
  el.addEventListener("mousemove", (e) => {
    if (!downPos || dragging) return;
    dbg.moves++;
    if (Math.hypot(e.screenX - downPos.x, e.screenY - downPos.y) > 12) {
      startNativeDrag(e);
    }
  });
  el.addEventListener("mouseup", (e) => {
    endNativeDrag();
    if (!downPos) return;
    if ((e.target as HTMLElement).closest("button")) { downPos = null; return; }
    const moved = Math.hypot(e.screenX - downPos.x, e.screenY - downPos.y);
    const quick = performance.now() - downTime < 500;
    downPos = null;
    if (moved < 10 && quick && el.id === "pill") setExpanded(true);
  });
  // 拖拽进行中光标可跑出窗口（线程跟随有 8ms 延迟）：mouseleave 只清点击判定，
  // 绝不能 endNativeDrag——拖拽线程由物理左键松开/显式 mouseup 终止
  el.addEventListener("mouseleave", () => { downPos = null; });
}

attachThresholdDrag(pill);
attachThresholdDrag($("panel-head"));

$("btn-collapse").addEventListener("click", () => setExpanded(false));
$("btn-drawer-back").addEventListener("click", (e) => { e.stopPropagation(); app.classList.remove("drawer-open"); });

/* 瞬态面板整收：关右键菜单+复位抽屉+收起面板，再展开不留半开态。
   失焦（Rust window-blurred）与 Esc 共用此链路，动画与底部收起键一致。 */
function collapsePanel() {
  hideCtxMenu();
  app.classList.remove("drawer-open");
  if (expanded) setExpanded(false); // mini 态忽略
}

/* ---------- 右键菜单：设置唯一入口（面板内不再保留设置图标） ---------- */

const ctxMenu = $("ctx-menu");

function hideCtxMenu() {
  ctxMenu.classList.remove("open");
  ctxMenu.setAttribute("aria-hidden", "true");
}

app.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  // v11：mini 胶囊右键取消——菜单(89px)比迷你窗(76px)高，物理裁切无解；
  // 菜单入口 = 展开态右键 + 托盘自绘菜单
  if (!expanded) return;
  // 先展开量尺寸再定位（同一帧内完成，无闪烁），钳制在窗口内防出屏
  ctxMenu.classList.add("open");
  const mw = ctxMenu.offsetWidth, mh = ctxMenu.offsetHeight;
  const x = Math.max(4, Math.min(e.clientX, window.innerWidth - mw - 4));
  const y = Math.max(4, Math.min(e.clientY, window.innerHeight - mh - 4));
  ctxMenu.style.left = `${x}px`;
  ctxMenu.style.top = `${y}px`;
  ctxMenu.style.transformOrigin = `${e.clientX - x + 8}px ${e.clientY - y + 8}px`;
  ctxMenu.setAttribute("aria-hidden", "false");
  refreshAutostartItem(); // 勾选态跟随真实注册状态（每次开菜单现查）
});

/* 设置入口：右键菜单与托盘「设置…」共用（展开面板+滑入抽屉）。
   08-16 v15c：内容超高由 .drawer .settings 滚动承载（业界标准，窗口恒 467）；
   每次打开滚动复位到顶，防上次浏览位置残留 */
async function openSettings() {
  if (!expanded) await setExpanded(true);
  app.classList.add("drawer-open");
  (document.querySelector(".drawer .settings") as HTMLElement).scrollTop = 0;
}

$("ctx-settings").addEventListener("click", async (e) => {
  e.stopPropagation();
  hideCtxMenu();
  await openSettings();
});
bridge.onOpenSettings(() => { openSettings(); });

/* 开机自启：勾选态 = 注册表真实状态（get/set 都回读真相，失败路径 Rust 侧记日志） */
const ctxAutostart = $("ctx-autostart");
async function refreshAutostartItem() {
  const on = (await bridge.invoke("get_autostart")) as boolean;
  ctxAutostart.classList.toggle("checked", on);
  ctxAutostart.setAttribute("aria-checked", String(on));
}
ctxAutostart.addEventListener("click", async (e) => {
  e.stopPropagation();
  const cur = ctxAutostart.getAttribute("aria-checked") === "true";
  const now = (await bridge.invoke("set_autostart", { enabled: !cur })) as boolean;
  ctxAutostart.classList.toggle("checked", now);
  ctxAutostart.setAttribute("aria-checked", String(now));
  hideCtxMenu();
});

$("ctx-quit").addEventListener("click", () => {
  bridge.invoke("quit_app"); // 真退进程（Rust app.exit(0)）
});

document.addEventListener("pointerdown", (e) => {
  if (!ctxMenu.contains(e.target as Node)) hideCtxMenu();
}, true);
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") collapsePanel(); // Esc 整收（抽屉/菜单开着也一并收）
});
bridge.onBlur(collapsePanel); // 失焦即收（苹果瞬态面板语义）

/* v11 托盘自绘菜单联动：应答托盘页的主题/材质查询；托盘菜单关闭时若面板展开且未聚焦→整收 */
bridge.onEvent("query-ui-style", () => {
  bridge.emit("ui-style", { theme: document.documentElement.dataset.theme, material: document.documentElement.dataset.material });
});
bridge.onEvent("tray-menu-closed", () => {
  if (expanded && !document.hasFocus()) collapsePanel();
});

/* 灵动岛式 morph：窗口瞬时 set_size（透明窗口下不可见），卡片在窗口内做
   就地 morph——WAAPI 驱动 width/height/border-radius/translate，340ms 弹簧，
   内容固定自然尺寸 + 交叉淡入淡出（旧内容快淡出、新内容延迟淡入），可打断。
   展开：窗口先到位（Rust 防出屏钳制），卡片从旧位置连续生长；
   收起：窗口保持大，卡片先收缩到位，再 morph_commit 落窗（不可见瞬切）。 */
const RADIUS_PANEL = 22, RADIUS_MINI = 38; // 76/2，视觉等同 999px 胶囊
const MORPH_MS = 340;
const MORPH_EASE = "cubic-bezier(0.32, 0.72, 0, 1)";

let morphGen = 0;
let morphAnim: Animation | null = null;

function readCard() {
  const cs = getComputedStyle(app);
  const h = parseFloat(cs.height);
  return {
    w: cs.width,
    h: cs.height,
    // 迷你态 CSS 半径 999px，钳到 h/2 避免插值经过巨大半径（圆球伪影）
    r: `${Math.min(parseFloat(cs.borderTopLeftRadius), h / 2)}px`,
    t: cs.transform === "none" ? "translate(0px, 0px)" : cs.transform,
  };
}

function settleCard(w: number, h: number) {
  app.style.width = `${w}px`;
  app.style.height = `${h}px`;
  app.style.transform = "";
  morphAnim?.cancel();
  morphAnim = null;
}

async function setExpanded(v: boolean) {
  if (expanded === v) return;
  expanded = v;
  const gen = ++morphGen;
  const wasAnimating = morphAnim !== null;
  const from = readCard();
  morphAnim?.cancel();
  morphAnim = null;
  const to = v
    ? { w: PANEL_W, h: PANEL_H, r: RADIUS_PANEL }
    : { w: MINI_W, h: MINI_H, r: RADIUS_MINI };

  if (v) {
    const [dx, dy] = await bridge.morphBegin(PANEL_W, PANEL_H);
    if (gen !== morphGen) return;
    app.classList.toggle("expanded", true);
    app.classList.toggle("mini", false);
    // 从静止起步：卡片锚在旧屏幕位置（窗口可能被钳制移动了 dx,dy）；
    // 打断反转：从当前帧的 transform 连续接管
    const fromT = wasAnimating ? from.t : `translate(${dx}px, ${dy}px)`;
    morphAnim = app.animate(
      [
        { width: from.w, height: from.h, borderRadius: from.r, transform: fromT },
        { width: `${to.w}px`, height: `${to.h}px`, borderRadius: `${to.r}px`, transform: "translate(0px, 0px)" },
      ],
      { duration: MORPH_MS, easing: MORPH_EASE, fill: "forwards" },
    );
    try { await morphAnim.finished; } catch { return; }
    if (gen !== morphGen) return;
    settleCard(to.w, to.h);
    refreshStats();
  } else {
    app.classList.toggle("expanded", false);
    app.classList.toggle("mini", true);
    morphAnim = app.animate(
      [
        { width: from.w, height: from.h, borderRadius: from.r, transform: from.t },
        { width: `${to.w}px`, height: `${to.h}px`, borderRadius: `${to.r}px`, transform: "translate(0px, 0px)" },
      ],
      { duration: MORPH_MS, easing: MORPH_EASE, fill: "forwards" },
    );
    try { await morphAnim.finished; } catch { return; }
    if (gen !== morphGen) return;
    await bridge.morphCommit(MINI_W, MINI_H);
    if (gen !== morphGen) return;
    settleCard(to.w, to.h);
  }
}

/* ---------- 统计 ---------- */

const WEEKDAY = ["一", "二", "三", "四", "五", "六", "日"];

/* 单位（个数/分钟）与范围（周/月）切换：同时作用于周柱与月格，本地 UI 态不落盘 */
type StatsUnit = "count" | "minutes";
type StatsRange = "week" | "month";
let statsUnit: StatsUnit = "count";
let statsRange: StatsRange = "week";
// 月历游标（‹ › 翻历史月份），初始当月；不许翻到未来月
const now0 = new Date();
let monthCursor = { y: now0.getFullYear(), m: now0.getMonth() + 1 };

const statVal = (b: { count: number; minutes: number }) =>
  statsUnit === "count" ? b.count : b.minutes;
const statUnitLabel = () => (statsUnit === "count" ? "个番茄" : "分钟");
/* 08-16：每日目标换算到当前统计口径——个数直取；分钟=个数×工作时长（snapshot 下发，不自猜输入框） */
const goalStatVal = () => {
  const g = snapshot?.daily_goal ?? 8;
  return statsUnit === "count" ? g : Math.round((g * (snapshot?.work_ms ?? 25 * 60_000)) / 60000);
};
/* 图表 Y 上限恒留 25% 顶部空间：目标线/最高柱最高到 80%，永不贴顶。
   （无轴设计下 nice number 无观察者，恒定 headroom 一行覆盖全部边界——含 goal 恰为整档的 case） */
const chartCeil = (v: number) => Math.ceil(Math.max(1, v) * 1.25);

async function refreshStats() {
  const sum = (await bridge.invoke("stats_summary")) as StatsSummary;
  $("stat-today-count").textContent = String(sum.today_count);
  $("stat-today-min").textContent = String(sum.today_minutes);
  $("stat-streak").textContent = String(sum.streak_days);
  if (statsRange !== "week") return; // 月模式只刷新顶部三卡
  $("chart-title").textContent = "本周";

  const chart = $("week-chart");
  chart.innerHTML = "";
  // 08-16：刻度锚定每日目标（Apple Books 手法）——上限 = niceCeil(max(数据最大, 目标))，
  // 数据稀疏时唯一有数据的天不再满格、目标线必然低于顶端；虚线横贯（bottom 与基线 26px 同参照系）
  const goalVal = goalStatVal();
  const chartMax = chartCeil(Math.max(goalVal, ...sum.week.map(statVal)));
  const goalLine = document.createElement("div");
  goalLine.className = "goal-line";
  goalLine.style.bottom = `${26 + (goalVal / chartMax) * 40}px`;
  goalLine.title = `每日目标：${goalVal} ${statUnitLabel()}`;
  chart.append(goalLine);
  const todayStr = new Date().toLocaleDateString("sv-SE");
  sum.week.forEach((b, i) => {
    const v = statVal(b);
    const col = document.createElement("div");
    col.className = "week-col" + (b.date === todayStr ? " today" : "");
    const track = document.createElement("div");
    // v8⑥⑦：无数据日=基线短划（empty）、有数据日=去灰底纯柱（has-data）
    track.className = "week-track " + (v > 0 ? "has-data" : "empty");
    const bar = document.createElement("div");
    bar.className = "week-bar";
    if (v > 0) {
      bar.style.height = `${Math.max(4, (v / chartMax) * 40)}px`;
      bar.title = `${b.date}：${v} ${statUnitLabel()}`;
    }
    track.append(bar);
    const day = document.createElement("div");
    day.className = "week-day";
    day.textContent = WEEKDAY[i];
    col.append(track, day);
    chart.append(col);
  });
}

/* 月历热力格：当月每日一格（周一为首列，first_weekday 缩进），深浅=强度四档 */
async function refreshMonth() {
  const ms = (await bridge.invoke("stats_month", {
    year: monthCursor.y,
    month: monthCursor.m,
  })) as MonthSummary;
  $("chart-title").textContent = `${ms.year}年${ms.month}月`;

  const grid = $("month-chart");
  grid.innerHTML = "";
  // 08-16：月格分档同样锚定目标（无轴无虚线，深浅=值/niceCeil(max(目标,当月最大)) 四档）
  const chartMax = chartCeil(Math.max(goalStatVal(), ...ms.days.map(statVal)));
  const todayStr = new Date().toLocaleDateString("sv-SE");
  for (let i = 0; i < ms.first_weekday; i++) {
    const pad = document.createElement("span");
    pad.className = "month-pad";
    grid.append(pad);
  }
  ms.days.forEach((b) => {
    const v = statVal(b);
    const cell = document.createElement("div");
    cell.className = "month-cell" + (b.date === todayStr ? " today" : "");
    cell.dataset.level = String(v === 0 ? 0 : Math.min(4, Math.ceil((v / chartMax) * 4)));
    cell.title = `${b.date}：${v} ${statUnitLabel()}`;
    grid.append(cell);
  });

  // 翻页边界：› 到当月即止（未来月无数据可翻）
  const cur = new Date();
  const isCurrent =
    monthCursor.y === cur.getFullYear() && monthCursor.m === cur.getMonth() + 1;
  ($("month-next") as HTMLButtonElement).disabled = isCurrent;
}

function setStatsRange(r: StatsRange) {
  statsRange = r;
  $("range-week").classList.toggle("active", r === "week");
  $("range-month").classList.toggle("active", r === "month");
  $("week-chart").hidden = r !== "week";
  $("month-chart").hidden = r !== "month";
  $("month-prev").hidden = r !== "month";
  $("month-next").hidden = r !== "month";
  if (r === "week") refreshStats(); else refreshMonth();
}

function setStatsUnit(u: StatsUnit) {
  statsUnit = u;
  $("unit-count").classList.toggle("active", u === "count");
  $("unit-min").classList.toggle("active", u === "minutes");
  if (statsRange === "week") refreshStats(); else refreshMonth();
}

$("range-week").addEventListener("click", () => setStatsRange("week"));
$("range-month").addEventListener("click", () => setStatsRange("month"));
$("unit-count").addEventListener("click", () => setStatsUnit("count"));
$("unit-min").addEventListener("click", () => setStatsUnit("minutes"));
$("month-prev").addEventListener("click", () => {
  monthCursor.m -= 1;
  if (monthCursor.m < 1) { monthCursor.m = 12; monthCursor.y -= 1; }
  refreshMonth();
});
$("month-next").addEventListener("click", () => {
  monthCursor.m += 1;
  if (monthCursor.m > 12) { monthCursor.m = 1; monthCursor.y += 1; }
  refreshMonth();
});

/* ---------- 设置 ---------- */

const toggleAutoStart = $("toggle-auto-start");

async function loadConfig() {
  const cfg = (await bridge.invoke("get_config")) as TimerConfig;
  ($("input-work") as HTMLInputElement).value = String(Math.round(cfg.work_ms / 60000));
  ($("input-short") as HTMLInputElement).value = String(Math.round(cfg.short_break_ms / 60000));
  ($("input-long") as HTMLInputElement).value = String(Math.round(cfg.long_break_ms / 60000));
  ($("input-every") as HTMLInputElement).value = String(cfg.long_break_every);
  ($("input-goal") as HTMLInputElement).value = String(cfg.daily_goal);
  toggleAutoStart.setAttribute("aria-checked", String(cfg.auto_start_next));
  toggleAutoStart.classList.toggle("on", cfg.auto_start_next);
  applyMaterial(cfg.material);
  // v16：自定义配色（null=默认），启动即行内覆盖
  customColors = {
    work: cfg.colors?.work ?? null,
    short_break: cfg.colors?.short_break ?? null,
    long_break: cfg.colors?.long_break ?? null,
  };
  applyColors();
  syncColorUI();
}

/* 材质（经典/液态玻璃）：切换即生效并持久化到 config.json */
function applyMaterial(material: string) {
  document.documentElement.dataset.material = material;
  document.querySelectorAll(".material-opt").forEach((b) =>
    b.classList.toggle("active", (b as HTMLElement).dataset.materialOpt === material)
  );
}

/* ---------- v16 三阶段自定义配色：行内覆盖 CSS 变量，null=跟随主题默认 ---------- */
type ColorKey = "work" | "short_break" | "long_break";
const COLOR_VAR: Record<ColorKey, string> = { work: "--work", short_break: "--break", long_break: "--long" };
let customColors: Record<ColorKey, string | null> = { work: null, short_break: null, long_break: null };
let colorErrTimer = 0;

function applyColors() {
  const root = document.documentElement;
  const dark = root.dataset.theme === "dark";
  (Object.keys(COLOR_VAR) as ColorKey[]).forEach((k) => {
    const c = customColors[k];
    if (c) {
      root.style.setProperty(COLOR_VAR[k], c);
      // 浅色文字需深一号（v8③ 既有纪律）：82% 混黑派生；深色同 fill
      if (k === "work") root.style.setProperty("--work-text", dark ? c : `color-mix(in srgb, ${c} 82%, black)`);
    } else {
      root.style.removeProperty(COLOR_VAR[k]);
      if (k === "work") root.style.removeProperty("--work-text");
    }
  });
}

/* UI 对表：色块（button 背景）/色号框显示生效色（默认态取 computed 真值——深色主题默认色与浅色不同） */
function syncColorUI() {
  const root = document.documentElement;
  document.querySelectorAll<HTMLElement>("[data-color]").forEach((el) => {
    const k = el.dataset.color as ColorKey;
    const shown =
      customColors[k] ?? getComputedStyle(root).getPropertyValue(COLOR_VAR[k]).trim();
    if (el instanceof HTMLInputElement) el.value = shown;
    else el.style.background = shown;
  });
  document.querySelectorAll<HTMLButtonElement>(".color-reset").forEach((b) => {
    b.classList.toggle("default", customColors[b.dataset.color as ColorKey] === null);
  });
}

function persistColors() {
  bridge.invoke("set_colors", { colors: { ...customColors } });
}

/* ---------- v16b 自绘 iOS 风色盘浮层（方块+彩虹条；替换丑的 Windows 系统色盘） ---------- */
function hsvToHex(h: number, s: number, v: number): string {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    const c = v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
    return Math.round(c * 255).toString(16).padStart(2, "0");
  };
  return `#${f(5)}${f(3)}${f(1)}`;
}
function hexToHsv(hex: string): { h: number; s: number; v: number } {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b), d = max - min;
  let h = 0;
  if (d > 0) {
    if (max === r) h = 60 * (((g - b) / d) % 6);
    else if (max === g) h = 60 * ((b - r) / d + 2);
    else h = 60 * ((r - g) / d + 4);
  }
  if (h < 0) h += 360;
  return { h, s: max === 0 ? 0 : d / max, v: max };
}

const colorPop = document.querySelector<HTMLElement>(".color-pop")!;
const cpSv = colorPop.querySelector<HTMLElement>(".cp-sv")!;
const cpDot = colorPop.querySelector<HTMLElement>(".cp-dot")!;
const cpHue = colorPop.querySelector<HTMLElement>(".cp-hue")!;
const cpHueDot = colorPop.querySelector<HTMLElement>(".cp-hue-dot")!;
const cpHex = colorPop.querySelector<HTMLInputElement>(".cp-hex")!;
let cpKey: ColorKey | null = null;
let cpH = 0, cpS = 1, cpV = 1;
/* v16c：拖拽中标志——悬停跟手处理器据此让位（拖拽由 window 级监听全权驱动） */
let cpDragging = false;

const currentCpHex = () => hsvToHex(cpH, cpS, cpV);

function renderColorPop() {
  colorPop.style.setProperty("--cp-h", String(Math.round(cpH)));
  colorPop.style.setProperty("--cp-color", currentCpHex());
  cpDot.style.left = `${cpS * 100}%`;
  cpDot.style.top = `${(1 - cpV) * 100}%`;
  cpHueDot.style.left = `${(cpH / 360) * 100}%`;
  cpHex.value = currentCpHex();
}

/* 拖动即时生效：全 UI 同步换色（直控取色器的核心手感） */
function applyFromPicker() {
  if (!cpKey) return;
  customColors[cpKey] = currentCpHex();
  applyColors();
  syncColorUI();
  renderColorPop();
}

function openColorPop(k: ColorKey) {
  cpKey = k;
  const hex =
    customColors[k] ?? getComputedStyle(document.documentElement).getPropertyValue(COLOR_VAR[k]).trim();
  ({ h: cpH, s: cpS, v: cpV } = hexToHsv(hex));
  colorPop.hidden = false;
  renderColorPop();
}

function closeColorPop() {
  if (!cpKey) return;
  cpKey = null;
  colorPop.hidden = true;
  persistColors(); // 关闭时兜底落盘（拖完/hex 提交时已各落一次，幂等）
}

/* SV 方块拖拽：x=饱和度、y=1-明度；按下即选色（iOS 手感），拖完落盘一次。
   move/up 挂 window 级——不依赖 setPointerCapture（WebView2 对真实指针可能抛 InvalidPointerId
   炸掉整个 handler，08-16 用户实测拖不动），指针跑出方块/彩虹条也持续跟踪 */
cpSv.addEventListener("pointerdown", (e) => {
  if (!cpKey) return;
  e.preventDefault();
  cpDragging = true;
  try { cpSv.setPointerCapture(e.pointerId); } catch {}
  const move = (ev: PointerEvent) => {
    const r = cpSv.getBoundingClientRect();
    cpS = Math.max(0, Math.min(1, (ev.clientX - r.left) / r.width));
    cpV = 1 - Math.max(0, Math.min(1, (ev.clientY - r.top) / r.height));
    applyFromPicker();
  };
  const up = () => {
    cpDragging = false;
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("pointercancel", up);
    persistColors();
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  window.addEventListener("pointercancel", up);
  move(e);
});

/* 彩虹条拖拽：x=色相 0-360°，SV 方块底色随 --cp-h 即时换（同 window 级监听） */
cpHue.addEventListener("pointerdown", (e) => {
  if (!cpKey) return;
  e.preventDefault();
  cpDragging = true;
  try { cpHue.setPointerCapture(e.pointerId); } catch {}
  const move = (ev: PointerEvent) => {
    const r = cpHue.getBoundingClientRect();
    cpH = Math.max(0, Math.min(1, (ev.clientX - r.left) / r.width)) * 360;
    applyFromPicker();
  };
  const up = () => {
    cpDragging = false;
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("pointercancel", up);
    persistColors();
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  window.addEventListener("pointercancel", up);
  move(e);
});

/* v16c 悬停跟手：cursor:none 后圆环=指针。悬停时圆环贴光标并预览其下颜色（不提交任何状态），
   离开控件回弹到已选位置/已选色（renderColorPop 重算） */
cpSv.addEventListener("pointermove", (e) => {
  if (!cpKey || cpDragging) return;
  const r = cpSv.getBoundingClientRect();
  const hs = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
  const hv = 1 - Math.max(0, Math.min(1, (e.clientY - r.top) / r.height));
  cpDot.style.left = `${hs * 100}%`;
  cpDot.style.top = `${(1 - hv) * 100}%`;
  colorPop.style.setProperty("--cp-color", hsvToHex(cpH, hs, hv));
});
cpSv.addEventListener("pointerleave", () => { if (cpKey && !cpDragging) renderColorPop(); });

cpHue.addEventListener("pointermove", (e) => {
  if (!cpKey || cpDragging) return;
  const r = cpHue.getBoundingClientRect();
  const hh = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * 360;
  cpHueDot.style.left = `${(hh / 360) * 100}%`;
  colorPop.style.setProperty("--cp-color", hsvToHex(hh, cpS, cpV));
});
cpHue.addEventListener("pointerleave", () => { if (cpKey && !cpDragging) renderColorPop(); });

/* 浮层内 hex 框：合法→HSV 状态同步（dot/条回位）；非法→闪红回退 */
cpHex.addEventListener("change", () => {
  const v = cpHex.value.trim().toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(v)) {
    ({ h: cpH, s: cpS, v: cpV } = hexToHsv(v));
    applyFromPicker();
    persistColors();
  } else {
    cpHex.classList.add("error");
    window.clearTimeout(colorErrTimer);
    colorErrTimer = window.setTimeout(() => {
      cpHex.classList.remove("error");
      renderColorPop();
    }, 1500);
  }
});

colorPop.querySelector<HTMLButtonElement>(".cp-done")!.addEventListener("click", (e) => {
  e.stopPropagation();
  closeColorPop();
});

/* 色块点击=打开浮层（已开则切换编辑对象，浮层不关） */
document.querySelectorAll<HTMLButtonElement>(".color-swatch").forEach((b) => {
  b.addEventListener("click", (e) => {
    e.stopPropagation();
    openColorPop(b.dataset.color as ColorKey);
  });
});

/* 点外关闭：抽屉内 pointerdown（capture）——浮层自身与色块之外的区域 */
document.querySelector(".drawer")!.addEventListener("pointerdown", (e) => {
  if (!cpKey) return;
  const t = e.target as HTMLElement;
  if (!colorPop.contains(t) && !t.closest(".color-swatch")) closeColorPop();
}, true);

/* Esc 只关浮层不整收面板（capture 拦截，同快捷键录制手法） */
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && cpKey) {
    e.preventDefault();
    e.stopImmediatePropagation();
    closeColorPop();
  }
}, true);

document.querySelectorAll<HTMLInputElement>(".color-hex").forEach((input) => {
  input.addEventListener("change", () => {
    const k = input.dataset.color as ColorKey;
    const v = input.value.trim().toLowerCase();
    if (/^#[0-9a-f]{6}$/.test(v)) {
      input.value = v;
      customColors[k] = v;
      applyColors();
      syncColorUI();
      persistColors();
    } else {
      // 非法色号：保留所输文字 + 闪红提示，1.5s 后回退生效色
      input.classList.add("error");
      window.clearTimeout(colorErrTimer);
      colorErrTimer = window.setTimeout(() => {
        input.classList.remove("error");
        syncColorUI();
      }, 1500);
    }
  });
});

document.querySelectorAll<HTMLButtonElement>(".color-reset").forEach((b) => {
  b.addEventListener("click", (e) => {
    e.stopPropagation();
    customColors[b.dataset.color as ColorKey] = null;
    applyColors();
    syncColorUI();
    persistColors();
  });
});

/* 步进器/输入框的范围表：which → [inputId, min, max] */
const STEP_RANGE: Record<string, [string, number, number]> = {
  work: ["input-work", 1, 120],
  short: ["input-short", 1, 60],
  long: ["input-long", 1, 60],
  every: ["input-every", 2, 10],
  goal: ["input-goal", 1, 20],
};

function readStep(which: string): number {
  const [id, min, max] = STEP_RANGE[which];
  const el = $(id) as HTMLInputElement;
  const v = Math.max(min, Math.min(max, Number(el.value) || min));
  el.value = String(v);
  return v;
}

async function applyConfig() {
  await bridge.invoke("set_config", {
    workMin: readStep("work"),
    shortBreakMin: readStep("short"),
    longBreakMin: readStep("long"),
    longBreakEvery: readStep("every"),
    autoStartNext: toggleAutoStart.getAttribute("aria-checked") === "true",
    dailyGoal: readStep("goal"),
  });
  render((await bridge.invoke("timer_snapshot")) as TimerSnapshot);
  // 08-16：目标/时长变更会改变周柱刻度与目标虚线（分钟口径换算），统计图须跟着重刷
  await refreshStats();
  if (statsRange === "month") await refreshMonth();
}

toggleAutoStart.addEventListener("click", (e) => {
  e.stopPropagation();
  const on = toggleAutoStart.getAttribute("aria-checked") !== "true";
  toggleAutoStart.setAttribute("aria-checked", String(on));
  toggleAutoStart.classList.toggle("on", on);
  applyConfig();
});

/* ---------- v11 快捷键改键：显示当前生效键、按下即录、占用回退保留旧键、可一键关闭 ---------- */
interface HotkeyUIState { enabled: boolean; shortcut: string; effective: string | null; }
let hotkeyState: HotkeyUIState | null = null;
let hotkeyRecording = false;
let hotkeyErrTimer = 0;

function renderHotkey() {
  if (!hotkeyState) return;
  hotkeyCap.classList.remove("recording", "error");
  if (!hotkeyState.enabled) {
    hotkeyCap.textContent = "已关闭";
    hotkeyCap.classList.add("off");
    hotkeyOff.style.display = "none";
  } else {
    hotkeyCap.classList.remove("off");
    hotkeyCap.textContent = hotkeyState.effective ?? hotkeyState.shortcut;
    hotkeyOff.style.display = "";
  }
}

async function refreshHotkey() {
  hotkeyState = (await bridge.invoke("get_hotkey")) as HotkeyUIState;
  renderHotkey();
}

function stopHotkeyRecord() {
  hotkeyRecording = false;
  document.removeEventListener("keydown", onHotkeyRecordKey, true);
  document.removeEventListener("pointerdown", onHotkeyRecordAway, true);
}
function cancelHotkeyRecord() { stopHotkeyRecord(); renderHotkey(); }

function startHotkeyRecord() {
  if (hotkeyRecording) return;
  hotkeyRecording = true;
  hotkeyCap.classList.remove("off", "error");
  hotkeyCap.classList.add("recording");
  hotkeyCap.textContent = "按修饰键+主键…";
  document.addEventListener("keydown", onHotkeyRecordKey, true);
  document.addEventListener("pointerdown", onHotkeyRecordAway, true);
}
function onHotkeyRecordAway(e: PointerEvent) {
  if (!hotkeyCap.contains(e.target as Node)) cancelHotkeyRecord();
}
async function onHotkeyRecordKey(e: KeyboardEvent) {
  // capture 阶段拦下：preventDefault+stopImmediatePropagation，Esc 不触发面板整收
  e.preventDefault();
  e.stopImmediatePropagation();
  if (e.key === "Escape") { cancelHotkeyRecord(); return; }
  if (e.key === "Backspace" || e.key === "Delete") { stopHotkeyRecord(); await applyHotkey(false); return; }
  if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return; // 纯修饰键：继续等主键
  const mods = (e.ctrlKey ? "Ctrl+" : "") + (e.altKey ? "Alt+" : "") + (e.shiftKey ? "Shift+" : "") + (e.metaKey ? "Super+" : "");
  if (!mods) {
    hotkeyCap.textContent = "需要修饰键…";
    window.setTimeout(() => { if (hotkeyRecording) hotkeyCap.textContent = "按修饰键+主键…"; }, 900);
    return;
  }
  const key = e.key.length === 1 ? e.key.toUpperCase() : e.key; // Space/ArrowUp/F5 等保留原名
  stopHotkeyRecord();
  await applyHotkey(true, mods + key);
}
async function applyHotkey(enabled: boolean, shortcut?: string) {
  try {
    hotkeyState = (await bridge.invoke("set_hotkey", shortcut ? { enabled, shortcut } : { enabled })) as HotkeyUIState;
    renderHotkey();
  } catch (err) {
    // 占用/不可识别：提示并保留旧键（Rust 侧未动配置未动注册）
    hotkeyCap.classList.remove("recording");
    hotkeyCap.classList.add("error");
    hotkeyCap.textContent = String(err).includes("修饰键") || String(err).includes("识别") ? "不支持的按键" : "被占用，已保留原键";
    window.clearTimeout(hotkeyErrTimer);
    hotkeyErrTimer = window.setTimeout(renderHotkey, 2200);
  }
}
hotkeyCap.addEventListener("click", (e) => { e.stopPropagation(); if (hotkeyRecording) cancelHotkeyRecord(); else startHotkeyRecord(); });
hotkeyOff.addEventListener("click", async (e) => { e.stopPropagation(); await applyHotkey(false); });

$("input-work").addEventListener("change", applyConfig);
$("input-short").addEventListener("change", applyConfig);
$("input-long").addEventListener("change", applyConfig);
$("input-every").addEventListener("change", applyConfig);
$("input-goal").addEventListener("change", applyConfig);

/* 步进器 */
document.querySelectorAll<HTMLButtonElement>(".step-btn").forEach((btn) => {
  btn.addEventListener("click", () => {
    const [which, dir] = btn.dataset.step!.split(":");
    const [id, min, max] = STEP_RANGE[which];
    const input = $(id) as HTMLInputElement;
    input.value = String(Math.max(min, Math.min(max, Number(input.value) + Number(dir))));
    applyConfig();
  });
});

/* ---------- 主题 ---------- */

document.querySelectorAll<HTMLButtonElement>(".theme-opt").forEach((btn) => {
  btn.addEventListener("click", () => {
    const theme = btn.dataset.themeOpt!;
    document.documentElement.dataset.theme = theme;
    // v16：work-text 派生随主题变（浅=82% 混黑 / 深=原色），换肤即重算
    applyColors();
    syncColorUI();
    document.querySelectorAll(".theme-opt").forEach((b) => b.classList.toggle("active", b === btn));
    // v11：托盘自绘菜单页跟随换肤（广播给所有窗口，托盘页 load/打开时也会 query 兜底）
    bridge.emit("ui-style", { theme: document.documentElement.dataset.theme, material: document.documentElement.dataset.material });
  });
});

document.querySelectorAll<HTMLButtonElement>(".material-opt").forEach((btn) => {
  btn.addEventListener("click", async () => {
    const material = btn.dataset.materialOpt!;
    applyMaterial(material);
    // v11：托盘自绘菜单页跟随换肤（同 theme-opt 链路）
    bridge.emit("ui-style", { theme: document.documentElement.dataset.theme, material: document.documentElement.dataset.material });
    await bridge.invoke("set_material", { material });
  });
});

/* ---------- 事件订阅 ---------- */

bridge.onTick(render);
bridge.onPhaseCompleted(() => {
  playChime();
  refreshStats();
});

/* 一声轻提示音：WebAudio 合成柔和高音，无需音频资源 */
function playChime() {
  try {
    const ctx = new AudioContext();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = "sine";
    osc.frequency.value = 880;
    gain.gain.setValueAtTime(0.0001, ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.12, ctx.currentTime + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.9);
    osc.connect(gain).connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 1);
    osc.onended = () => ctx.close();
  } catch { /* 无声可接受 */ }
}

/* ---------- 初始化 ---------- */

(async () => {
  await loadConfig();
  await refreshHotkey();
  render((await bridge.invoke("timer_snapshot")) as TimerSnapshot);
  await refreshStats();
})();
