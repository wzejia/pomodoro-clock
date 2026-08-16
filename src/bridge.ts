/* 前后端桥：真实 Tauri 环境走 IPC；纯浏览器环境走 mock（仅供截图评审/开发预览，
   mock 不参与真实应用的计时真相） */

export type Phase = "work" | "short_break" | "long_break";
export type Status = "idle" | "running" | "paused";

export interface TimerSnapshot {
  phase: Phase;
  status: Status;
  remaining_ms: number;
  total_ms: number;
  completed_work_count: number;
  long_break_every: number;
}

export interface PhaseCompleted {
  from: Phase;
  to: Phase;
  completed_work_count: number;
}

export type Material = "classic" | "liquid_glass";

export interface HotkeyConfig { enabled: boolean; shortcut: string; }
export interface HotkeyState extends HotkeyConfig { effective: string | null; }

export interface TimerConfig {
  work_ms: number;
  short_break_ms: number;
  long_break_ms: number;
  long_break_every: number;
  auto_start_next: boolean;
  material: Material;
  hotkey: HotkeyConfig;
}

export interface DayBar { date: string; count: number; minutes: number; }
export interface StatsSummary {
  today_count: number;
  today_minutes: number;
  week: DayBar[];
  streak_days: number;
}

export interface MonthSummary {
  year: number;
  month: number;
  /** 当月 1 号星期偏移（0=周一 … 6=周日），月历首行缩进 */
  first_weekday: number;
  days: DayBar[];
}

export interface Bridge {
  isMock: boolean;
  invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown>;
  onTick(cb: (s: TimerSnapshot) => void): void;
  onPhaseCompleted(cb: (e: PhaseCompleted) => void): void;
  onBlur(cb: () => void): void;
  /** 托盘「设置…」→ 展开面板+滑入抽屉（与右键菜单同链路） */
  onOpenSettings(cb: () => void): void;
  /** 通用事件总线（v11 托盘菜单页主题/材质对表、整收联动用） */
  emit(event: string, payload?: unknown): Promise<void>;
  onEvent(event: string, cb: (payload: any) => void): void;
  morphBegin(width: number, height: number): Promise<[number, number]>;
  morphCommit(width: number, height: number): Promise<void>;
  requestNotifyPermission(): Promise<void>;
}

declare global {
  interface Window {
    __TAURI__?: unknown;
  }
}

function createTauriBridge(): Bridge {
  // 运行时从全局取（withGlobalTauri: true），避免打包依赖路径问题
  const tauri = (window as any).__TAURI__;
  const invoke = tauri.core.invoke;
  const listen = tauri.event.listen;
  return {
    isMock: false,
    invoke: (cmd, args) => invoke(cmd, args),
    onTick(cb) {
      listen("timer-tick", (e: any) => cb(e.payload.snapshot));
    },
    onPhaseCompleted(cb) {
      listen("phase-completed", (e: any) => cb(e.payload));
    },
    onBlur(cb) {
      // Rust on_window_event(Focused(false)) → 失焦即收（瞬态面板）
      listen("window-blurred", () => cb());
    },
    onOpenSettings(cb) {
      listen("open-settings", () => cb());
    },
    emit: (e, p) => tauri.event.emit(e, p),
    onEvent(e, cb) {
      listen(e, (ev: any) => cb(ev.payload));
    },
    async morphBegin(width, height) {
      // 窗口瞬时到位（防出屏钳制），返回旧原点在新窗口内的 CSS px 偏移
      return (await invoke("morph_begin", { width, height })) as [number, number];
    },
    async morphCommit(width, height) {
      await invoke("morph_commit", { width, height });
    },
    async requestNotifyPermission() {
      const n = tauri.notification;
      let granted = await n.isPermissionGranted();
      if (!granted) {
        granted = (await n.requestPermission()) === "granted";
      }
    },
  };
}

/* ---------------- mock（浏览器预览用） ---------------- */

