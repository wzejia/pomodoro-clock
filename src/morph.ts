import { bridge } from "./bridge";

const MINI_W = 220, MINI_H = 76;
export const PANEL_W = 340, PANEL_H = 467;
const RADIUS_PANEL = 22, RADIUS_MINI = 38;
const MORPH_MS_EXPAND = 340;
const MORPH_MS_COLLAPSE = 280;
const MORPH_EASE = "cubic-bezier(0.37, 0.68, 0.2, 1)";

const app = document.getElementById("app") as HTMLElement;
const pillTime = document.getElementById("pill-time") as HTMLElement;
const panelTime = document.getElementById("panel-time") as HTMLElement;

let expanded = false;
let morphGen = 0;
let morphAnim: Animation | null = null;
let timeFlipAnim: Animation | null = null;
let onExpandSettled: () => void = () => {};
let onVisualChange: () => void = () => {};

export function isExpanded() {
  return expanded;
}

export function bindMorph(opts: { onExpandSettled: () => void; onVisualChange: () => void }) {
  onExpandSettled = opts.onExpandSettled;
  onVisualChange = opts.onVisualChange;
}

function readCard() {
  const cs = getComputedStyle(app);
  const h = parseFloat(cs.height);
  return {
    w: cs.width,
    h: cs.height,
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

function animateTimeFlip(expand: boolean) {
  timeFlipAnim?.cancel();
  timeFlipAnim = null;
  const pr = pillTime.getBoundingClientRect();
  const qr = panelTime.getBoundingClientRect();
  if (!pr.width || !qr.width) return;
  const scale = pr.width / qr.width;
  const dx = (pr.left + pr.width / 2) - (qr.left + qr.width / 2);
  const dy = (pr.top + pr.height / 2) - (qr.top + qr.height / 2);
  const pillTf = `translate(${dx}px, ${dy}px) scale(${scale.toFixed(4)})`;
  timeFlipAnim = panelTime.animate(
    expand
      ? [{ transform: pillTf }, { transform: "none" }]
      : [{ transform: "none" }, { transform: pillTf }],
    { duration: expand ? MORPH_MS_EXPAND : MORPH_MS_COLLAPSE, easing: MORPH_EASE, fill: "none" },
  );
}

export async function setExpanded(v: boolean) {
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
    onVisualChange();
    const fromT = wasAnimating ? from.t : `translate(${dx}px, ${dy}px)`;
    morphAnim = app.animate(
      [
        { width: from.w, height: from.h, borderRadius: from.r, transform: fromT },
        { width: `${to.w}px`, height: `${to.h}px`, borderRadius: `${to.r}px`, transform: "translate(0px, 0px)" },
      ],
      { duration: MORPH_MS_EXPAND, easing: MORPH_EASE, fill: "forwards" },
    );
    animateTimeFlip(true);
    try { await morphAnim.finished; } catch { return; }
    if (gen !== morphGen) return;
    settleCard(to.w, to.h);
    onExpandSettled();
  } else {
    app.classList.toggle("expanded", false);
    app.classList.toggle("mini", true);
    onVisualChange();
    morphAnim = app.animate(
      [
        { width: from.w, height: from.h, borderRadius: from.r, transform: from.t },
        { width: `${to.w}px`, height: `${to.h}px`, borderRadius: `${to.r}px`, transform: "translate(0px, 0px)" },
      ],
      { duration: MORPH_MS_COLLAPSE, easing: MORPH_EASE, fill: "forwards" },
    );
    animateTimeFlip(false);
    try { await morphAnim.finished; } catch { return; }
    if (gen !== morphGen) return;
    await bridge.morphCommit(MINI_W, MINI_H);
    if (gen !== morphGen) return;
    settleCard(to.w, to.h);
  }
}
