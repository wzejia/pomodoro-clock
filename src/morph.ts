import { bridge } from "./bridge";
import { springValue } from "./spring";

const MINI_W = 220, MINI_H = 76;
export const PANEL_W = 340, PANEL_H = 467;
const RADIUS_PANEL = 22, RADIUS_MINI = 38;
/** 感知时长（秒）。尾巴继续到位移和速度都低于阈值，不在整数秒硬切。 */
const EXPAND_S = 0.42, COLLAPSE_S = 0.32;
/** 亚像素位移，以及约 0.2px/帧（60Hz）以下的速度，才算视觉落定。 */
const SETTLE_POS = 0.5, SETTLE_VEL = 12;

const app = document.getElementById("app") as HTMLElement;
const pillTime = document.getElementById("pill-time") as HTMLElement;
const panelTime = document.getElementById("panel-time") as HTMLElement;

let expanded = false;
let morphGen = 0;
let onExpandSettled: () => void = () => {};
let onVisualChange: () => void = () => {};

interface Chan { y0: number; v0: number; target: number }
interface Glyph { dx: number; dy: number; scale: number; ref: number }
interface Pose {
  w: number; h: number; r: number;
  x: number; y: number;
  nx: number; ny: number; ns: number;
  vw: number; vh: number; vr: number;
  vx: number; vy: number;
  vnx: number; vny: number; vns: number;
}
interface Live {
  gen: number;
  t0: number;
  duration: number;
  expand: boolean;
  ref: number;
  w: Chan; h: Chan; r: Chan;
  x: Chan; y: Chan;
  nx: Chan; ny: Chan; ns: Chan;
}

let live: Live | null = null;
let raf = 0;
let settleWait: ((completed: boolean) => void) | null = null;
let cachedGlyph: Glyph | null = null;
/** morphBegin / morphCommit 串行，避免连点时后发的 begin 盖住已经该收小的窗口。 */
let geomChain: Promise<void> = Promise.resolve();

export function isExpanded() {
  return expanded;
}

export function bindMorph(opts: { onExpandSettled: () => void; onVisualChange: () => void }) {
  onExpandSettled = opts.onExpandSettled;
  onVisualChange = opts.onVisualChange;
}

function reducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function enqueueGeom<T>(op: () => Promise<T>): Promise<T> {
  const run = geomChain.then(op, op);
  geomChain = run.then(() => undefined, () => undefined);
  return run;
}

function chan(value: number, velocity: number, target: number): Chan {
  return { y0: value - target, v0: velocity, target };
}

function px(raw: string, fallback: number) {
  const n = parseFloat(raw);
  return Number.isFinite(n) ? n : fallback;
}

function zeroPose(w: number, h: number, r: number, x: number, y: number, nx: number, ny: number, ns: number): Pose {
  return {
    w, h, r, x, y, nx, ny, ns,
    vw: 0, vh: 0, vr: 0, vx: 0, vy: 0, vnx: 0, vny: 0, vns: 0,
  };
}

function readBox(): Pose {
  const cs = getComputedStyle(app);
  const w = px(cs.width, MINI_W);
  const h = px(cs.height, MINI_H);
  const r = Math.min(px(cs.borderTopLeftRadius, RADIUS_MINI), h / 2);
  let x = 0, y = 0;
  if (cs.transform && cs.transform !== "none") {
    const m = new DOMMatrix(cs.transform);
    x = m.m41;
    y = m.m42;
  }
  return zeroPose(w, h, r, x, y, 0, 0, 1);
}

/** 胶囊中心相对面板数字自然中心的位移，字号比作 scale。当前行内 transform 先扣掉。 */
function measureGlyph(nx: number, ny: number, ns: number): Glyph | null {
  const pr = pillTime.getBoundingClientRect();
  const qr = panelTime.getBoundingClientRect();
  const pillFs = parseFloat(getComputedStyle(pillTime).fontSize);
  const panelFs = parseFloat(getComputedStyle(panelTime).fontSize);
  if (!(pr.width > 0) || !(qr.width > 0) || !(pillFs > 0) || !(panelFs > 0)) return cachedGlyph;
  const s = Math.abs(ns) > 1e-4 ? Math.abs(ns) : 1;
  const g: Glyph = {
    dx: (pr.left + pr.width / 2) - (qr.left + qr.width / 2 - nx),
    dy: (pr.top + pr.height / 2) - (qr.top + qr.height / 2 - ny),
    scale: pillFs / panelFs,
    ref: qr.width / s,
  };
  cachedGlyph = g;
  return g;
}

