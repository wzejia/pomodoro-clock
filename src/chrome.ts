import { bridge } from "./bridge";
import { isExpanded, setExpanded, whenPresented } from "./morph";

const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;

const app = $("app");
const pill = $("pill");
const panel = $("panel");
const panelHead = $("panel-head");
const controls = document.querySelector(".controls") as HTMLElement;
const stats = document.querySelector(".stats") as HTMLElement;
const foot = document.querySelector(".panel-foot") as HTMLElement;
const drawer = $("drawer");
const drawerHead = document.querySelector(".drawer-head") as HTMLElement;
const settingsEl = document.querySelector(".drawer .settings") as HTMLElement;
const colorPop = document.querySelector(".color-pop") as HTMLElement;
const ctxMenu = $("ctx-menu");

let menuOpen = false;
let drawerOpen = false;
let colorOpen = false;
let hotkeyOpen = false;
let onMenuOpened: () => void = () => {};
let onDismiss: () => void = () => {};

const dbg = { moves: 0, dragCalls: 0 };
(window as unknown as { __dbg: typeof dbg }).__dbg = dbg;

function setInert(el: HTMLElement, inert: boolean) {
  if (inert) el.setAttribute("inert", "");
  else el.removeAttribute("inert");
}

export function syncInert() {
  const expanded = isExpanded();
  setInert(pill, expanded);
  setInert(panel, !expanded);
  setInert(drawer, !expanded || !drawerOpen);
  const blockRest = !expanded || drawerOpen || menuOpen;
  setInert(panelHead, blockRest);
  setInert(controls, blockRest);
  setInert(stats, blockRest);
  setInert(foot, blockRest);
  setInert(app, menuOpen);
  setInert(ctxMenu, !menuOpen);
  setInert(settingsEl, !drawerOpen || colorOpen || hotkeyOpen);
  setInert(drawerHead, !drawerOpen || colorOpen || hotkeyOpen);
  setInert(colorPop, !colorOpen);
}

export function hideMenu() {
  menuOpen = false;
  ctxMenu.classList.remove("open");
  ctxMenu.setAttribute("aria-hidden", "true");
  syncInert();
}

export function closeDrawer() {
  drawerOpen = false;
  app.classList.remove("drawer-open");
  syncInert();
}

export async function openSettings() {
  hideMenu();
  if (!isExpanded()) void setExpanded(true);
  await whenPresented();
  if (!isExpanded()) return;
  drawerOpen = true;
  app.classList.add("drawer-open");
  settingsEl.scrollTop = 0;
  syncInert();
}

export function setColorLayer(open: boolean) {
  colorOpen = open;
  syncInert();
}

export function setHotkeyLayer(open: boolean) {
  hotkeyOpen = open;
  syncInert();
}

export function bindChrome(opts: { onMenuOpened: () => void; onDismiss: () => void }) {
  onMenuOpened = opts.onMenuOpened;
  onDismiss = opts.onDismiss;
  pill.tabIndex = 0;
  attachThresholdDrag(pill);
  attachThresholdDrag(panelHead);
  pill.addEventListener("keydown", (e) => {
    if (e.target !== pill) return;
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      setExpanded(true);
    }
  });
  $("btn-collapse").addEventListener("click", () => collapseToMini());
  $("btn-settings").addEventListener("click", (e) => {
    e.stopPropagation();
    openSettings();
  });
  $("btn-drawer-back").addEventListener("click", (e) => {
    e.stopPropagation();
    closeDrawer();
  });
  app.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    if (drawerOpen || colorOpen || hotkeyOpen) return;
    // 迷你胶囊只有 76px，菜单放不下。先展开到面板，再贴卡片右缘打开，五项都能点到。
    if (!isExpanded()) {
      void setExpanded(true);
      void whenPresented().then(() => {
        if (!isExpanded() || drawerOpen || colorOpen || hotkeyOpen) return;
        const box = app.getBoundingClientRect();
        openMenu(box.right - 20, box.top + 28);
      });
      return;
    }
    openMenu(e.clientX, e.clientY);
  });
  document.addEventListener("pointerdown", (e) => {
    if (!ctxMenu.contains(e.target as Node)) hideMenu();
  }, true);
  document.addEventListener("keydown", (e) => {
    if (e.key !== "Escape") return;
    if (colorOpen || hotkeyOpen) return;
    if (menuOpen) { hideMenu(); return; }
    if (drawerOpen) { closeDrawer(); return; }
    collapseToMini();
  });
  bridge.onBlur(() => collapseToMini());
  bridge.onOpenSettings(() => { openSettings(); });
  syncInert();
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(value, max));
}

function boxesOverlap(a: DOMRect, left: number, top: number, width: number, height: number) {
  return left < a.right && left + width > a.left && top < a.bottom && top + height > a.top;
}

/** 迷你态面板被 visibility / opacity 藏起时，时钟和主键不算可见遮挡。 */
function isShown(el: HTMLElement | null): el is HTMLElement {
  if (!el) return false;
  for (let node: HTMLElement | null = el; node; node = node.parentElement) {
    const style = getComputedStyle(node);
    if (style.display === "none" || style.visibility === "hidden" || Number(style.opacity) === 0) return false;
  }
  const box = el.getBoundingClientRect();
  return box.width > 0 && box.height > 0;
}

