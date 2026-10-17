import { bridge, PhaseColors, ThemeName, TimerConfig, TimerSnapshot } from "./bridge";
import { setColorLayer, setHotkeyLayer } from "./chrome";
import { render } from "./session";
import { refreshVisible } from "./stats-view";

const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;

const toggleAutoStart = $("toggle-auto-start");
const hotkeyCap = $("hotkey-cap") as HTMLButtonElement;
const hotkeyOff = $("hotkey-off") as HTMLButtonElement;

type ColorKey = "work" | "short_break" | "long_break";
const COLOR_VAR: Record<ColorKey, string> = { work: "--work", short_break: "--break", long_break: "--long" };
const TEXT_VAR: Record<ColorKey, string> = { work: "--work-text", short_break: "--break-text", long_break: "--long-text" };
let customColors: Record<ColorKey, string | null> = { work: null, short_break: null, long_break: null };

function flashHexError(el: HTMLElement, onRecover: () => void) {
  el.classList.add("error");
  const prev = (el as HTMLElement & { __hexErrTimer?: number }).__hexErrTimer;
  if (prev) window.clearTimeout(prev);
  (el as HTMLElement & { __hexErrTimer?: number }).__hexErrTimer = window.setTimeout(() => {
    el.classList.remove("error");
    onRecover();
  }, 1500);
}

export function applyColors() {
  const root = document.documentElement;
  const dark = root.dataset.theme === "dark";
  (Object.keys(COLOR_VAR) as ColorKey[]).forEach((k) => {
    const c = customColors[k];
    if (c) {
      root.style.setProperty(COLOR_VAR[k], c);
      root.style.setProperty(TEXT_VAR[k], dark ? c : `color-mix(in srgb, ${c} 62%, black)`);
    } else {
      root.style.removeProperty(COLOR_VAR[k]);
      root.style.removeProperty(TEXT_VAR[k]);
    }
  });
}

function syncColorUI() {
  const root = document.documentElement;
  const shownOf = (k: ColorKey) =>
    customColors[k] ?? getComputedStyle(root).getPropertyValue(COLOR_VAR[k]).trim();
  document.querySelectorAll<HTMLInputElement>(".color-hex[data-color]").forEach((el) => {
    el.value = shownOf(el.dataset.color as ColorKey);
  });
  document.querySelectorAll<HTMLElement>(".color-swatch").forEach((el) => {
    el.style.background = shownOf(el.dataset.color as ColorKey);
  });
  document.querySelectorAll<HTMLButtonElement>(".color-reset").forEach((b) => {
    b.classList.toggle("default", customColors[b.dataset.color as ColorKey] === null);
  });
}

function persistColors() {
  const colors: PhaseColors = {
    work: customColors.work,
    short_break: customColors.short_break,
    long_break: customColors.long_break,
  };
  bridge.invoke("set_colors", { colors });
}

function applyMaterial(material: string) {
  document.documentElement.dataset.material = material;
  document.querySelectorAll(".material-opt").forEach((b) =>
    b.classList.toggle("active", (b as HTMLElement).dataset.materialOpt === material)
  );
}

function applyTheme(theme: ThemeName) {
  document.documentElement.dataset.theme = theme;
  document.querySelectorAll(".theme-opt").forEach((b) =>
    b.classList.toggle("active", (b as HTMLElement).dataset.themeOpt === theme)
  );
  applyColors();
  syncColorUI();
  bridge.emit("ui-style", { theme, material: document.documentElement.dataset.material });
}

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
let cpDragging = false;
let cpCommitted: string | null = null;
const currentCpHex = () => hsvToHex(cpH, cpS, cpV);

function renderColorPop() {
  colorPop.style.setProperty("--cp-h", String(Math.round(cpH)));
  colorPop.style.setProperty("--cp-sv-color", currentCpHex());
  colorPop.style.setProperty("--cp-hue-color", currentCpHex());
  cpDot.style.left = `${cpS * 100}%`;
  cpDot.style.top = `${(1 - cpV) * 100}%`;
  cpHueDot.style.left = `${(cpH / 360) * 100}%`;
  cpHex.value = currentCpHex();
}

function previewFromPicker() {
  if (!cpKey) return;
  customColors[cpKey] = currentCpHex();
  applyColors();
  syncColorUI();
  renderColorPop();
}