function sample(ch: Chan, duration: number, t: number) {
  const s = springValue(duration, ch.y0, ch.v0, t);
  return { value: ch.target + s.value, velocity: s.velocity };
}

function poseAt(m: Live, now: number): Pose {
  const t = Math.max(0, (now - m.t0) / 1000);
  const d = m.duration;
  const w = sample(m.w, d, t);
  const h = sample(m.h, d, t);
  const r = sample(m.r, d, t);
  const x = sample(m.x, d, t);
  const y = sample(m.y, d, t);
  const nx = sample(m.nx, d, t);
  const ny = sample(m.ny, d, t);
  const ns = sample(m.ns, d, t);
  return {
    w: w.value, h: h.value, r: r.value, x: x.value, y: y.value,
    nx: nx.value, ny: ny.value, ns: ns.value,
    vw: w.velocity, vh: h.velocity, vr: r.velocity, vx: x.velocity, vy: y.velocity,
    vnx: nx.velocity, vny: ny.velocity, vns: ns.velocity,
  };
}

function paint(p: Pose) {
  app.style.width = `${p.w}px`;
  app.style.height = `${p.h}px`;
  app.style.borderRadius = `${p.r}px`;
  app.style.transform = `translate(${p.x}px, ${p.y}px)`;
  panelTime.style.transform = `translate(${p.nx}px, ${p.ny}px) scale(${p.ns})`;
}

function commitBox(w: number, h: number) {
  app.style.width = `${w}px`;
  app.style.height = `${h}px`;
  app.style.transform = "";
  app.style.borderRadius = "";
  panelTime.style.transform = "";
}

function near(value: number, velocity: number, target: number, toPx: number) {
  if (!Number.isFinite(value) || !Number.isFinite(velocity)) return true;
  return Math.abs(value - target) * toPx < SETTLE_POS && Math.abs(velocity) * toPx < SETTLE_VEL;
}

function settled(p: Pose, m: Live) {
  const ref = m.ref > 1 ? m.ref : 56;
  return near(p.w, p.vw, m.w.target, 1)
    && near(p.h, p.vh, m.h.target, 1)
    && near(p.r, p.vr, m.r.target, 1)
    && near(p.x, p.vx, m.x.target, 1)
    && near(p.y, p.vy, m.y.target, 1)
    && near(p.nx, p.vnx, m.nx.target, 1)
    && near(p.ny, p.vny, m.ny.target, 1)
    && near(p.ns, p.vns, m.ns.target, ref);
}

function finish(gen: number) {
  if (gen !== morphGen || !live || live.gen !== gen) return;
  const w = live.w.target;
  const h = live.h.target;
  const expand = live.expand;
  live = null;
  if (raf) cancelAnimationFrame(raf);
  raf = 0;
  if (!expand) panelTime.style.visibility = "hidden";
  commitBox(w, h);
  const wait = settleWait;
  settleWait = null;
  try {
    if (expand) onExpandSettled();
  } finally {
    wait?.(true);
  }
}

function tick(now: number) {
  const m = live;
  if (!m || m.gen !== morphGen) return;
  const p = poseAt(m, now);
  if (settled(p, m)) {
    finish(m.gen);
    return;
  }
  paint(p);
  raf = requestAnimationFrame(tick);
}

/** 停掉正在跑的弹簧，返回这一刻的位移和速度（闭式解，不是上一帧的差分）。 */
function takeLive(now: number): Pose | null {
  if (!live) return null;
  const p = poseAt(live, now);
  paint(p);
  live = null;
  if (raf) cancelAnimationFrame(raf);
  raf = 0;
  const wait = settleWait;
  settleWait = null;
  wait?.(false);
  return p;
}

