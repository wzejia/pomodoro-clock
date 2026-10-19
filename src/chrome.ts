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
  $("btn-drawer-back").addEventListener("click", (e) => {
    e.stopPropagation();
    closeDrawer();
  });
  app.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    if (!isExpanded() || drawerOpen || colorOpen || hotkeyOpen) return;
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

function openMenu(x: number, y: number) {
  menuOpen = true;
  ctxMenu.classList.add("open");
  const mw = ctxMenu.offsetWidth, mh = ctxMenu.offsetHeight;
  const left = Math.max(4, Math.min(x, window.innerWidth - mw - 4));
  const top = Math.max(4, Math.min(y, window.innerHeight - mh - 4));
  ctxMenu.style.left = `${left}px`;
  ctxMenu.style.top = `${top}px`;
  ctxMenu.style.transformOrigin = `${x - left + 8}px ${y - top + 8}px`;
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