function shownGuards(): DOMRect[] {
  return [$("panel-time"), $("btn-primary")].filter(isShown).map((el) => el.getBoundingClientRect());
}

function placeNearClick(x: number, y: number, mw: number, mh: number) {
  const box = app.getBoundingClientRect();
  const inset = 4;
  return {
    left: clamp(x, box.left + inset, Math.max(box.left + inset, box.right - mw - inset)),
    top: clamp(y, box.top + inset, Math.max(box.top + inset, box.bottom - mh - inset)),
  };
}

/** 贴卡片右缘 8px，纵向先夹进卡片，再让开与菜单横向相交的时钟和主键。 */
function placeTrailing(clickY: number, mw: number, mh: number, obstacles: DOMRect[]) {
  const box = app.getBoundingClientRect();
  const inset = 8;
  const left = Math.max(box.left + inset, box.right - mw - inset);
  const minTop = box.top + inset;
  const maxTop = Math.max(minTop, box.bottom - mh - inset);
  const preferred = clamp(clickY, minTop, maxTop);
  return { left, top: clearTop(preferred, minTop, maxTop, left, mw, mh, obstacles) };
}

function clearTop(
  preferred: number,
  minTop: number,
  maxTop: number,
  left: number,
  mw: number,
  mh: number,
  obstacles: DOMRect[],
) {
  const hits = (top: number) => obstacles.some((r) => boxesOverlap(r, left, top, mw, mh));
  if (!hits(preferred)) return preferred;

  // top 落在 (rect.top - mh, rect.bottom) 就会盖住该区域；两侧各让 1px，避免亚像素贴边。
  const blocked: Array<[number, number]> = [];
  for (const r of obstacles) {
    if (r.right <= left || r.left >= left + mw) continue;
    blocked.push([r.top - mh - 1, r.bottom + 1]);
  }
  blocked.sort((a, b) => a[0] - b[0]);
  const merged: Array<[number, number]> = [];
  for (const span of blocked) {
    const last = merged[merged.length - 1];
    if (!last || span[0] > last[1]) merged.push([span[0], span[1]]);
    else last[1] = Math.max(last[1], span[1]);
  }

  const options: number[] = [];
  let start = minTop;
  for (const [blockStart, blockEnd] of merged) {
    const gapEnd = Math.min(maxTop, blockStart);
    if (gapEnd >= start) options.push(clamp(preferred, start, gapEnd));
    start = Math.max(start, blockEnd);
  }
  if (start <= maxTop) options.push(clamp(preferred, start, maxTop));

  const clear = options.filter((top) => !hits(top));
  if (!clear.length) return preferred;
  return clear.reduce((best, top) => (Math.abs(top - preferred) < Math.abs(best - preferred) ? top : best));
}

function openMenu(x: number, y: number) {
  menuOpen = true;
  ctxMenu.classList.add("open");
  const mw = ctxMenu.offsetWidth, mh = ctxMenu.offsetHeight;
  const guards = shownGuards();
  const spot = guards.length ? placeTrailing(y, mw, mh, guards) : placeNearClick(x, y, mw, mh);
  ctxMenu.style.left = `${spot.left}px`;
  ctxMenu.style.top = `${spot.top}px`;
  ctxMenu.style.transformOrigin = `${x - spot.left + 8}px ${y - spot.top + 8}px`;
  ctxMenu.setAttribute("aria-hidden", "false");
  syncInert();
  onMenuOpened();
  (ctxMenu.querySelector("button") as HTMLButtonElement | null)?.focus();
}

export function collapseToMini() {
  hideMenu();
  closeDrawer();
  onDismiss();
  if (isExpanded()) setExpanded(false);
  else syncInert();
}

let downPos: { x: number; y: number } | null = null;
let downTime = 0;
let dragging = false;

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
  try { await bridge.invoke("drag_end"); } catch { /* 拖拽线程已结束 */ }
}

function isChromeControl(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el?.closest?.("button, input, textarea, select, a");
}

function attachThresholdDrag(el: HTMLElement) {
  el.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    if (isChromeControl(e.target)) return;
    dragging = false;
    downPos = { x: e.screenX, y: e.screenY };
    downTime = performance.now();
  });
  el.addEventListener("mousemove", (e) => {
    if (!downPos || dragging) return;
    dbg.moves++;
    if (Math.hypot(e.screenX - downPos.x, e.screenY - downPos.y) > 12) startNativeDrag(e);
  });
  el.addEventListener("mouseup", (e) => {
    endNativeDrag();
    if (!downPos) return;
    if (isChromeControl(e.target)) { downPos = null; return; }
    const moved = Math.hypot(e.screenX - downPos.x, e.screenY - downPos.y);
    const quick = performance.now() - downTime < 500;
    downPos = null;
    if (moved < 10 && quick && el.id === "pill") setExpanded(true);
  });
  el.addEventListener("mouseleave", () => { downPos = null; });
}
