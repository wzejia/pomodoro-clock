//! 应用级配置：TimerConfig（计时参数）+ Material（界面材质）共用 config.json。
//! TimerConfig 保持纯净（计时引擎不管 UI）；本结构 flatten 计时字段再挂 material，
//! 因此旧版 config.json（无 material 键）加载后 material 取默认 LiquidGlass，
//! 且 set_config 保存时不会再丢掉材质键（v6.1 之前 TimerConfig::save 直写会丢）。

use crate::timer::TimerConfig;

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
    #[serde(flatten)]
    pub timer: TimerConfig,
    pub material: Material,
    pub hotkey: HotkeyConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            timer: TimerConfig::default(),
            material: Material::default(),
            hotkey: HotkeyConfig::default(),
        }
    }
}

impl AppConfig {
    /// 与 TimerConfig::load 同策略：文件缺失/损坏回默认（不崩），缺字段补默认
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
            timer: TimerConfig {
                work_ms: 30 * MIN,
                short_break_ms: 10 * MIN,
                long_break_ms: 20 * MIN,
                long_break_every: 3,
                auto_start_next: true,
            },
            material: Material::Classic,
            hotkey: HotkeyConfig::default(),
        };
        cfg.save(&path).unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded, cfg);
        std::fs::remove_dir_all(&dir).ok();
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

    /// hotkey 字段类型错误（如 "hotkey": 123）→ 与整文件损坏同策略：整体回默认不崩
    #[test]
    fn corrupt_hotkey_field_falls_back_to_default() {
        let dir = tmpdir("hotkey-corrupt");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"work_ms": 1800000, "short_break_ms": 300000, "long_break_ms": 900000,
                "long_break_every": 4, "auto_start_next": true, "hotkey": 123}"#,
        )
        .unwrap();
        let loaded = AppConfig::load(&path);
        assert_eq!(loaded, AppConfig::default());
        std::fs::remove_dir_all(&dir).ok();
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
}