function openColorPop(k: ColorKey) {
  cpKey = k;
  cpCommitted = customColors[k];
  const hex = customColors[k] ?? getComputedStyle(document.documentElement).getPropertyValue(COLOR_VAR[k]).trim();
  ({ h: cpH, s: cpS, v: cpV } = hexToHsv(/^#[0-9a-f]{6}$/i.test(hex) ? hex : "#ff3b30"));
  colorPop.hidden = false;
  setColorLayer(true);
  renderColorPop();
}

function closeColorPop() {
  if (!cpKey) return;
  cpKey = null;
  colorPop.hidden = true;
  setColorLayer(false);
  persistColors();
}

export function dismissSettingsLayers() {
  if (cpKey) {
    if (cpCommitted === null) customColors[cpKey] = null;
    else customColors[cpKey] = cpCommitted;
    applyColors();
    syncColorUI();
    cpKey = null;
    colorPop.hidden = true;
    setColorLayer(false);
    persistColors();
  }
  cancelHotkeyRecord();
}

function bindDrag(surface: HTMLElement, map: (ev: PointerEvent, rect: DOMRect) => void) {
  surface.addEventListener("pointerdown", (e) => {
    if (!cpKey) return;
    e.preventDefault();
    cpDragging = true;
    try { surface.setPointerCapture(e.pointerId); } catch { /* WebView2 可能拒绝捕获 */ }
    const move = (ev: PointerEvent) => {
      if (ev.buttons === 0) { up(); return; }
      map(ev, surface.getBoundingClientRect());
      previewFromPicker();
    };
    const up = () => {
      cpDragging = false;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
      persistColors();
      if (cpKey) cpCommitted = customColors[cpKey];
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
    move(e);
  });
}

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
  const snap = (await bridge.invoke("set_config", {
    workMin: readStep("work"),
    shortBreakMin: readStep("short"),
    longBreakMin: readStep("long"),
    longBreakEvery: readStep("every"),
    autoStartNext: toggleAutoStart.getAttribute("aria-checked") === "true",
    dailyGoal: readStep("goal"),
  })) as TimerSnapshot;
  render(snap);
  await refreshVisible();
}

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
  setHotkeyLayer(false);
  document.removeEventListener("keydown", onHotkeyRecordKey, true);
  document.removeEventListener("pointerdown", onHotkeyRecordAway, true);
}
function cancelHotkeyRecord() { stopHotkeyRecord(); renderHotkey(); }

function startHotkeyRecord() {
  if (hotkeyRecording) return;
  hotkeyRecording = true;
  setHotkeyLayer(true);
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
  e.preventDefault();
  e.stopImmediatePropagation();
  if (e.key === "Escape") { cancelHotkeyRecord(); return; }
  if (e.key === "Backspace" || e.key === "Delete") { stopHotkeyRecord(); await applyHotkey(false); return; }
  if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
  const mods = (e.ctrlKey ? "Ctrl+" : "") + (e.altKey ? "Alt+" : "") + (e.shiftKey ? "Shift+" : "") + (e.metaKey ? "Super+" : "");
  if (!mods) {
    hotkeyCap.textContent = "需要修饰键…";
    window.setTimeout(() => { if (hotkeyRecording) hotkeyCap.textContent = "按修饰键+主键…"; }, 900);
    return;
  }
  const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  stopHotkeyRecord();
  await applyHotkey(true, mods + key);
}
async function applyHotkey(enabled: boolean, shortcut?: string) {
  try {
    hotkeyState = (await bridge.invoke("set_hotkey", shortcut ? { enabled, shortcut } : { enabled })) as HotkeyUIState;
    renderHotkey();
  } catch (err) {
    hotkeyCap.classList.remove("recording");
    hotkeyCap.classList.add("error");
    hotkeyCap.textContent = String(err).includes("修饰键") || String(err).includes("识别") ? "不支持的按键" : "被占用，已保留原键";
    window.clearTimeout(hotkeyErrTimer);
    hotkeyErrTimer = window.setTimeout(renderHotkey, 2200);
  }
}

export async function loadConfig() {
  const cfg = (await bridge.invoke("get_config")) as TimerConfig;
  ($("input-work") as HTMLInputElement).value = String(Math.round(cfg.work_ms / 60000));
  ($("input-short") as HTMLInputElement).value = String(Math.round(cfg.short_break_ms / 60000));
  ($("input-long") as HTMLInputElement).value = String(Math.round(cfg.long_break_ms / 60000));
  ($("input-every") as HTMLInputElement).value = String(cfg.long_break_every);
  ($("input-goal") as HTMLInputElement).value = String(cfg.daily_goal);
  toggleAutoStart.setAttribute("aria-checked", String(cfg.auto_start_next));
  toggleAutoStart.classList.toggle("on", cfg.auto_start_next);
  applyMaterial(cfg.material);
  const safeHex = (s: string | null | undefined) =>
    s && /^#[0-9a-fA-F]{6}$/.test(s) ? s.toLowerCase() : null;
  customColors = {
    work: safeHex(cfg.colors?.work),
    short_break: safeHex(cfg.colors?.short_break),
    long_break: safeHex(cfg.colors?.long_break),
  };
  applyTheme(cfg.theme === "dark" ? "dark" : "light");
  await refreshHotkey();
}

export function bindSettings() {
  bindDrag(cpSv, (ev, r) => {
    cpS = Math.max(0, Math.min(1, (ev.clientX - r.left) / r.width));
    cpV = 1 - Math.max(0, Math.min(1, (ev.clientY - r.top) / r.height));
  });
  bindDrag(cpHue, (ev, r) => {
    cpH = Math.max(0, Math.min(1, (ev.clientX - r.left) / r.width)) * 360;
  });
  cpSv.addEventListener("pointermove", (e) => {
    if (!cpKey || cpDragging) return;
    const r = cpSv.getBoundingClientRect();
    const hs = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
    const hv = 1 - Math.max(0, Math.min(1, (e.clientY - r.top) / r.height));
    cpDot.style.left = `${hs * 100}%`;
    cpDot.style.top = `${(1 - hv) * 100}%`;
    colorPop.style.setProperty("--cp-sv-color", hsvToHex(cpH, hs, hv));
  });
  cpSv.addEventListener("pointerleave", () => { if (cpKey && !cpDragging) renderColorPop(); });
  cpHue.addEventListener("pointermove", (e) => {
    if (!cpKey || cpDragging) return;
    const r = cpHue.getBoundingClientRect();
    const hh = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * 360;
    cpHueDot.style.left = `${(hh / 360) * 100}%`;
    colorPop.style.setProperty("--cp-hue-color", hsvToHex(hh, cpS, cpV));
  });
  cpHue.addEventListener("pointerleave", () => { if (cpKey && !cpDragging) renderColorPop(); });
  cpHex.addEventListener("change", () => {
    const v = cpHex.value.trim().toLowerCase();
    if (/^#[0-9a-f]{6}$/.test(v)) {
      ({ h: cpH, s: cpS, v: cpV } = hexToHsv(v));
      previewFromPicker();
      persistColors();
      if (cpKey) cpCommitted = customColors[cpKey];
    } else flashHexError(cpHex, renderColorPop);
  });
  colorPop.querySelector<HTMLButtonElement>(".cp-done")!.addEventListener("click", (e) => {
    e.stopPropagation();
    closeColorPop();
  });
  document.querySelectorAll<HTMLButtonElement>(".color-swatch").forEach((b) => {
    b.addEventListener("click", (e) => {
      e.stopPropagation();
      openColorPop(b.dataset.color as ColorKey);
    });
  });
  document.querySelector(".drawer")!.addEventListener("pointerdown", (e) => {
    if (!cpKey) return;
    const t = e.target as HTMLElement;
    if (!colorPop.contains(t) && !t.closest(".color-swatch")) closeColorPop();
  }, true);
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && cpKey) {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (cpKey) {
        customColors[cpKey] = cpCommitted;
        applyColors();
        syncColorUI();
      }
      cpKey = null;
      colorPop.hidden = true;
      setColorLayer(false);
    }
  }, true);
  document.querySelectorAll<HTMLInputElement>(".color-hex[data-color]").forEach((input) => {
    input.addEventListener("change", () => {
      const k = input.dataset.color as ColorKey;
      const v = input.value.trim().toLowerCase();
      if (/^#[0-9a-f]{6}$/.test(v)) {
        input.value = v;
        customColors[k] = v;
        applyColors();
        syncColorUI();
        persistColors();
      } else flashHexError(input, syncColorUI);
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
  toggleAutoStart.addEventListener("click", (e) => {
    e.stopPropagation();
    const on = toggleAutoStart.getAttribute("aria-checked") !== "true";
    toggleAutoStart.setAttribute("aria-checked", String(on));
    toggleAutoStart.classList.toggle("on", on);
    applyConfig();
  });
  for (const id of ["input-work", "input-short", "input-long", "input-every", "input-goal"]) {
    $(id).addEventListener("change", applyConfig);
  }
  document.querySelectorAll<HTMLButtonElement>(".step-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const [which, dir] = btn.dataset.step!.split(":");
      const [id, min, max] = STEP_RANGE[which];
      const input = $(id) as HTMLInputElement;
      input.value = String(Math.max(min, Math.min(max, Number(input.value) + Number(dir))));
      applyConfig();
    });
  });
  hotkeyCap.addEventListener("click", (e) => {
    e.stopPropagation();
    if (hotkeyRecording) cancelHotkeyRecord(); else startHotkeyRecord();
  });
  hotkeyOff.addEventListener("click", async (e) => { e.stopPropagation(); await applyHotkey(false); });
  document.querySelectorAll<HTMLButtonElement>(".theme-opt").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const theme = btn.dataset.themeOpt === "dark" ? "dark" : "light";
      applyTheme(theme);
      await bridge.invoke("set_theme", { theme });
    });
  });
  document.querySelectorAll<HTMLButtonElement>(".material-opt").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const material = btn.dataset.materialOpt!;
      applyMaterial(material);
      bridge.emit("ui-style", {
        theme: document.documentElement.dataset.theme,
        material,
      });
      await bridge.invoke("set_material", { material });
    });
  });
}
