//! 应用级配置：TimerConfig（计时参数）+ Material（界面材质）共用 config.json。
//! TimerConfig 保持纯净（计时引擎不管 UI）；本结构 flatten 计时字段再挂 material，
//! 因此旧版 config.json（无 material 键）加载后 material 取默认 LiquidGlass，
//! 且 set_config 保存时不会再丢掉材质键（v6.1 之前 TimerConfig::save 直写会丢）。

use crate::timer::{TimerConfig, TimerSession};

pub const SCHEMA_VERSION: u32 = 1;

fn schema_version() -> u32 {
    SCHEMA_VERSION
}

fn sound_on() -> bool {
    true
}

/// 界面材质：经典（实心毛玻璃拟态，v6 观感）/ 液态玻璃（iOS 26 拟态，默认）
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    Classic,
    LiquidGlass,
}

impl Default for Material {
    fn default() -> Self {
        Material::LiquidGlass
    }
}

/// 三阶段自定义配色（08-16 v16）：None=跟随主题默认色；Some="#rrggbb"
/// 前端在 documentElement 行内覆盖 --work/--break/--long 生效，Rust 只管存取与校验
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Colors {
    pub work: Option<String>,
    pub short_break: Option<String>,
    pub long_break: Option<String>,
}

impl Colors {
    /// "#rrggbb" 校验（大小写不敏感；不带 # / 长度≠7 / 非十六进制一律拒绝）
    pub fn valid_hex(s: &str) -> bool {
        s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
    }

    /// 逐阶段校验：任一 Some 非法则 Err（None=恢复默认，恒合法）
    pub fn validate(&self) -> Result<(), String> {
        for (name, c) in [("work", &self.work), ("short_break", &self.short_break), ("long_break", &self.long_break)] {
            if let Some(c) = c {
                if !Self::valid_hex(c) {
                    return Err(format!("{name} 颜色须为 #rrggbb 格式"));
                }
            }
        }
        Ok(())
    }
}

/// 界面主题。旧 config.json 没有该字段时回到浅色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Light
    }
}

/// 全局快捷键配置：enabled=false 时保留 shortcut 记忆（再开恢复）
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub enabled: bool,
    /// Tauri Shortcut 字符串（如 "Ctrl+Alt+O"），必须含至少一个修饰键
    pub shortcut: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self { enabled: true, shortcut: "Ctrl+Alt+P".to_string() }
    }
}

impl HotkeyConfig {
    /// 启动时：未启用则原样保留。启用但无法通过「可解析且含修饰键」校验时，回退默认键。
    pub fn resolved_for_startup(self) -> Self {
        if self.enabled && Self::validate(&self.shortcut).is_err() {
            Self::default()
        } else {
            self
        }
    }

    /// 校验：可解析且至少一个修饰键（纯主键会拦全局输入，拒绝）
    pub fn validate(shortcut: &str) -> Result<(), String> {
        use tauri_plugin_global_shortcut::Shortcut;
        let sc = shortcut.parse::<Shortcut>().map_err(|e| format!("无法识别的快捷键（{e}）"))?;
        if sc.mods.is_empty() {
            return Err("快捷键需要至少一个修饰键（Ctrl/Alt/Shift）".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// 缺省按 v1 读。当前写入也是 1。
    #[serde(default = "schema_version")]
    pub version: u32,
    #[serde(flatten)]
    pub timer: TimerConfig,
    pub material: Material,
    pub hotkey: HotkeyConfig,
    /// 三阶段自定义配色（None=默认；serde(default) 保证旧 config.json 升级不炸）
    pub colors: Colors,
    pub theme: Theme,
    /// 本组已完成的工作番茄。今日个数在 stats.json，这是圆点和长休间隔的进度。
    pub completed_work_count: u32,
    /// 完成提示音。旧文件没有该字段时保持开启。
    #[serde(default = "sound_on")]
    pub sound_enabled: bool,
    /// 进行中的阶段。旧文件没有该字段时为 None，启动不恢复。
    #[serde(default)]
    pub session: Option<TimerSession>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            timer: TimerConfig::default(),
            material: Material::default(),
            hotkey: HotkeyConfig::default(),
            colors: Colors::default(),
            theme: Theme::default(),
            completed_work_count: 0,
            sound_enabled: true,
            session: None,
        }
    }
}

fn clamp_minutes_ms(ms: u64, min_minutes: u64, max_minutes: u64) -> u64 {
    ms.clamp(min_minutes * 60_000, max_minutes * 60_000)
}

impl AppConfig {
    fn normalize(&mut self) {
        self.timer.work_ms = clamp_minutes_ms(self.timer.work_ms, 1, 120);
        self.timer.short_break_ms = clamp_minutes_ms(self.timer.short_break_ms, 1, 60);
        self.timer.long_break_ms = clamp_minutes_ms(self.timer.long_break_ms, 1, 60);
        self.timer.long_break_every = self.timer.long_break_every.clamp(1, 10);
        self.timer.daily_goal = self.timer.daily_goal.clamp(1, 20);
    }
}

fn scrub_bad_field<T: serde::de::DeserializeOwned>(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) {
    let Some(value) = obj.get(key) else { return };
    if serde_json::from_value::<T>(value.clone()).is_err() {
        eprintln!("[config] 字段 {key} 无法解析，该字段回默认，其余保留");
        obj.remove(key);
    }
}

pub struct ConfigLoad {
    pub config: AppConfig,
    /// 原文件还在，且没能备份。调用方不得 save。
    pub save_blocked: bool,
}

impl AppConfig {
    /// 文件缺失或整份不是 JSON 时回默认。单个字段类型不对时丢掉该字段，其余保留。
    pub fn load(path: &std::path::Path) -> Self {
        Self::load_reported(path).config
    }

