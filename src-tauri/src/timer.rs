//! 番茄钟计时状态机 —— 唯一计时真相来源。
//! 所有时间均为毫秒；tick(now_ms) 由调用方（Tauri 端定时器）驱动，
//! 测试时可直接注入虚拟时间，保证暂停/继续零失真。

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
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self).unwrap())?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

/// tick 产生的事件：阶段完成，自动切换到下一阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct PhaseCompleted {
    pub from: Phase,
    pub to: Phase,
    /// 若 from == Work，这是本轮完成的工作番茄序号（1 起）
    pub completed_work_count: u32,
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
}

pub struct Timer {
    config: TimerConfig,
    phase: Phase,
    status: Status,
    remaining_ms: u64,
    /// Running 时最后一次 tick 的时间戳
    last_tick_ms: u64,
    completed_work_count: u32,
}

impl Timer {
    pub fn new(config: TimerConfig) -> Self {
        Self {
            phase: Phase::Work,
            status: Status::Idle,
            remaining_ms: config.work_ms,
            last_tick_ms: 0,
            completed_work_count: 0,
            config,
        }
    }

    pub fn config(&self) -> &TimerConfig {
        &self.config
    }

    pub fn work_duration_secs(&self) -> u64 {
        self.config.work_ms / 1000
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
            total_ms: self.phase_duration(self.phase),
            completed_work_count: self.completed_work_count,
            long_break_every: self.config.long_break_every,
            daily_goal: self.config.daily_goal,
            work_ms: self.config.work_ms,
        }
    }

    /// 开始（Idle → Running）；运行中调用等价于重置后重新开始
    pub fn start(&mut self, now_ms: u64) {
        self.remaining_ms = self.phase_duration(self.phase);
        self.status = Status::Running;
        self.last_tick_ms = now_ms;
    }

    pub fn pause(&mut self, now_ms: u64) {
        if self.status != Status::Running {
            return;
        }
        self.accumulate(now_ms);
        self.status = Status::Paused;
    }

    pub fn resume(&mut self, now_ms: u64) {
        if self.status != Status::Paused {
            return;
        }
        self.status = Status::Running;
        self.last_tick_ms = now_ms;
    }

    /// 重置：回到当前阶段初始、Idle
    pub fn reset(&mut self) {
        self.remaining_ms = self.phase_duration(self.phase);
        self.status = Status::Idle;
    }

    /// 跳过当前阶段（不计入完成数），进入下一阶段并停表
    pub fn skip(&mut self) -> PhaseCompleted {
        let from = self.phase;
        let to = self.next_phase_after(from, false);
        self.phase = to;
        self.remaining_ms = self.phase_duration(to);
        self.status = Status::Idle;
        PhaseCompleted {
            from,
            to,
            completed_work_count: self.completed_work_count,
        }
    }

    pub fn set_config(&mut self, config: TimerConfig) {
        self.config = config;
        // 配置变更后当前阶段剩余时间按新时长重置（停表）
        self.remaining_ms = self.phase_duration(self.phase);
        self.status = Status::Idle;
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
        if from == Phase::Work {
            self.completed_work_count += 1;
        }
        let to = self.next_phase_after(from, true);
        self.phase = to;
        self.remaining_ms = self.phase_duration(to);
        // auto_start_next=true 时自动开跑下一阶段；否则停表等手动开始
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
                if completed && self.completed_work_count % self.config.long_break_every == 0 {
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
    fn set_config_resets_current_phase() {
        let mut t = default_timer();
        t.start(0);
        t.tick(5_000);
        let mut cfg = TimerConfig::default();
        cfg.work_ms = 50 * MIN;
        t.set_config(cfg);
        let s = t.snapshot();
        assert_eq!(s.status, Status::Idle);
        assert_eq!(s.remaining_ms, 50 * MIN);
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
}
