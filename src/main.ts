import { bridge } from "./bridge";
import { bindChrome, hideMenu, openSettings, syncInert } from "./chrome";
import { bindMorph } from "./morph";
import { runLabel } from "./menu";
import { bindSession, getSnapshot, render, resetPhase, toggleRun } from "./session";
import { bindSettings, dismissSettingsLayers, isSoundEnabled, loadConfig } from "./settings";
import { bindStats, refreshStats, refreshVisible } from "./stats-view";

window.addEventListener("error", (e) => {
  try { (window as unknown as { __TAURI__?: { core?: { invoke: (c: string, a: unknown) => void } } }).__TAURI__?.core?.invoke("log_js_error", { message: `${e.message} @ ${e.filename}:${e.lineno}` }); } catch { /* 日志失败不影响界面 */ }
});
window.addEventListener("unhandledrejection", (e: PromiseRejectionEvent) => {
  try { (window as unknown as { __TAURI__?: { core?: { invoke: (c: string, a: unknown) => void } } }).__TAURI__?.core?.invoke("log_js_error", { message: `unhandledrejection: ${e.reason}` }); } catch { /* 同上 */ }
});

const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;

if (bridge.isMock) document.body.classList.add("mock-preview");

let askedNotify = false;

let chimeCtx: AudioContext | null = null;
function playChime() {
  if (!isSoundEnabled()) return;
  try {
    chimeCtx ??= new AudioContext();
    const ctx = chimeCtx;
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
  } catch { /* 无声可接受 */ }
}

const ctxAutostart = $("ctx-autostart");
async function refreshAutostartItem() {
  const on = (await bridge.invoke("get_autostart")) as boolean;
  ctxAutostart.classList.toggle("checked", on);
  ctxAutostart.setAttribute("aria-checked", String(on));
}

bindMorph({ onExpandSettled: () => { refreshStats(); }, onVisualChange: syncInert });
bindSession();
bindStats();
bindSettings();
bindChrome({
  onMenuOpened: () => {
    $("ctx-toggle").textContent = runLabel(getSnapshot()?.status);
    refreshAutostartItem();
  },
  onDismiss: () => dismissSettingsLayers(),
});

$("ctx-toggle").addEventListener("click", async (e) => {
  e.stopPropagation();
  hideMenu();
  await toggleRun();
});
$("ctx-reset").addEventListener("click", async (e) => {
  e.stopPropagation();
  hideMenu();
  await resetPhase();
});
$("ctx-settings").addEventListener("click", async (e) => {
  e.stopPropagation();
  hideMenu();
  await openSettings();
});
ctxAutostart.addEventListener("click", async (e) => {
  e.stopPropagation();
  const cur = ctxAutostart.getAttribute("aria-checked") === "true";
  const now = (await bridge.invoke("set_autostart", { enabled: !cur })) as boolean;
  ctxAutostart.classList.toggle("checked", now);
  ctxAutostart.setAttribute("aria-checked", String(now));
  hideMenu();
});
$("ctx-quit").addEventListener("click", () => { bridge.invoke("quit_app"); });

bridge.onPhaseCompleted(() => {
  if (bridge.isMock) playChime();
  refreshVisible();
  if (!askedNotify) {
    askedNotify = true;
    bridge.requestNotifyPermission().catch(() => {});
  }
});

(async () => {
  await loadConfig();
  render((await bridge.invoke("timer_snapshot")) as import("./bridge").TimerSnapshot);
  await refreshStats();
})();
