//! 番茄钟计时状态机 —— 唯一计时真相来源。
//! 所有时间均为毫秒。`tick(now_ms)` 按调用方给的逻辑时间扣减，测试可一次跳过整段番茄。
//! 生产环境走 `sample(mono_ms, wall_ms)`：单调时钟扣减，墙钟只用来识别睡眠和校时。
//! 两次采样间隔超过 [`MAX_SAMPLE_GAP_MS`] 时自动暂停，这段不计入专注。

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Work => "专注",
            Phase::ShortBreak => "短休息",
            Phase::LongBreak => "长休息",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Idle,
    Running,
    Paused,
}

/// 生产采样允许的最大间隔。tick 线程 250ms 一拍，超过几秒就是睡眠、挂起或校时，不是专注。
pub const MAX_SAMPLE_GAP_MS: u64 = 3_000;

/// `sample` 的结果。`tick` 仍按逻辑时间前进，不在这里拦截测试里的整段跳时。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sample {
    /// 不在运行，什么都没改。
    Idle,
    /// 已按单调时钟扣减。内层 Some 表示阶段刚完成。
    Ticked(Option<PhaseCompleted>),
    /// 间隔异常，已暂停，剩余时间原样保留。
    AutoPaused,
}

/// 为什么停在暂停。手暂停、睡眠或校时、进程恢复，界面要用不同的一句话。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PauseReason {
    User,
    Gap,
    Restored,
}

/// 落盘的进行中阶段。重启时 Running 会改成 Paused，避免把关机时间算进番茄。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimerSession {
    pub phase: Phase,
    pub status: Status,
    pub phase_total_ms: u64,
    pub remaining_ms: u64,
    /// 旧文件没有该字段时，暂停按手暂停理解。
    #[serde(default)]
    pub pause_reason: Option<PauseReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TimerConfig {
    pub work_ms: u64,
    pub short_break_ms: u64,
    pub long_break_ms: u64,
    /// 每完成 N 个工作番茄后进入长休息
    pub long_break_every: u32,
    /// 阶段结束后是否自动开跑下一阶段；false=停表等手动开始（默认）
    pub auto_start_next: bool,
    /// 每日目标番茄数：统计周柱/月格刻度与目标虚线的锚点（个数；分钟口径=个数×工作时长）
    pub daily_goal: u32,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            work_ms: 25 * 60 * 1000,
            short_break_ms: 5 * 60 * 1000,
            long_break_ms: 15 * 60 * 1000,
            long_break_every: 4,
            auto_start_next: false,
            daily_goal: 8,
        }
    }
}

impl TimerConfig {
    /// 从文件加载；文件不存在或损坏时返回默认配置（不崩），缺字段补默认值
    pub fn load(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            // 08-18 R1：手改 config 写 long_break_every=0 会令取模除零——load 钳回 ≥1
            //（set_config 命令层已有 .max(1)，此为绕过 UI 直改文件时的第二道闸）
            Ok(s) => {
                let mut cfg = serde_json::from_str::<Self>(&s).unwrap_or_else(|e| {
                    eprintln!("[config] 计时配置解析失败，回默认（{e}）");
                    Self::default()
                });
                cfg.long_break_every = cfg.long_break_every.max(1);
                cfg
            }
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        // 08-18 P2：序列化失败转 io::Error 走 ?，不在 save 路径 panic
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        crate::atomic_file::write_atomic(path, json.as_bytes())
    }
}

/// tick 产生的事件：阶段完成，自动切换到下一阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct PhaseCompleted {
    pub from: Phase,
    pub to: Phase,
    /// 若 from == Work，这是本轮完成的工作番茄序号（1 起）
    pub completed_work_count: u32,
    /// `from` 这一阶段锁定的满时长。统计必须用它，不能用之后可能已改过的配置。
    pub phase_total_ms: u64,
}

