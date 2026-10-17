import { bridge, DayBar, MonthSummary, StatsSummary } from "./bridge";
import { getSnapshot } from "./session";

const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;

const WEEKDAY = ["一", "二", "三", "四", "五", "六", "日"];
type StatsUnit = "count" | "minutes";
type StatsRange = "week" | "month";
let statsUnit: StatsUnit = "count";
let statsRange: StatsRange = "week";
const now0 = new Date();
let monthCursor = { y: now0.getFullYear(), m: now0.getMonth() + 1 };

const statVal = (b: DayBar) => (statsUnit === "count" ? b.count : b.minutes);
const statUnitLabel = () => (statsUnit === "count" ? "个番茄" : "分钟");
const goalStatVal = () => {
  const s = getSnapshot();
  const g = s?.daily_goal ?? 8;
  return statsUnit === "count" ? g : Math.round((g * (s?.work_ms ?? 25 * 60_000)) / 60000);
};
const chartCeil = (v: number) => Math.ceil(Math.max(1, v) * 1.25);

export async function refreshStats() {
  const sum = (await bridge.invoke("stats_summary")) as StatsSummary;
  $("stat-today-count").textContent = String(sum.today_count);
  $("stat-today-min").textContent = String(sum.today_minutes);
  $("stat-streak").textContent = String(sum.streak_days);
  if (statsRange !== "week") return;
  $("chart-title").textContent = "本周";
  const chart = $("week-chart");
  chart.replaceChildren();
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

export async function refreshMonth() {
  const ms = (await bridge.invoke("stats_month", {
    year: monthCursor.y,
    month: monthCursor.m,
  })) as MonthSummary;
  $("chart-title").textContent = `${ms.year}年${ms.month}月`;
  const grid = $("month-chart");
  grid.replaceChildren();
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
  const cur = new Date();
  const isCurrent = monthCursor.y === cur.getFullYear() && monthCursor.m === cur.getMonth() + 1;
  ($("month-next") as HTMLButtonElement).disabled = isCurrent;
}

export function refreshVisible() {
  if (statsRange === "week") return refreshStats();
  return refreshMonth();
}

function setStatsRange(r: StatsRange) {
  statsRange = r;
  $("range-week").classList.toggle("active", r === "week");
  $("range-month").classList.toggle("active", r === "month");
  $("range-week").setAttribute("aria-pressed", String(r === "week"));
  $("range-month").setAttribute("aria-pressed", String(r === "month"));
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
  $("unit-count").setAttribute("aria-pressed", String(u === "count"));
  $("unit-min").setAttribute("aria-pressed", String(u === "minutes"));
  if (statsRange === "week") refreshStats(); else refreshMonth();
}

export function bindStats() {
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
}