function numberChans(expand: boolean, from: Pose, interrupted: boolean, g: Glyph | null) {
  const ref = g && g.ref > 1 ? g.ref : 56;
  if (expand) {
    if (interrupted) {
      return { nx: chan(from.nx, from.vnx, 0), ny: chan(from.ny, from.vny, 0), ns: chan(from.ns, from.vns, 1), ref };
    }
    if (!g) return { nx: chan(0, 0, 0), ny: chan(0, 0, 0), ns: chan(1, 0, 1), ref };
    return { nx: chan(g.dx, 0, 0), ny: chan(g.dy, 0, 0), ns: chan(g.scale, 0, 1), ref };
  }
  if (!g) {
    return { nx: chan(from.nx, from.vnx, 0), ny: chan(from.ny, from.vny, 0), ns: chan(from.ns, from.vns, 1), ref };
  }
  if (interrupted) {
    return { nx: chan(from.nx, from.vnx, g.dx), ny: chan(from.ny, from.vny, g.dy), ns: chan(from.ns, from.vns, g.scale), ref };
  }
  return { nx: chan(0, 0, g.dx), ny: chan(0, 0, g.dy), ns: chan(1, 0, g.scale), ref };
}

function launch(gen: number, expand: boolean, from: Pose, interrupted: boolean, anchorX: number, anchorY: number, g: Glyph | null) {
  const nums = numberChans(expand, from, interrupted, g);
  const x0 = expand && !interrupted ? anchorX : from.x;
  const y0 = expand && !interrupted ? anchorY : from.y;
  const vx = expand && !interrupted ? 0 : from.vx;
  const vy = expand && !interrupted ? 0 : from.vy;
  const now = performance.now();
  live = {
    gen,
    t0: now,
    duration: expand ? EXPAND_S : COLLAPSE_S,
    expand,
    ref: nums.ref,
    w: chan(from.w, from.vw, expand ? PANEL_W : MINI_W),
    h: chan(from.h, from.vh, expand ? PANEL_H : MINI_H),
    r: chan(from.r, from.vr, expand ? RADIUS_PANEL : RADIUS_MINI),
    x: chan(x0, vx, 0),
    y: chan(y0, vy, 0),
    nx: nums.nx,
    ny: nums.ny,
    ns: nums.ns,
  };
  const p = poseAt(live, now);
  if (settled(p, live)) return;
  paint(p);
  raf = requestAnimationFrame(tick);
}

function untilSettled(gen: number): Promise<boolean> {
  if (!live || live.gen !== gen) return Promise.resolve(false);
  if (settled(poseAt(live, live.t0), live)) {
    finish(gen);
    return Promise.resolve(true);
  }
  return new Promise((resolve) => { settleWait = resolve; });
}

function setMode(expand: boolean) {
  app.classList.toggle("expanded", expand);
  app.classList.toggle("mini", !expand);
  if (expand) panelTime.style.visibility = "";
  onVisualChange();
}

async function applyReduced(v: boolean, gen: number) {
  if (v) {
    await enqueueGeom(() => bridge.morphBegin(PANEL_W, PANEL_H));
    if (gen !== morphGen) return;
    setMode(true);
    commitBox(PANEL_W, PANEL_H);
    onExpandSettled();
    return;
  }
  setMode(false);
  commitBox(MINI_W, MINI_H);
  await enqueueGeom(() => bridge.morphCommit(MINI_W, MINI_H));
  if (gen !== morphGen) return;
  commitBox(MINI_W, MINI_H);
}

export async function setExpanded(v: boolean): Promise<void> {
  if (expanded === v) return;
  const gen = ++morphGen;
  const held = takeLive(performance.now());
  expanded = v;

  if (reducedMotion()) {
    await applyReduced(v, gen);
    return;
  }

  const from = held ?? readBox();
  const interrupted = held !== null;
  if (v) {
    const [dx, dy] = await enqueueGeom(() => bridge.morphBegin(PANEL_W, PANEL_H));
    if (gen !== morphGen) return;
    setMode(true);
    launch(gen, true, from, interrupted, dx, dy, measureGlyph(from.nx, from.ny, from.ns));
  } else {
    setMode(false);
    launch(gen, false, from, interrupted, 0, 0, measureGlyph(from.nx, from.ny, from.ns));
  }
  if (!(await untilSettled(gen))) return;
  if (gen !== morphGen) return;
  if (!v) {
    await enqueueGeom(() => bridge.morphCommit(MINI_W, MINI_H));
    if (gen !== morphGen) return;
    commitBox(MINI_W, MINI_H);
  }
}