impl PhaseCompleted {
    /// 工作阶段完成时写入统计的秒数。休息阶段不记。
    pub fn stats_duration_secs(self) -> Option<u64> {
        if self.from == Phase::Work {
            Some(self.phase_total_ms / 1000)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TimerSnapshot {
    pub phase: Phase,
    pub status: Status,
    pub remaining_ms: u64,
    pub total_ms: u64,
    pub completed_work_count: u32,
    /// 循环指示器的数据源：圆点数随配置变，前端只取模不自计数
    pub long_break_every: u32,
    /// 统计目标锚点（每日目标番茄数），周柱/月格刻度用；分钟口径换算需 work_ms 一并下发
    pub daily_goal: u32,
    /// 当前工作阶段时长（分钟口径目标线换算用；阶段字段而非剩余值，break 阶段下也取 work）
    pub work_ms: u64,
    /// 仅暂停时有值。
    pub pause_reason: Option<PauseReason>,
}

pub struct Timer {
    config: TimerConfig,
    phase: Phase,
    status: Status,
    remaining_ms: u64,
    /// 本阶段锁定的满时长。运行或暂停中改配置不改它，下一阶段才读新时长。
    phase_total_ms: u64,
    /// Running 时最后一次采样的单调时间戳
    last_tick_ms: u64,
    /// Running 时最后一次采样的墙钟。0 表示还没锚定，只按单调间隔判断。
    last_wall_ms: u64,
    completed_work_count: u32,
    pause_reason: Option<PauseReason>,
}

impl Timer {
    pub fn new(config: TimerConfig) -> Self {
        Self {
            phase: Phase::Work,
            status: Status::Idle,
            remaining_ms: config.work_ms,
            phase_total_ms: config.work_ms,
            last_tick_ms: 0,
            last_wall_ms: 0,
            completed_work_count: 0,
            pause_reason: None,
            config,
        }
    }

    pub fn session(&self) -> TimerSession {
        TimerSession {
            phase: self.phase,
            status: self.status,
            phase_total_ms: self.phase_total_ms,
            remaining_ms: self.remaining_ms,
            pause_reason: self.pause_reason,
        }
    }

    /// 从磁盘恢复。Running 一律改成 Paused：退出到再次启动之间没有人在专注。
    pub fn restore_session(&mut self, session: TimerSession) {
        self.phase = session.phase;
        let fallback = self.phase_duration(session.phase).max(1);
        let total = if session.phase_total_ms == 0 {
            fallback
        } else {
            session.phase_total_ms
        };
        self.phase_total_ms = total;
        self.remaining_ms = session.remaining_ms.min(total);
        self.status = match session.status {
            Status::Running | Status::Paused => Status::Paused,
            Status::Idle => Status::Idle,
        };
        self.pause_reason = match session.status {
            Status::Running => Some(PauseReason::Restored),
            Status::Paused => session.pause_reason.or(Some(PauseReason::User)),
            Status::Idle => None,
        };
        self.last_tick_ms = 0;
        self.last_wall_ms = 0;
    }

    /// 刷盘窗口结束。墙钟跳变超过阈值就暂停且不扣时；否则把锚点拨到现在，磁盘等待不算专注。
    pub fn finish_io(&mut self, wall_before: u64, mono_after: u64, wall_after: u64) {
        if self.status != Status::Running {
            return;
        }
        let forward = wall_after.saturating_sub(wall_before);
        let back = wall_before.saturating_sub(wall_after);
        if forward > MAX_SAMPLE_GAP_MS || back > MAX_SAMPLE_GAP_MS {
            eprintln!(
                "[timer] 刷盘期间墙钟异常（前跳 {forward} ms，回拨 {back} ms），不计入专注，已暂停"
            );
            self.status = Status::Paused;
            self.pause_reason = Some(PauseReason::Gap);
            self.last_tick_ms = mono_after;
            self.last_wall_ms = wall_after;
            return;
        }
        self.reanchor(mono_after, wall_after);
    }

    pub fn mark_wall(&mut self, wall_ms: u64) {
        self.last_wall_ms = wall_ms;
    }

    /// 把采样锚点拨到现在，不扣剩余时间。刷盘之后调用，避免磁盘等待被当成专注或睡眠。
    pub fn reanchor(&mut self, mono_ms: u64, wall_ms: u64) {
        if self.status == Status::Running {
            self.last_tick_ms = mono_ms;
            self.last_wall_ms = wall_ms;
        }
    }

    pub fn config(&self) -> &TimerConfig {
        &self.config
    }

    pub fn work_duration_secs(&self) -> u64 {
        self.config.work_ms / 1000
    }

    pub fn completed_count(&self) -> u32 {
        self.completed_work_count
    }

    pub fn restore_completed(&mut self, n: u32) {
        self.completed_work_count = n;
    }

    fn phase_duration(&self, phase: Phase) -> u64 {
        match phase {
            Phase::Work => self.config.work_ms,
            Phase::ShortBreak => self.config.short_break_ms,
            Phase::LongBreak => self.config.long_break_ms,
        }
    }

    pub fn snapshot(&self) -> TimerSnapshot {
        TimerSnapshot {
            phase: self.phase,
            status: self.status,
            remaining_ms: self.remaining_ms,
            total_ms: self.phase_total_ms,
            completed_work_count: self.completed_work_count,
            long_break_every: self.config.long_break_every,
            daily_goal: self.config.daily_goal,
            work_ms: self.config.work_ms,
            pause_reason: self.pause_reason,
        }
    }

    /// 开始（Idle → Running）；运行中调用等价于重置后重新开始
    pub fn start(&mut self, now_ms: u64) {
        self.enter_phase(self.phase);
        self.status = Status::Running;
        self.pause_reason = None;
        self.last_tick_ms = now_ms;
        // 墙钟由调用方 mark_wall 重新锚定。留 0 则暂停期间的墙钟差不会在下一次采样里被当成睡眠。
        self.last_wall_ms = 0;
    }

    pub fn pause(&mut self, now_ms: u64) {
        if self.status != Status::Running {
            return;
        }
        self.accumulate(now_ms);
        self.status = Status::Paused;
        self.pause_reason = Some(PauseReason::User);
    }

    pub fn resume(&mut self, now_ms: u64) {
        if self.status != Status::Paused {
            return;
        }
        self.status = Status::Running;
        self.pause_reason = None;
        self.last_tick_ms = now_ms;
        self.last_wall_ms = 0;
    }

    /// 用户暂停。间隔异常时不扣这段时间，直接暂停。
    pub fn pause_at(&mut self, mono_ms: u64, wall_ms: u64) {
        if self.status != Status::Running {
            return;
        }
        if self.disarm_if_gap(mono_ms, wall_ms) {
            return;
        }
        self.pause(mono_ms);
        self.last_wall_ms = wall_ms;
    }

    /// 生产环境的一次采样。正常间隔等价于 `tick(mono_ms)`；异常间隔自动暂停且不扣时。
    pub fn sample(&mut self, mono_ms: u64, wall_ms: u64) -> Sample {
        if self.status != Status::Running {
            return Sample::Idle;
        }
        if self.disarm_if_gap(mono_ms, wall_ms) {
            return Sample::AutoPaused;
        }
        self.last_wall_ms = wall_ms;
        Sample::Ticked(self.tick(mono_ms))
    }

    /// 间隔异常则暂停并返回 true。剩余时间不动。
    fn disarm_if_gap(&mut self, mono_ms: u64, wall_ms: u64) -> bool {
        let mono_gap = mono_ms.saturating_sub(self.last_tick_ms);
        let (forward, back) = if self.last_wall_ms == 0 {
            (0, 0)
        } else {
            (
                wall_ms.saturating_sub(self.last_wall_ms),
                self.last_wall_ms.saturating_sub(wall_ms),
            )
        };
        if mono_gap <= MAX_SAMPLE_GAP_MS
            && forward <= MAX_SAMPLE_GAP_MS
            && back <= MAX_SAMPLE_GAP_MS
        {
            return false;
        }
        eprintln!(
            "[timer] 采样间隔异常（单调 {mono_gap} ms，墙钟前跳 {forward} ms，回拨 {back} ms），本段不计入专注，已自动暂停"
        );
        self.status = Status::Paused;
        self.pause_reason = Some(PauseReason::Gap);
        self.last_tick_ms = mono_ms;
        self.last_wall_ms = wall_ms;
        true
    }

    /// 重置：回到当前阶段初始、Idle。本组完成数保留。
    pub fn reset(&mut self) {
        self.enter_phase(self.phase);
        self.status = Status::Idle;
        self.pause_reason = None;
    }

    /// 跳过当前阶段（不计入完成数），进入下一阶段并停表
    pub fn skip(&mut self) -> PhaseCompleted {
        let from = self.phase;
        let phase_total_ms = self.phase_total_ms;
        let to = self.next_phase_after(from, false);
        self.enter_phase(to);
        self.status = Status::Idle;
        self.pause_reason = None;
        PhaseCompleted {
            from,
            to,
            completed_work_count: self.completed_work_count,
            phase_total_ms,
        }
    }

    /// 写入偏好。空闲时当前阶段的满时长跟着变；运行或暂停中剩余时间和本阶段满时长保持，下一阶段才用新时长。
    pub fn apply_prefs(&mut self, mut config: TimerConfig) {
        config.long_break_every = config.long_break_every.max(1);
        config.daily_goal = config.daily_goal.clamp(1, 20);
        self.config = config;
        if self.status == Status::Idle {
            self.enter_phase(self.phase);
            self.status = Status::Idle;
        }
    }

    pub fn set_config(&mut self, config: TimerConfig) {
        self.apply_prefs(config);
    }

    fn enter_phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.phase_total_ms = self.phase_duration(phase);
        self.remaining_ms = self.phase_total_ms;
    }

    /// 驱动时间前进。返回 Some 表示阶段完成并已自动切换（新阶段自动开跑）。
    pub fn tick(&mut self, now_ms: u64) -> Option<PhaseCompleted> {
        if self.status != Status::Running {
            return None;
        }
        self.accumulate(now_ms);
        if self.remaining_ms > 0 {
            return None;
        }
        let from = self.phase;
        let phase_total_ms = self.phase_total_ms;
        if from == Phase::Work {
            self.completed_work_count += 1;
        }
        let to = self.next_phase_after(from, true);
        self.enter_phase(to);
        // auto_start_next=true 时自动开跑下一阶段；否则停表等手动开始
        self.pause_reason = None;
        if self.config.auto_start_next {
            self.status = Status::Running;
            self.last_tick_ms = now_ms;
        } else {
            self.status = Status::Idle;
        }
        Some(PhaseCompleted {
            from,
            to,
            completed_work_count: self.completed_work_count,
            phase_total_ms,
        })
    }

    fn accumulate(&mut self, now_ms: u64) {
        let elapsed = now_ms.saturating_sub(self.last_tick_ms);
        self.last_tick_ms = now_ms;
        self.remaining_ms = self.remaining_ms.saturating_sub(elapsed);
    }

    fn next_phase_after(&self, from: Phase, completed: bool) -> Phase {
        match from {
            Phase::Work => {
                // 08-18 R1 用点兜底：max(1) 防 0 除零 panic（load/set_config 之外的最后一道闸）
                if completed && self.completed_work_count % self.config.long_break_every.max(1) == 0 {
                    Phase::LongBreak
                } else {
                    Phase::ShortBreak
                }
            }
            Phase::ShortBreak | Phase::LongBreak => Phase::Work,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    fn default_timer() -> Timer {
        Timer::new(TimerConfig::default())
    }

    /// auto_start_next=true 的计时器：阶段结束后自动开跑（旧默认行为）
    fn auto_timer() -> Timer {
        let mut cfg = TimerConfig::default();
        cfg.auto_start_next = true;
        Timer::new(cfg)
    }

    #[test]
    fn start_sets_running_with_full_work_duration() {
        let mut t = default_timer();
        t.start(0);
        let s = t.snapshot();
        assert_eq!(s.status, Status::Running);
        assert_eq!(s.phase, Phase::Work);
        assert_eq!(s.remaining_ms, 25 * MIN);
    }

    #[test]
    fn tick_counts_down_while_running() {
        let mut t = default_timer();
        t.start(0);
        t.tick(10_000);
        assert_eq!(t.snapshot().remaining_ms, 25 * MIN - 10_000);
    }

    #[test]
    fn pause_freezes_remaining() {
        let mut t = default_timer();
        t.start(0);
        t.tick(5_000);
        t.pause(5_000);
        assert_eq!(t.snapshot().remaining_ms, 25 * MIN - 5_000);
        // 暂停中 tick 不动
        assert!(t.tick(60_000).is_none());
        assert_eq!(t.snapshot().remaining_ms, 25 * MIN - 5_000);
    }

    #[test]
    fn resume_after_pause_keeps_time_without_drift() {
        let mut t = default_timer();
        t.start(0);
        t.tick(5_000);
        t.pause(5_000);
        // 暂停 10 分钟墙钟时间
        t.resume(5_000 + 10 * MIN);
        t.tick(5_000 + 10 * MIN + 3_000);
        assert_eq!(t.snapshot().remaining_ms, 25 * MIN - 8_000);
    }

    #[test]
    fn reset_restores_full_duration_and_idle() {
        let mut t = default_timer();
        t.start(0);
        t.tick(60_000);
        t.reset();
        let s = t.snapshot();
        assert_eq!(s.status, Status::Idle);
        assert_eq!(s.remaining_ms, 25 * MIN);
    }

    #[test]
    fn work_completion_auto_switches_to_short_break() {
        let mut t = auto_timer();
        t.start(0);
        let ev = t.tick(25 * MIN).expect("should complete");
        assert_eq!(ev.from, Phase::Work);
        assert_eq!(ev.to, Phase::ShortBreak);
        assert_eq!(ev.completed_work_count, 1);
        let s = t.snapshot();
        assert_eq!(s.phase, Phase::ShortBreak);
        assert_eq!(s.status, Status::Running);
        assert_eq!(s.remaining_ms, 5 * MIN);
    }

    #[test]
    fn work_completion_stops_when_auto_start_disabled() {
        let mut t = default_timer();
        t.start(0);
        let ev = t.tick(25 * MIN).expect("should complete");
        assert_eq!(ev.to, Phase::ShortBreak);
        let s = t.snapshot();
        assert_eq!(s.phase, Phase::ShortBreak);
        assert_eq!(s.status, Status::Idle);
        assert_eq!(s.remaining_ms, 5 * MIN);
    }

    #[test]
    fn break_completion_returns_to_work() {
        let mut t = auto_timer();
        t.start(0);
        t.tick(25 * MIN);
        let ev = t.tick(30 * MIN).expect("break should complete");
        assert_eq!(ev.from, Phase::ShortBreak);
        assert_eq!(ev.to, Phase::Work);
        assert_eq!(t.snapshot().remaining_ms, 25 * MIN);
    }

    #[test]
    fn skip_switches_phase_without_counting() {
        let mut t = default_timer();
        t.start(0);
        let ev = t.skip();
        assert_eq!(ev.from, Phase::Work);
        assert_eq!(ev.to, Phase::ShortBreak);
        assert_eq!(ev.completed_work_count, 0);
        assert_eq!(t.snapshot().status, Status::Idle);
        // 跳过休息回到工作
        let ev2 = t.skip();
        assert_eq!(ev2.to, Phase::Work);
    }

    #[test]
    fn fourth_work_triggers_long_break() {
        let mut t = auto_timer();
        let mut now = 0u64;
        for i in 1..=4u32 {
            t.start(now);
            let ev = t.tick(now + 25 * MIN).expect("work completes");
            now += 25 * MIN;
            assert_eq!(ev.completed_work_count, i);
            if i < 4 {
                assert_eq!(ev.to, Phase::ShortBreak);
                let ev_b = t.tick(now + 5 * MIN).expect("break completes");
                assert_eq!(ev_b.to, Phase::Work);
                now += 5 * MIN;
            } else {
                assert_eq!(ev.to, Phase::LongBreak);
                assert_eq!(t.snapshot().remaining_ms, 15 * MIN);
            }
        }
    }

    #[test]
    fn custom_config_is_respected() {
        let mut t = Timer::new(TimerConfig {
            work_ms: 1_000,
            short_break_ms: 500,
            long_break_ms: 2_000,
            long_break_every: 2,
            auto_start_next: true,
            daily_goal: 6,
        });
        t.start(0);
        let ev = t.tick(1_000).unwrap();
        assert_eq!(ev.to, Phase::ShortBreak);
        assert_eq!(t.snapshot().remaining_ms, 500);
        t.tick(1_500);
        let ev2 = t.tick(2_500).unwrap();
        assert_eq!(ev2.completed_work_count, 2);
        assert_eq!(ev2.to, Phase::LongBreak);
        assert_eq!(t.snapshot().remaining_ms, 2_000);
    }

    #[test]
    fn apply_prefs_keeps_running_phase() {
        let mut t = default_timer();
        t.start(0);
        t.tick(5_000);
        let remain = t.snapshot().remaining_ms;
        let mut cfg = *t.config();
        cfg.daily_goal = 3;
        cfg.auto_start_next = true;
        cfg.work_ms = 50 * MIN;
        t.apply_prefs(cfg);
        let s = t.snapshot();
        assert_eq!(s.status, Status::Running);
        assert_eq!(s.remaining_ms, remain);
        assert_eq!(s.total_ms, 25 * MIN);
        assert_eq!(s.daily_goal, 3);
        assert_eq!(s.work_ms, 50 * MIN);
    }

    #[test]
    fn apply_prefs_paused_keeps_status() {
        let mut t = default_timer();
        t.start(0);
        t.tick(5_000);
        t.pause(5_000);
        let remain = t.snapshot().remaining_ms;
        let mut cfg = *t.config();
        cfg.short_break_ms = 8 * MIN;
        t.apply_prefs(cfg);
        let s = t.snapshot();
        assert_eq!(s.status, Status::Paused);
        assert_eq!(s.remaining_ms, remain);
        assert_eq!(s.phase, Phase::Work);
    }

    #[test]
    fn apply_prefs_idle_refreshes_full_duration() {
        let mut t = default_timer();
        let mut cfg = TimerConfig::default();
        cfg.work_ms = 50 * MIN;
        t.apply_prefs(cfg);
        let s = t.snapshot();
        assert_eq!(s.status, Status::Idle);
        assert_eq!(s.remaining_ms, 50 * MIN);
        assert_eq!(s.total_ms, 50 * MIN);
    }

    #[test]
    fn restore_completed_survives_reset() {
        let mut t = default_timer();
        t.restore_completed(3);
        assert_eq!(t.snapshot().completed_work_count, 3);
        t.reset();
        assert_eq!(t.snapshot().status, Status::Idle);
        assert_eq!(t.snapshot().completed_work_count, 3);
        assert_eq!(t.snapshot().remaining_ms, 25 * MIN);
    }

    #[test]
    fn next_phase_uses_duration_changed_while_running() {
        let mut t = default_timer();
        t.start(0);
        let mut cfg = *t.config();
        cfg.short_break_ms = 8 * MIN;
        t.apply_prefs(cfg);
        let ev = t.tick(25 * MIN).unwrap();
        assert_eq!(ev.to, Phase::ShortBreak);
        assert_eq!(t.snapshot().remaining_ms, 8 * MIN);
        assert_eq!(t.snapshot().total_ms, 8 * MIN);
        assert_eq!(t.snapshot().status, Status::Idle);
    }

    /// 间隔可调：set_config 把 long_break_every 改成 2 后，第 2 个工作番茄即触发长休
    #[test]
    fn set_config_long_break_every_two_triggers_long_break() {
        let mut t = auto_timer();
        let mut cfg = *t.config();
        cfg.long_break_every = 2;
        t.set_config(cfg);
        assert_eq!(t.snapshot().long_break_every, 2);
        let mut now = 0u64;
        // 第 1 个番茄 → 短休
        t.start(now);
        let ev1 = t.tick(now + 25 * MIN).expect("first work completes");
        assert_eq!(ev1.to, Phase::ShortBreak);
        assert_eq!(ev1.completed_work_count, 1);
        now += 25 * MIN;
        let ev_b = t.tick(now + 5 * MIN).expect("break completes");
        assert_eq!(ev_b.to, Phase::Work);
        now += 5 * MIN;
        // 第 2 个番茄 → 长休（默认 every=4 时不会触发）
        t.start(now);
        let ev2 = t.tick(now + 25 * MIN).expect("second work completes");
        assert_eq!(ev2.completed_work_count, 2);
        assert_eq!(ev2.to, Phase::LongBreak);
        assert_eq!(t.snapshot().remaining_ms, 15 * MIN);
    }

    #[test]
    fn config_save_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("pomodoro-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let cfg = TimerConfig {
            work_ms: 30 * MIN,
            short_break_ms: 10 * MIN,
            long_break_ms: 20 * MIN,
            long_break_every: 3,
            auto_start_next: true,
            daily_goal: 12,
        };
        cfg.save(&path).unwrap();
        let loaded = TimerConfig::load(&path);
        assert_eq!(loaded, cfg);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn config_load_missing_file_returns_default() {
        let path = std::env::temp_dir().join("pomodoro-cfg-definitely-not-exist-xyz.json");
        assert_eq!(TimerConfig::load(&path), TimerConfig::default());
    }

    #[test]
    fn config_load_corrupt_file_returns_default() {
        let dir = std::env::temp_dir().join(format!("pomodoro-cfg-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, "{ not valid json !!!").unwrap();
        assert_eq!(TimerConfig::load(&path), TimerConfig::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 08-16：旧 config.json 无 daily_goal 键 → serde(default) 补默认 8（升级路径不炸不断档）
    #[test]
    fn config_load_missing_daily_goal_fills_default() {
        let dir = std::env::temp_dir().join(format!("pomodoro-cfg-nogoal-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"work_ms":1500000,"short_break_ms":300000,"long_break_ms":900000,"long_break_every":4,"auto_start_next":false}"#,
        )
        .unwrap();
        let loaded = TimerConfig::load(&path);
        assert_eq!(loaded.daily_goal, 8);
        assert_eq!(loaded.work_ms, 25 * MIN);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 08-16：snapshot 下发 daily_goal 与 work_ms（周/月刻度与分钟口径目标换算的数据源）
    #[test]
    fn snapshot_carries_goal_and_work_ms() {
        let mut t = default_timer();
        let mut cfg = *t.config();
        cfg.daily_goal = 5;
        cfg.work_ms = 50 * MIN;
        t.set_config(cfg);
        let s = t.snapshot();
        assert_eq!(s.daily_goal, 5);
        assert_eq!(s.work_ms, 50 * MIN);
    }

    /// 08-18 审计 R1：手改 config.json 写 long_break_every=0 → load 须钳回 ≥1，
    /// 且取模用点不得 panic（0 会除零炸 tick 线程）
    #[test]
    fn load_clamps_long_break_every_zero() {
        let dir = std::env::temp_dir().join(format!("pomodoro-cfg-zeroevery-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"work_ms":1000,"short_break_ms":500,"long_break_ms":2000,"long_break_every":0,"auto_start_next":true}"#,
        )
        .unwrap();
        let loaded = TimerConfig::load(&path);
        assert!(loaded.long_break_every >= 1, "load 必须把 0 钳回 ≥1，实际 {}", loaded.long_break_every);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 08-18 审计 R1 用点兜底：即使 config 被构造为 0，next_phase_after 也不得 panic
    #[test]
    fn zero_long_break_every_does_not_panic_in_phase_transition() {
        let mut t = default_timer();
        let mut cfg = *t.config();
        cfg.long_break_every = 0; // 绕过 load/set_config 两道钳制，直击用点
        t.set_config(cfg);
        // 跑完一个工作阶段触发 next_phase_after（0 % max(1,0) 而非除零）
        t.start(0);
        let _ = t.tick(cfg.work_ms);
    }

    #[test]
    fn sample_suspend_gap_pauses_without_completing() {
        let mut t = default_timer();
        t.start(0);
        t.mark_wall(5_000_000);
        // 旧实现按墙钟差扣时，这一跳会直接完成番茄并记入统计。
        let s = t.sample(25 * MIN, 5_000_000 + 25 * MIN);
        assert_eq!(s, Sample::AutoPaused);
        let snap = t.snapshot();
        assert_eq!(snap.phase, Phase::Work);
        assert_eq!(snap.status, Status::Paused);
        assert_eq!(snap.remaining_ms, 25 * MIN);
        assert_eq!(snap.completed_work_count, 0);
    }

    #[test]
    fn sample_wall_jump_forward_pauses_without_deducting() {
        let mut t = default_timer();
        t.start(10_000);
        t.mark_wall(1_000_000);
        let before = t.snapshot().remaining_ms;
        let s = t.sample(10_250, 1_000_000 + 60_000);
        assert_eq!(s, Sample::AutoPaused);
        assert_eq!(t.snapshot().status, Status::Paused);
        assert_eq!(t.snapshot().remaining_ms, before);
    }

    #[test]
    fn sample_wall_rollback_does_not_complete_on_the_next_tick() {
        let mut t = default_timer();
        t.start(1_000);
        t.mark_wall(10_000_000);
        t.sample(1_200, 10_000_200);
        let remain = t.snapshot().remaining_ms;
        let rolled = t.sample(1_400, 10_000_200 - 3_600_000);
        assert_eq!(rolled, Sample::AutoPaused);
        assert_eq!(t.snapshot().phase, Phase::Work);
        assert_eq!(t.snapshot().remaining_ms, remain);
        t.resume(50_000);
        t.mark_wall(20_000_000);
        let next = t.sample(50_400, 20_000_400);
        assert_eq!(next, Sample::Ticked(None));
        assert_eq!(t.snapshot().status, Status::Running);
        assert_eq!(t.snapshot().remaining_ms, remain - 400);
    }

    #[test]
    fn sample_mono_rollback_does_not_deduct_or_complete() {
        let mut t = default_timer();
        t.start(10_000);
        t.mark_wall(5_000_000);
        t.sample(10_200, 5_000_200);
        let remain = t.snapshot().remaining_ms;
        let s = t.sample(9_000, 5_000_400);
        assert_eq!(s, Sample::Ticked(None));
        assert_eq!(t.snapshot().status, Status::Running);
        assert_eq!(t.snapshot().phase, Phase::Work);
        assert_eq!(t.snapshot().remaining_ms, remain);
    }

    #[test]
    fn sample_small_wall_step_keeps_running_on_mono() {
        let mut t = default_timer();
        t.start(1_000);
        t.mark_wall(10_000_000);
        let s = t.sample(1_250, 10_000_000 - 20);
        assert_eq!(s, Sample::Ticked(None));
        assert_eq!(t.snapshot().status, Status::Running);
        assert_eq!(t.snapshot().remaining_ms, 25 * MIN - 250);
    }

    #[test]
    fn pause_at_suspend_does_not_deduct() {
        let mut t = default_timer();
        t.start(0);
        t.mark_wall(1_000);
        t.pause_at(30_000, 31_000);
        assert_eq!(t.snapshot().status, Status::Paused);
        assert_eq!(t.snapshot().remaining_ms, 25 * MIN);
    }

    #[test]
    fn restore_running_session_comes_back_paused() {
        let mut t = default_timer();
        t.restore_completed(2);
        t.restore_session(TimerSession {
            phase: Phase::Work,
            status: Status::Running,
            phase_total_ms: 25 * MIN,
            remaining_ms: 10 * MIN,
            pause_reason: None,
        });
        assert_eq!(t.snapshot().pause_reason, Some(PauseReason::Restored));
        let s = t.snapshot();
        assert_eq!(s.status, Status::Paused);
        assert_eq!(s.phase, Phase::Work);
        assert_eq!(s.remaining_ms, 10 * MIN);
        assert_eq!(s.total_ms, 25 * MIN);
        assert_eq!(s.completed_work_count, 2);
        assert!(t.tick(60_000).is_none());
        assert_eq!(t.snapshot().remaining_ms, 10 * MIN);
        t.resume(60_000);
        t.tick(65_000);
        assert_eq!(t.snapshot().remaining_ms, 10 * MIN - 5_000);
    }

    #[test]
    fn restore_paused_and_idle_keep_their_status() {
        let mut paused = default_timer();
        paused.restore_session(TimerSession {
            phase: Phase::ShortBreak,
            status: Status::Paused,
            phase_total_ms: 5 * MIN,
            remaining_ms: 90_000,
            pause_reason: Some(PauseReason::Gap),
        });
        assert_eq!(paused.snapshot().pause_reason, Some(PauseReason::Gap));
        assert_eq!(paused.snapshot().status, Status::Paused);
        assert_eq!(paused.snapshot().phase, Phase::ShortBreak);
        assert_eq!(paused.snapshot().remaining_ms, 90_000);

        let mut idle = default_timer();
        idle.restore_session(TimerSession {
            phase: Phase::LongBreak,
            status: Status::Idle,
            phase_total_ms: 15 * MIN,
            remaining_ms: 15 * MIN,
            pause_reason: None,
        });
        assert_eq!(idle.snapshot().status, Status::Idle);
        assert_eq!(idle.snapshot().remaining_ms, 15 * MIN);
    }

    #[test]
    fn phase_completed_reports_locked_total_not_live_config() {
        let mut t = default_timer();
        t.start(0);
        let mut cfg = *t.config();
        cfg.work_ms = 50 * MIN;
        t.apply_prefs(cfg);
        let ev = t.tick(25 * MIN).unwrap();
        assert_eq!(ev.from, Phase::Work);
        assert_eq!(ev.phase_total_ms, 25 * MIN);
        assert_eq!(t.work_duration_secs(), 50 * 60);
        assert_eq!(ev.stats_duration_secs(), Some(25 * 60));
        assert_eq!(t.snapshot().remaining_ms, 5 * MIN);
    }

    #[test]
    fn flush_wall_jump_pauses_without_deducting() {
        let mut t = default_timer();
        t.start(1_000);
        t.mark_wall(5_000_000);
        t.sample(1_200, 5_000_200);
        let remain = t.snapshot().remaining_ms;
        t.finish_io(5_000_200, 1_400, 5_000_200 + 60_000);
        assert_eq!(t.snapshot().status, Status::Paused);
        assert_eq!(t.snapshot().pause_reason, Some(PauseReason::Gap));
        assert_eq!(t.snapshot().remaining_ms, remain);
    }

    #[test]
    fn flush_disk_wait_is_not_counted_as_focus() {
        let mut t = default_timer();
        t.start(1_000);
        t.mark_wall(5_000_000);
        let remain = t.snapshot().remaining_ms;
        t.finish_io(5_000_000, 3_000, 5_002_000);
        assert_eq!(t.snapshot().status, Status::Running);
        assert_eq!(t.snapshot().remaining_ms, remain);
        let next = t.sample(3_250, 5_002_250);
        assert_eq!(next, Sample::Ticked(None));
        assert_eq!(t.snapshot().remaining_ms, remain - 250);
    }

    #[test]
    fn manual_pause_records_user_reason() {
        let mut t = default_timer();
        t.start(0);
        t.mark_wall(1_000);
        t.pause_at(200, 1_200);
        assert_eq!(t.snapshot().pause_reason, Some(PauseReason::User));
    }
}
