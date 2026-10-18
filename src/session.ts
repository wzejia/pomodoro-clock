import { bridge, TimerSnapshot } from "./bridge";
import { contrastingInk } from "./ink";
import { runLabel } from "./menu";

const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;

function pauseCopy(reason: TimerSnapshot["pause_reason"]): string {
  if (reason === "gap") return "间隔异常，这段没有计入";
  if (reason === "restored") return "已恢复，退出期间没有继续计时";
  return "";
}

const app = $("app");
const pillTime = $("pill-time");
const panelTime = $("panel-time");
const panelPhase = $("panel-phase");
const progressBar = $("progress-bar");
const btnToggleRun = $("btn-toggle-run") as HTMLButtonElement;
const iconPlay = $("icon-play") as unknown as SVGElement;
const iconPause = $("icon-pause") as unknown as SVGElement;
const btnPrimary = $("btn-primary") as HTMLButtonElement;
const switchLabel = $("btn-switch").querySelector(".switch-label") as HTMLElement;
const cycleDots = $("cycle-dots");
const pillProgressBar = $("pill-progress-bar");

const PHASE_LABEL: Record<string, string> = {
  work: "专注",
  short_break: "短休息",
  long_break: "长休息",
};

const PHASE_VAR: Record<string, string> = {
  work: "--work",
  short_break: "--break",
  long_break: "--long",
};

let snapshot: TimerSnapshot | null = null;

export function getSnapshot(): TimerSnapshot | null {
  return snapshot;
}

function fmt(ms: number): string {
  const total = Math.ceil(ms / 1000);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

export function render(s: TimerSnapshot) {
  snapshot = s;
  const time = fmt(s.remaining_ms);
  pillTime.textContent = time;
  $("pill-phase").textContent = PHASE_LABEL[s.phase] ?? s.phase;
  panelTime.textContent = time;
  panelPhase.textContent = PHASE_LABEL[s.phase] ?? s.phase;
  app.dataset.phase = s.phase;
  app.dataset.status = s.status;
  const done = s.total_ms > 0 ? 1 - s.remaining_ms / s.total_ms : 0;
  const showBar = done > 0.02;
  progressBar.style.width = showBar ? `${(Math.max(0, done) * 100).toFixed(1)}%` : "0";
  progressBar.style.minWidth = showBar ? "" : "0";
  pillProgressBar.style.width = `${(Math.max(0, Math.min(1, done)) * 100).toFixed(1)}%`;

  const pct = String(Math.round(Math.max(0, Math.min(1, done)) * 100));
  for (const track of [document.querySelector(".pill-progress"), document.querySelector(".progress-track")]) {
    if (!track) continue;
    track.setAttribute("aria-valuenow", pct);
  }
  const reason = s.status === "paused" ? pauseCopy(s.pause_reason) : "";
  const note = $("pause-note");
  note.hidden = reason.length === 0;
  note.textContent = reason;
  const spoken = reason ? `${time}，${reason}` : time;
  pillTime.setAttribute("aria-label", spoken);
  panelTime.setAttribute("aria-label", spoken);
  const focusInput = $("focus-label") as HTMLInputElement;
  if (document.activeElement !== focusInput && focusInput.value !== (s.focus_label ?? "")) {
    focusInput.value = s.focus_label ?? "";
  }

  const label = runLabel(s.status);
  const running = s.status === "running";
  iconPlay.style.display = running ? "none" : "";
  iconPause.style.display = running ? "" : "none";
  btnPrimary.textContent = label;
  btnToggleRun.title = label;
  btnToggleRun.setAttribute("aria-label", label);
  const toBreak = s.phase === "work";
  switchLabel.textContent = toBreak ? "休息" : "专注";

  const fill = getComputedStyle(document.documentElement).getPropertyValue(PHASE_VAR[s.phase] ?? "--work").trim();
  if (fill) document.documentElement.style.setProperty("--on-phase", contrastingInk(fill));

  const every = Math.max(1, s.long_break_every);
  if (cycleDots.childElementCount !== every) {
    cycleDots.replaceChildren();
    for (let i = 0; i < every; i++) {
      const d = document.createElement("span");
      d.className = "cycle-dot";
      cycleDots.append(d);
    }
  }
  const lit = s.phase === "long_break" ? every : s.completed_work_count % every;
  const current = s.phase === "work" ? s.completed_work_count % every : -1;
  cycleDots.querySelectorAll(".cycle-dot").forEach((d, i) => {
    d.classList.toggle("lit", i < lit);
    d.classList.toggle("current", i === current);
  });
}

async function paint(cmd: string) {
  render((await bridge.invoke(cmd)) as TimerSnapshot);
}

export async function toggleRun() {
  if (!snapshot) return;
  if (snapshot.status === "running") await paint("timer_pause");
  else if (snapshot.status === "paused") await paint("timer_resume");
  else await paint("timer_start");
}

export async function resetPhase() {
  await paint("timer_reset");
}

export async function skipPhase() {
  await paint("timer_skip");
}

export function bindSession() {
  bridge.onTick(render);
  const focusInput = $("focus-label") as HTMLInputElement;
  focusInput.addEventListener("change", async () => {
    render((await bridge.invoke("set_focus_label", { label: focusInput.value })) as TimerSnapshot);
  });
  $("btn-toggle-run").addEventListener("click", (e) => { e.stopPropagation(); toggleRun(); });
  $("btn-primary").addEventListener("click", () => toggleRun());
  $("btn-reset").addEventListener("click", () => resetPhase());
  $("btn-switch").addEventListener("click", (e) => { e.stopPropagation(); skipPhase(); });
}
