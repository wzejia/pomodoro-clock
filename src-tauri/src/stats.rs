//! 番茄统计：记录、聚合、JSON 持久化。
//! 文件是带 `version` 的对象（旧文件没有 version，按 v1 读）。聚合逻辑全部在 Rust 侧。

use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroRecord {
    /// 完成时刻（epoch 秒）
    pub completed_at: i64,
    /// 本次专注时长（秒）
    pub duration_secs: u64,
    /// 这一轮「正在做」的标题。旧记录没有该字段。
    #[serde(default)]
    pub focus_label: String,
}

fn schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsStore {
    /// 缺省按 v1 读。当前写入也是 1。
    #[serde(default = "schema_version")]
    pub version: u32,
    pub records: Vec<PomodoroRecord>,
    /// 解析失败且备份也失败时为 true，`save` 拒绝覆盖原文件。
    #[serde(skip, default)]
    quarantine_failed: bool,
}

impl Default for StatsStore {
    fn default() -> Self {
        Self {
            version: schema_version(),
            records: Vec::new(),
            quarantine_failed: false,
        }
    }
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
    /// 文件不存在时返回空库。解析失败时把原文件备份成带时间戳的 `.corrupt`，
    /// 内存用空库，且不会在这次加载里用空库覆盖原文件。备份失败则拒绝之后的 save。
    pub fn load(path: &Path) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                eprintln!("[stats] 读取失败（{e}），尝试把原文件备份后再放弃");
                return Self::after_failed_read(path);
            }
        };
        match serde_json::from_str::<Self>(&text) {
            Ok(store) => store,
            Err(e) => {
                eprintln!("[stats] 解析失败（{e}）");
                Self::after_failed_read(path)
            }
        }
    }

    /// 读不出或解析不了时，先把原字节挪走。挪不走就拒绝以后的 save。
    fn after_failed_read(path: &Path) -> Self {
        match crate::atomic_file::quarantine_corrupt(path) {
            Ok(dest) => {
                eprintln!(
                    "[stats] 已备份到 {}，不会用空库覆盖原历史",
                    dest.display()
                );
                Self::default()
            }
            Err(be) => {
                eprintln!("[stats] 备份失败（{be}）。拒绝之后用空库覆盖原文件");
                Self {
                    quarantine_failed: true,
                    ..Self::default()
                }
            }
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if self.quarantine_failed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "stats.json 损坏且备份失败，拒绝用空库覆盖",
            ));
        }
        // 08-18 P2：序列化失败转 io::Error 走 ?，不在 save 路径 panic
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        crate::atomic_file::write_atomic(path, json.as_bytes())
    }

    pub fn add_record(&mut self, completed_at: i64, duration_secs: u64) {
        self.add_labeled(completed_at, duration_secs, "");
    }

    pub fn add_labeled(&mut self, completed_at: i64, duration_secs: u64, focus_label: impl Into<String>) {
        self.records.push(PomodoroRecord {
            completed_at,
            duration_secs,
            focus_label: focus_label.into(),
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
        // 08-18 P2-1：年份钳 chrono 有效域（≤262143 且给 year+1 留余量），
        // from_ymd_opt 才恒 Some，杜绝 unwrap panic（前端传 0/负数/极端年也不炸）
        let year = year.clamp(1, 262_142);
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
    use chrono::{Datelike, TimeZone};

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
        let today_idx = today.weekday().num_days_from_monday() as usize;
        assert_eq!(sum.week[today_idx].count, 2);
        let total_week: u32 = sum.week.iter().map(|b| b.count).sum();
        if today_idx == 0 {
            assert_eq!(total_week, 2);
        } else {
            assert_eq!(sum.week[today_idx - 1].count, 1);
            assert_eq!(total_week, 3);
        }
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
        s.add_labeled(1_700_003_600, 1500, "写方案");
        s.save(&path).unwrap();
        let loaded = StatsStore::load(&path);
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.records.len(), 2);
        assert_eq!(loaded.records[0].completed_at, 1_700_000_000);
        assert_eq!(loaded.records[0].focus_label, "");
        assert_eq!(loaded.records[1].focus_label, "写方案");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"version\": 1"));
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_without_version_reads_as_v1() {
        let dir = std::env::temp_dir().join(format!("pomodoro-nover-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.json");
        std::fs::write(
            &path,
            r#"{"records":[{"completed_at":1700000000,"duration_secs":1500}]}"#,
        )
        .unwrap();
        let loaded = StatsStore::load(&path);
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.records[0].duration_secs, 1500);
        assert!(std::fs::read_to_string(&path).unwrap().contains("records"));
        assert!(
            std::fs::read_dir(&dir)
                .unwrap()
                .all(|e| !e.unwrap().file_name().to_string_lossy().contains("corrupt"))
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let path = std::env::temp_dir().join("pomodoro-definitely-not-exist-xyz.json");
        let s = StatsStore::load(&path);
        assert!(s.records.is_empty());
    }

    #[test]
    fn load_corrupt_file_is_backed_up_and_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("pomodoro-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.json");
        let original = "{ not valid json !!!";
        std::fs::write(&path, original).unwrap();
        let s = StatsStore::load(&path);
        assert!(s.records.is_empty());
        assert_eq!(s.version, 1);
        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".corrupt"))
            .collect();
        assert_eq!(backups.len(), 1, "损坏文件必须留一份带时间戳的备份");
        assert_eq!(std::fs::read_to_string(backups[0].path()).unwrap(), original);
        // 原路径上不再是那份损坏内容；空库 save 只写新文件。
        s.save(&path).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("\"records\""));
        assert!(!saved.contains("not valid"));
        assert_eq!(std::fs::read_to_string(backups[0].path()).unwrap(), original);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unreadable_bytes_are_backed_up_before_empty_save() {
        let dir = std::env::temp_dir().join(format!("pomodoro-bin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.json");
        let original = vec![0xFFu8, 0xFE, 0x00, 0x41];
        std::fs::write(&path, &original).unwrap();
        let s = StatsStore::load(&path);
        assert!(s.records.is_empty());
        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".corrupt"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(std::fs::read(backups[0].path()).unwrap(), original);
        s.save(&path).unwrap();
        assert_eq!(std::fs::read(backups[0].path()).unwrap(), original);
        assert!(std::fs::read_to_string(&path).unwrap().contains("records"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_refuses_when_backup_failed_and_keeps_bytes() {
        let dir = std::env::temp_dir().join(format!("pomodoro-nobackup-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.json");
        let original = b"{ not valid json !!!";
        std::fs::write(&path, original).unwrap();
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&dir, perms).unwrap();
        let loaded = StatsStore::load(&path);
        let save_result = loaded.save(&path);
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(&dir, perms).unwrap();
        assert!(save_result.is_err(), "备份失败后必须拒绝保存");
        assert_eq!(std::fs::read(&path).unwrap(), original);
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
