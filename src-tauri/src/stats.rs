//! 番茄统计：记录、聚合、JSON 持久化。
//! 存储为纯 JSON 数组，便携优先；聚合逻辑全部在 Rust 侧，便于测试。

use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroRecord {
    /// 完成时刻（epoch 秒）
    pub completed_at: i64,
    /// 本次专注时长（秒）
    pub duration_secs: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsStore {
    pub records: Vec<PomodoroRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayBar {
    /// YYYY-MM-DD
    pub date: String,
    pub count: u32,
    pub minutes: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsSummary {
    pub today_count: u32,
    pub today_minutes: u32,
    /// 本周（周一到周日）每日柱状
    pub week: Vec<DayBar>,
    /// 连续打卡天数（今天没打则算到昨天为止）
    pub streak_days: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonthSummary {
    pub year: i32,
    pub month: u32,
    /// 当月 1 号的星期偏移（0=周一 … 6=周日），前端月历首行缩进用
    pub first_weekday: u32,
    /// 当月每日（1 号起按序）
    pub days: Vec<DayBar>,
}

impl StatsStore {
    /// 从文件加载；文件不存在或损坏时返回空库（不崩）
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let tmp: PathBuf = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self).unwrap())?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn add_record(&mut self, completed_at: i64, duration_secs: u64) {
        self.records.push(PomodoroRecord {
            completed_at,
            duration_secs,
        });
    }

    /// now: 任意时刻（通常 Local::now()）；测试可注入
    pub fn summary(&self, now: chrono::DateTime<Local>) -> StatsSummary {
        let today = now.date_naive();
        let mut today_count = 0u32;
        let mut today_secs = 0u64;

        // 本周周一
        let weekday = today.weekday().num_days_from_monday();
        let monday = today - Duration::days(weekday as i64);
        let mut week: Vec<DayBar> = (0..7)
            .map(|i| {
                let d = monday + Duration::days(i);
                DayBar {
                    date: d.format("%Y-%m-%d").to_string(),
                    count: 0,
                    minutes: 0,
                }
            })
            .collect();

        let mut days_with: std::collections::HashSet<NaiveDate> = std::collections::HashSet::new();

        for r in &self.records {
            let dt = chrono::DateTime::from_timestamp(r.completed_at, 0)
                .map(|u| u.with_timezone(&Local));
            let Some(dt) = dt else { continue };
            let d = dt.date_naive();
            days_with.insert(d);
            if d == today {
                today_count += 1;
                today_secs += r.duration_secs;
            }
            if d >= monday && d <= today {
                let idx = (d - monday).num_days() as usize;
                if idx < 7 {
                    week[idx].count += 1;
                    week[idx].minutes += (r.duration_secs / 60) as u32;
                }
            }
        }

        // 连续打卡：从今天（若有）否则从昨天往前数
        let mut streak = 0u32;
        let mut cursor = if days_with.contains(&today) {
            today
        } else {
            today - Duration::days(1)
        };
        while days_with.contains(&cursor) {
            streak += 1;
            cursor = cursor - Duration::days(1);
        }

        StatsSummary {
            today_count,
            today_minutes: (today_secs / 60) as u32,
            week,
            streak_days: streak,
        }
    }

    /// 月聚合：入参年月，返回当月每日 DayBar + 月初星期偏移（0=周一）
    pub fn month(&self, year: i32, month: u32) -> MonthSummary {
        // 非法月份钳到 1-12（命令层不设防，调用方越界也不 panic）
        let month = month.clamp(1, 12);
        let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let next = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
        };
        let n_days = (next - first).num_days() as i64;
        let mut days: Vec<DayBar> = (0..n_days)
            .map(|i| DayBar {
                date: (first + Duration::days(i)).format("%Y-%m-%d").to_string(),
                count: 0,
                minutes: 0,
            })
            .collect();

        for r in &self.records {
            let dt = chrono::DateTime::from_timestamp(r.completed_at, 0)
                .map(|u| u.with_timezone(&Local));
            let Some(dt) = dt else { continue };
            let d = dt.date_naive();
            if d.year() == year && d.month() == month {
                let idx = (d.day() - 1) as usize;
                days[idx].count += 1;
                days[idx].minutes += (r.duration_secs / 60) as u32;
            }
        }

        MonthSummary {
            year,
            month,
            first_weekday: first.weekday().num_days_from_monday(),
            days,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        Local
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .single()
            .unwrap()
            .timestamp()
    }

    #[test]
    fn empty_stats_summary_is_zero() {
        let s = StatsStore::default();
        let now = Local::now();
        let sum = s.summary(now);
        assert_eq!(sum.today_count, 0);
        assert_eq!(sum.today_minutes, 0);
        assert_eq!(sum.streak_days, 0);
        assert_eq!(sum.week.len(), 7);
        assert!(sum.week.iter().all(|b| b.count == 0));
    }

    #[test]
    fn today_records_aggregate_count_and_minutes() {
        let mut s = StatsStore::default();
        // 用「今天」动态构造，避免测试过期
        let now = Local::now();
        let today_9am = now.date_naive().and_hms_opt(9, 0, 0).unwrap();
        let ts = Local.from_local_datetime(&today_9am).single().unwrap().timestamp();
        s.add_record(ts, 25 * 60);
        s.add_record(ts + 3600, 25 * 60);
        let sum = s.summary(now);
        assert_eq!(sum.today_count, 2);
        assert_eq!(sum.today_minutes, 50);
    }

    #[test]
    fn cross_day_records_split_correctly() {
        let mut s = StatsStore::default();
        let now = Local::now();
        let today = now.date_naive();
        let yesterday = today - Duration::days(1);
        let t_y = Local.from_local_datetime(&yesterday.and_hms_opt(10, 0, 0).unwrap()).single().unwrap().timestamp();
        let t_t = Local.from_local_datetime(&today.and_hms_opt(10, 0, 0).unwrap()).single().unwrap().timestamp();
        s.add_record(t_y, 25 * 60);
        s.add_record(t_t, 25 * 60);
        s.add_record(t_t + 60, 25 * 60);
        let sum = s.summary(now);
        assert_eq!(sum.today_count, 2);
        // 周视图里两天各在其位
        let total_week: u32 = sum.week.iter().map(|b| b.count).sum();
        assert_eq!(total_week, 3);
    }

    #[test]
    fn streak_counts_consecutive_days() {
        let mut s = StatsStore::default();
        let now = Local::now();
        let today = now.date_naive();
        for back in [0i64, 1, 2] {
            let d = today - Duration::days(back);
            let ts = Local.from_local_datetime(&d.and_hms_opt(12, 0, 0).unwrap()).single().unwrap().timestamp();
            s.add_record(ts, 25 * 60);
        }
        let sum = s.summary(now);
        assert_eq!(sum.streak_days, 3);
    }

    #[test]
    fn streak_breaks_on_gap_day() {
        let mut s = StatsStore::default();
        let now = Local::now();
        let today = now.date_naive();
        // 昨天、前天有，大前天断；今天没打
        for back in [1i64, 2, 4] {
            let d = today - Duration::days(back);
            let ts = Local.from_local_datetime(&d.and_hms_opt(12, 0, 0).unwrap()).single().unwrap().timestamp();
            s.add_record(ts, 25 * 60);
        }
        let sum = s.summary(now);
        assert_eq!(sum.streak_days, 2);
    }

    #[test]
    fn streak_zero_when_only_old_records() {
        let mut s = StatsStore::default();
        s.add_record(at(2020, 1, 1, 12, 0), 25 * 60);
        let sum = s.summary(Local::now());
        assert_eq!(sum.streak_days, 0);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("pomodoro-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.json");
        let mut s = StatsStore::default();
        s.add_record(1_700_000_000, 1500);
        s.add_record(1_700_003_600, 1500);
        s.save(&path).unwrap();
        let loaded = StatsStore::load(&path);
        assert_eq!(loaded.records.len(), 2);
        assert_eq!(loaded.records[0].completed_at, 1_700_000_000);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let path = std::env::temp_dir().join("pomodoro-definitely-not-exist-xyz.json");
        let s = StatsStore::load(&path);
        assert!(s.records.is_empty());
    }

    #[test]
    fn load_corrupt_file_returns_empty() {
        let dir = std::env::temp_dir().join(format!("pomodoro-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.json");
        std::fs::write(&path, "{ not valid json !!!").unwrap();
        let s = StatsStore::load(&path);
        assert!(s.records.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn week_bars_align_to_monday() {
        let s = StatsStore::default();
        // 2026-08-12 是周三；周一应为 2026-08-10
        let now = Local.with_ymd_and_hms(2026, 8, 12, 15, 0, 0).single().unwrap();
        let sum = s.summary(now);
        assert_eq!(sum.week[0].date, "2026-08-10");
        assert_eq!(sum.week[6].date, "2026-08-16");
    }

    #[test]
    fn month_aggregates_count_and_minutes() {
        let mut s = StatsStore::default();
        // 2026-08-05 两个 25 分钟番茄 + 2026-08-20 一个 50 分钟
        s.add_record(at(2026, 8, 5, 9, 0), 25 * 60);
        s.add_record(at(2026, 8, 5, 14, 30), 25 * 60);
        s.add_record(at(2026, 8, 20, 10, 0), 50 * 60);
        let m = s.month(2026, 8);
        assert_eq!(m.days.len(), 31);
        assert_eq!(m.days[4].date, "2026-08-05");
        assert_eq!(m.days[4].count, 2);
        assert_eq!(m.days[4].minutes, 50);
        assert_eq!(m.days[19].count, 1);
        assert_eq!(m.days[19].minutes, 50);
        // 其余日为零
        let total: u32 = m.days.iter().map(|b| b.count).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn cross_month_records_do_not_leak() {
        let mut s = StatsStore::default();
        s.add_record(at(2026, 7, 31, 23, 10), 25 * 60);
        s.add_record(at(2026, 8, 1, 0, 40), 25 * 60);
        let jul = s.month(2026, 7);
        let aug = s.month(2026, 8);
        assert_eq!(jul.days.len(), 31);
        assert_eq!(jul.days[30].count, 1);
        assert_eq!(jul.days.iter().map(|b| b.count).sum::<u32>(), 1);
        assert_eq!(aug.days[0].count, 1);
        assert_eq!(aug.days.iter().map(|b| b.count).sum::<u32>(), 1);
    }

    #[test]
    fn empty_month_is_all_zero() {
        let s = StatsStore::default();
        let m = s.month(2020, 1);
        assert_eq!(m.days.len(), 31);
        assert!(m.days.iter().all(|b| b.count == 0 && b.minutes == 0));
    }

    #[test]
    fn month_first_weekday_offset() {
        let s = StatsStore::default();
        // 由既有测试锚定 2026-08-10 是周一 → 2026-08-01 是周六（周一=0 → 5）
        assert_eq!(s.month(2026, 8).first_weekday, 5);
        // 2026-06-01 是周一（6-10 周一逆推：6-1 周一）
        assert_eq!(s.month(2026, 6).first_weekday, 0);
    }

    #[test]
    fn month_days_len_matches_calendar() {
        let s = StatsStore::default();
        assert_eq!(s.month(2026, 2).days.len(), 28); // 2026 非闰年
        assert_eq!(s.month(2024, 2).days.len(), 29); // 2024 闰年
        assert_eq!(s.month(2026, 4).days.len(), 30);
        // 越界月份钳制不 panic
        assert_eq!(s.month(2026, 13).days.len(), 31); // 钳到 12 月
    }
}