    /// `save_blocked` 为真时，调用方不得把内存里的默认配置写回原路径。
    pub fn load_reported(path: &std::path::Path) -> ConfigLoad {
        let text = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return ConfigLoad { config: Self::default(), save_blocked: false };
            }
            Err(e) => {
                eprintln!("[config] 读取失败（{e}）");
                return Self::quarantine_whole(path);
            }
        };
        if let Ok(mut cfg) = serde_json::from_str::<Self>(&text) {
            cfg.normalize();
            return ConfigLoad { config: cfg, save_blocked: false };
        }
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&text) else {
            eprintln!("[config] 应用配置不是 JSON");
            return Self::quarantine_whole(path);
        };
        let Some(obj) = value.as_object_mut() else {
            eprintln!("[config] 应用配置不是对象");
            return Self::quarantine_whole(path);
        };
        scrub_bad_field::<u64>(obj, "work_ms");
        scrub_bad_field::<u64>(obj, "short_break_ms");
        scrub_bad_field::<u64>(obj, "long_break_ms");
        scrub_bad_field::<u32>(obj, "long_break_every");
        scrub_bad_field::<bool>(obj, "auto_start_next");
        scrub_bad_field::<u32>(obj, "daily_goal");
        scrub_bad_field::<Material>(obj, "material");
        scrub_bad_field::<HotkeyConfig>(obj, "hotkey");
        scrub_bad_field::<Colors>(obj, "colors");
        scrub_bad_field::<Theme>(obj, "theme");
        scrub_bad_field::<u32>(obj, "completed_work_count");
        scrub_bad_field::<bool>(obj, "sound_enabled");
        scrub_bad_field::<u32>(obj, "version");
        scrub_bad_field::<TimerSession>(obj, "session");
        match serde_json::from_value::<Self>(value) {
            Ok(mut cfg) => {
                cfg.normalize();
                ConfigLoad { config: cfg, save_blocked: false }
            }
            Err(e) => {
                eprintln!("[config] 应用配置解析失败（{e}）");
                Self::quarantine_whole(path)
            }
        }
    }

    fn quarantine_whole(path: &std::path::Path) -> ConfigLoad {
        match crate::atomic_file::quarantine_corrupt(path) {
            Ok(dest) => {
                eprintln!("[config] 已备份到 {}，不会用默认配置覆盖原文件", dest.display());
                ConfigLoad { config: Self::default(), save_blocked: false }
            }
            Err(e) => {
                eprintln!("[config] 备份失败（{e}）。拒绝用默认配置覆盖原文件");
                ConfigLoad { config: Self::default(), save_blocked: true }
            }
        }
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        // 08-18 P2：序列化失败转 io::Error 走 ?，不在 save 路径 panic
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        crate::atomic_file::write_atomic(path, json.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    fn tmpdir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pomodoro-appcfg-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn appconfig_roundtrip_keeps_material_and_timer() {
        let dir = tmpdir("roundtrip");
        let path = dir.join("config.json");
        let cfg = AppConfig {
            version: SCHEMA_VERSION,
            timer: TimerConfig {
                work_ms: 30 * MIN,
                short_break_ms: 10 * MIN,
                long_break_ms: 20 * MIN,
                long_break_every: 3,
                auto_start_next: true,
                daily_goal: 10,
            },
            material: Material::Classic,
            hotkey: HotkeyConfig::default(),
            colors: Colors {
                work: Some("#ff9500".to_string()),
                short_break: Some("#30d158".to_string()),
                long_break: Some("#64d2ff".to_string()),
            },
            theme: Theme::Dark,
            completed_work_count: 2,
            sound_enabled: true,
            session: Some(TimerSession {
                phase: crate::timer::Phase::Work,
                status: crate::timer::Status::Paused,
                phase_total_ms: 30 * MIN,
                remaining_ms: 12 * MIN,
                pause_reason: None,
                focus_label: String::new(),
            }),
        };
        cfg.save(&path).unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded, cfg);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 08-16 v16：旧 config.json（无 colors 键）→ 三阶段全 None（跟默认）
    #[test]
    fn legacy_config_without_colors_defaults_to_none() {
        let dir = tmpdir("legacy-colors");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"work_ms": 1800000, "short_break_ms": 300000, "long_break_ms": 900000,
                "long_break_every": 4, "auto_start_next": false, "material": "classic"}"#,
        )
        .unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded.colors, Colors::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 部分自定义（只改专注色）roundtrip：None 阶段保持 None
    #[test]
    fn partial_colors_roundtrip() {
        let dir = tmpdir("partial-colors");
        let path = dir.join("config.json");
        let cfg = AppConfig {
            colors: Colors { work: Some("#af52de".to_string()), ..Colors::default() },
            ..AppConfig::default()
        };
        cfg.save(&path).unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded.colors.work.as_deref(), Some("#af52de"));
        assert_eq!(loaded.colors.short_break, None);
        assert_eq!(loaded.colors.long_break, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// valid_hex / validate 正反例（不带 #、短、非十六进制、大写合法）
    #[test]
    fn colors_validate_rules() {
        assert!(Colors::valid_hex("#ff3b30"));
        assert!(Colors::valid_hex("#FF3B30"));
        assert!(!Colors::valid_hex("ff3b30")); // 缺 #
        assert!(!Colors::valid_hex("#abc"));   // 短
        assert!(!Colors::valid_hex("#GGGGGG")); // 非十六进制
        assert!(!Colors::valid_hex("#ff3b300")); // 长
        // validate：全 None / 合法 Some 通过，任一非法 Some 拒绝
        assert!(Colors::default().validate().is_ok());
        assert!(Colors { work: Some("#123abc".into()), ..Colors::default() }.validate().is_ok());
        assert!(Colors { long_break: Some("red".into()), ..Colors::default() }.validate().is_err());
    }

    /// 旧版 config.json（v6.1 前，无 material 键）→ 材质回默认液态玻璃，计时字段原样
    #[test]
    fn legacy_config_without_material_defaults_to_liquid_glass() {
        let dir = tmpdir("legacy");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"work_ms": 1800000, "short_break_ms": 300000, "long_break_ms": 900000,
                "long_break_every": 4, "auto_start_next": false}"#,
        )
        .unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded.material, Material::LiquidGlass);
        assert_eq!(loaded.timer.work_ms, 30 * MIN);
        assert_eq!(loaded.timer.long_break_every, 4);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_config_returns_default_with_liquid_glass() {
        let dir = tmpdir("corrupt");
        let path = dir.join("config.json");
        std::fs::write(&path, "{ not valid json !!!").unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded, AppConfig::default());
        assert_eq!(loaded.material, Material::LiquidGlass);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn material_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&Material::LiquidGlass).unwrap(),
            "\"liquid_glass\""
        );
        assert_eq!(
            serde_json::to_string(&Material::Classic).unwrap(),
            "\"classic\""
        );
    }

    /// v11：旧版 config.json（无 hotkey 键）→ 快捷键回默认（Ctrl+Alt+P 启用）
    #[test]
    fn legacy_config_without_hotkey_gets_default() {
        let dir = tmpdir("legacy-hotkey");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"work_ms": 1800000, "short_break_ms": 300000, "long_break_ms": 900000,
                "long_break_every": 4, "auto_start_next": false, "material": "classic"}"#,
        )
        .unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded.hotkey, HotkeyConfig::default());
        assert_eq!(loaded.material, Material::Classic);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hotkey_roundtrip() {
        let dir = tmpdir("hotkey-roundtrip");
        let path = dir.join("config.json");
        let cfg = AppConfig {
            hotkey: HotkeyConfig { enabled: false, shortcut: "Ctrl+Shift+F5".to_string() },
            ..AppConfig::default()
        };
        cfg.save(&path).unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded, cfg);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// hotkey 字段类型错误时只丢掉热键，时长和自动开始仍保留
    #[test]
    fn corrupt_hotkey_field_keeps_timer() {
        let dir = tmpdir("hotkey-corrupt");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"work_ms": 1800000, "short_break_ms": 300000, "long_break_ms": 900000,
                "long_break_every": 4, "auto_start_next": true, "hotkey": 123}"#,
        )
        .unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded.timer.work_ms, 30 * MIN);
        assert!(loaded.timer.auto_start_next);
        assert_eq!(loaded.hotkey, HotkeyConfig::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn startup_hotkey_without_modifier_falls_back_to_default() {
        let bad = HotkeyConfig { enabled: true, shortcut: "P".into() };
        assert_eq!(bad.resolved_for_startup(), HotkeyConfig::default());
        let off = HotkeyConfig { enabled: false, shortcut: "P".into() };
        assert_eq!(off.clone().resolved_for_startup(), off);
        let ok = HotkeyConfig { enabled: true, shortcut: "Ctrl+Shift+F5".into() };
        assert_eq!(ok.clone().resolved_for_startup(), ok);
    }

    #[test]
    fn hotkey_validate_rules() {
        assert!(HotkeyConfig::validate("Ctrl+Alt+P").is_ok());
        assert!(HotkeyConfig::validate("Ctrl+Shift+F5").is_ok());
        assert!(HotkeyConfig::validate("P").is_err()); // 无修饰键
        assert!(HotkeyConfig::validate("Ctrl+Alt").is_err()); // 无主键
        assert!(HotkeyConfig::validate("").is_err());
        assert!(HotkeyConfig::validate("Ctrl+Alt+P+X").is_err()); // 双主键
    }

    #[test]
    fn load_clamps_every_goal_and_fills_theme() {
        let dir = tmpdir("clamp");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"work_ms":0,"short_break_ms":300000,"long_break_ms":900000,"long_break_every":0,"auto_start_next":false,"daily_goal":40}"#,
        )
        .unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded.timer.work_ms, 60_000);
        assert_eq!(loaded.timer.long_break_every, 1);
        assert_eq!(loaded.timer.daily_goal, 20);
        assert_eq!(loaded.theme, Theme::Light);
        assert_eq!(loaded.completed_work_count, 0);
        assert_eq!(loaded.version, SCHEMA_VERSION);
        assert_eq!(loaded.session, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn legacy_config_without_version_reads_as_v1() {
        let dir = tmpdir("legacy-version");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"work_ms": 1800000, "short_break_ms": 300000, "long_break_ms": 900000,
                "long_break_every": 4, "auto_start_next": false}"#,
        )
        .unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded.version, SCHEMA_VERSION);
        assert_eq!(loaded.session, None);
        assert_eq!(loaded.timer.work_ms, 30 * MIN);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_session_field_keeps_the_rest() {
        let dir = tmpdir("bad-session");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"version":1,"work_ms":1800000,"short_break_ms":300000,"long_break_ms":900000,"long_break_every":4,"auto_start_next":true,"session":"nope"}"#,
        )
        .unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded.timer.work_ms, 30 * MIN);
        assert!(loaded.timer.auto_start_next);
        assert_eq!(loaded.session, None);
        assert_eq!(loaded.version, SCHEMA_VERSION);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn whole_config_corrupt_is_backed_up_not_overwritten() {
        let dir = tmpdir("whole-corrupt");
        let path = dir.join("config.json");
        let original = "{ not valid json !!!";
        std::fs::write(&path, original).unwrap();
        let loaded = AppConfig::load_reported(&path);
        assert_eq!(loaded.config.material, Material::LiquidGlass);
        assert!(!loaded.save_blocked);
        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".corrupt"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(std::fs::read_to_string(backups[0].path()).unwrap(), original);
        loaded.config.save(&path).unwrap();
        assert_eq!(std::fs::read_to_string(backups[0].path()).unwrap(), original);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn config_save_blocks_when_backup_fails() {
        let dir = tmpdir("cfg-nobackup");
        let path = dir.join("config.json");
        let original = b"{ not valid json !!!";
        std::fs::write(&path, original).unwrap();
        let loaded = {
            let _guard = crate::atomic_file::block_quarantine(&path);
            AppConfig::load_reported(&path)
        };
        assert!(loaded.save_blocked, "备份失败时必须禁止覆盖");
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_dir_all(&dir).ok();
    }
}