function createMockBridge(): Bridge {
  const tickCbs: ((s: TimerSnapshot) => void)[] = [];
  const phaseCbs: ((e: PhaseCompleted) => void)[] = [];

  let phase: Phase = "work";
  let status: Status = "running";
  const durations: Record<Phase, number> = {
    work: 25 * 60_000,
    short_break: 5 * 60_000,
    long_break: 15 * 60_000,
  };
  let remaining = 12 * 60_000 + 34_000;
  let completed = 2;
  let longEvery = 4;
  let material: Material = "liquid_glass";
  let last = Date.now();
  // v11：mock 快捷键态（本机 Ctrl+Alt+P 被占用，演示与真机同口径的降级值）
  let hotkeyEnabled = true;
  let hotkeyShortcut = "Ctrl+Alt+O";

  const snap = (): TimerSnapshot => ({
    phase, status, remaining_ms: remaining, total_ms: durations[phase],
    completed_work_count: completed, long_break_every: longEvery,
  });

  setInterval(() => {
    const now = Date.now();
    if (status === "running") remaining = Math.max(0, remaining - (now - last));
    last = now;
    tickCbs.forEach((cb) => cb(snap()));
  }, 250);

  const today = new Date();
  const dayStr = (d: Date) => d.toISOString().slice(0, 10);
  const week: DayBar[] = Array.from({ length: 7 }, (_, i) => {
    const monday = new Date(today);
    monday.setDate(today.getDate() - ((today.getDay() + 6) % 7) + i);
    const counts = [3, 5, 2, 6, 4, 1, 2];
    return { date: dayStr(monday), count: counts[i], minutes: counts[i] * 25 };
  });

  return {
    isMock: true,
    async invoke(cmd, args) {
      switch (cmd) {
        case "timer_start": status = "running"; remaining = durations[phase]; break;
        case "timer_pause": status = "paused"; break;
        case "timer_resume": status = "running"; break;
        case "timer_reset": status = "idle"; remaining = durations[phase]; break;
        case "timer_skip":
          phase = phase === "work" ? "short_break" : "work";
          status = "idle"; remaining = durations[phase];
          break;
        case "timer_snapshot": return snap();
        case "set_config":
          // 前端发 camelCase（Tauri 侧自动转 snake_case；mock 直收 camelCase）
          durations.work = (args?.workMin as number) * 60_000;
          durations.short_break = (args?.shortBreakMin as number) * 60_000;
          durations.long_break = (args?.longBreakMin as number) * 60_000;
          longEvery = (args?.longBreakEvery as number) ?? longEvery;
          if (status === "idle") remaining = durations[phase];
          break;
        case "get_config":
          return {
            work_ms: durations.work, short_break_ms: durations.short_break,
            long_break_ms: durations.long_break, long_break_every: longEvery,
            auto_start_next: false, material,
            hotkey: { enabled: hotkeyEnabled, shortcut: hotkeyShortcut },
          } satisfies TimerConfig;
        case "set_material":
          material = (args?.material as Material) ?? material;
          break;
        case "get_hotkey":
          return { enabled: hotkeyEnabled, shortcut: hotkeyShortcut, effective: hotkeyEnabled ? hotkeyShortcut : null } satisfies HotkeyState;
        case "set_hotkey":
          // mock 模拟 Ctrl+Alt+P 被系统占用，便于评审演示错误态
          if (args?.enabled && args?.shortcut === "Ctrl+Alt+P") throw new Error("快捷键被占用或不可用（mock），已保留原键");
          hotkeyEnabled = !!args?.enabled;
          if (typeof args?.shortcut === "string") hotkeyShortcut = args.shortcut;
          return { enabled: hotkeyEnabled, shortcut: hotkeyShortcut, effective: hotkeyEnabled ? hotkeyShortcut : null } satisfies HotkeyState;
        case "stats_summary":
          return {
            today_count: 2, today_minutes: 50, week, streak_days: 5,
          } satisfies StatsSummary;
        case "stats_month": {
          // 确定性伪数据（截图评审用）：有零有峰能看热力层次
          const y = args?.year as number, m = args?.month as number;
          const n = new Date(y, m, 0).getDate();
          const first = (new Date(y, m - 1, 1).getDay() + 6) % 7; // 周一=0
          const days: DayBar[] = Array.from({ length: n }, (_, i) => {
            const c = (i * 7 + 3) % 5;
            return {
              date: `${y}-${String(m).padStart(2, "0")}-${String(i + 1).padStart(2, "0")}`,
              count: c, minutes: c * 25,
            };
          });
          return { year: y, month: m, first_weekday: first, days } satisfies MonthSummary;
        }
        case "get_autostart": return false;
        case "set_autostart": return args?.enabled ?? false;
        case "quit_app": return null;
      }
      return null;
    },
    onTick(cb) { tickCbs.push(cb); },
    onPhaseCompleted(cb) { phaseCbs.push(cb); },
    onBlur(_cb) { /* mock 预览无窗口焦点概念，不触发 */ },
    onOpenSettings(_cb) { /* mock 预览无托盘，不触发 */ },
    async emit() {},
    onEvent() {},
    async morphBegin(_w, _h) {
      return [0, 0] as [number, number];
    },
    async morphCommit(_w, _h) {},
    async requestNotifyPermission() {},
  };
}

export const bridge: Bridge = window.__TAURI__ ? createTauriBridge() : createMockBridge();
