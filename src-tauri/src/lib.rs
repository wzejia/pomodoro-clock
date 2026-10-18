mod appconfig;
mod atomic_file;
mod driver;
mod geom;
mod stats;
mod timer;

use appconfig::{AppConfig, HotkeyConfig, Material, Theme};
use stats::StatsStore;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use timer::{Phase, Status, Timer, TimerSnapshot};

/// stats.json 放 exe 同目录（便携优先）；取不到则退回当前目录
fn stats_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("stats.json")
}

/// config.json 与 stats.json 同目录同策略：缺失/损坏回退默认（AppConfig::load 钳制时长并不崩）
fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.json")
}

struct AppState {
    timer: Mutex<Timer>,
    stats: Mutex<StatsStore>,
    /// 界面材质（经典/液态玻璃），与计时参数同住 config.json（AppConfig flatten）
    material: Mutex<Material>,
    /// 配置的全局快捷键（config.json 持久化）
    hotkey: Mutex<appconfig::HotkeyConfig>,
    /// 三阶段自定义配色（config.json 持久化；None=默认）
    colors: Mutex<appconfig::Colors>,
    /// 浅色/深色，与计时参数同住 config.json
    theme: Mutex<Theme>,
    /// 实际注册生效的快捷键字符串（注册失败/关闭时为 None；可能≠配置值——启动降级）
    hotkey_effective: Mutex<Option<String>>,
    /// 配置写盘串行：从改字段到整份 JSON 落盘持有，避免两次保存交错盖掉对方的字段。
    persist: Mutex<()>,
    /// 正在刷盘。tick 在此期间不采样，避免把磁盘等待算进专注。
    io_hold: AtomicBool,
    /// 原 config.json 还在且没能备份。禁止 save。
    config_blocked: AtomicBool,
    sound_enabled: AtomicBool,
}

#[derive(Clone, serde::Serialize)]
struct TickPayload {
    snapshot: TimerSnapshot,
}

/// 进程内单调毫秒。倒计时只减这个，不受校时、NTP 回拨影响。
fn mono_ms() -> u64 {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    ORIGIN.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// 墙钟只用来发现睡眠/校时跳变，不参与扣剩余时间。
fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn anchor_running(timer: &mut Timer) {
    timer.mark_wall(wall_ms());
}

fn write_config(state: &AppState) {
    if state.config_blocked.load(Ordering::SeqCst) {
        eprintln!("[config] 原文件未能备份，拒绝用默认配置覆盖");
        return;
    }
    if let Err(e) = current_appconfig(state).save(&config_path()) {
        eprintln!("[config] 保存失败: {e}");
    }
}

fn settle_after_io(state: &AppState, wall_before: u64) {
    let mut timer = match state.timer.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    timer.finish_io(wall_before, mono_ms(), wall_ms());
}

fn persist<R>(state: &AppState, mutate: impl FnOnce() -> R) -> R {
    state.io_hold.store(true, Ordering::SeqCst);
    let wall_before = wall_ms();
    let outcome = {
        let _guard = match state.persist.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let result = mutate();
        write_config(state);
        result
    };
    settle_after_io(state, wall_before);
    state.io_hold.store(false, Ordering::SeqCst);
    outcome
}

fn emit_snapshot(app: &AppHandle, snap: &TimerSnapshot) {
    let _ = app.emit("timer-tick", TickPayload { snapshot: snap.clone() });
}

fn reveal_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn main_is_visible(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

/// 阶段完成后的统一处理：落统计（仅工作番茄）、存盘、发系统通知。
/// 记入统计的时长是刚结束阶段锁定的 `phase_total_ms`，不是当前配置里的工作时长。
fn on_phase_completed(app: &AppHandle, state: &AppState, ev: timer::PhaseCompleted) {
    let cfg = {
        let timer = state.timer.lock().unwrap();
        *timer.config()
    };
    // 先把「这一轮已经结束、下一阶段是什么」落盘，再写统计。
    // 崩溃发生在两次写入之间时，最多少记一笔，不会把同一轮再完成一次。
    persist(state, || {});
    if let Some(secs) = ev.stats_duration_secs() {
        let label = state.timer.lock().unwrap().session().focus_label;
        let mut stats = state.stats.lock().unwrap();
        stats.add_labeled(chrono::Local::now().timestamp(), secs, label);
        if let Err(e) = stats.save(&stats_path()) {
            eprintln!("[stats] 保存失败: {e}");
        }
    }
    let short_min = (cfg.short_break_ms / 60_000).max(1);
    let long_min = (cfg.long_break_ms / 60_000).max(1);
    let every = cfg.long_break_every.max(1);
    let waiting = !cfg.auto_start_next;
    let (title, body) = match ev.to {
        Phase::ShortBreak if waiting => (
            "专注完成".to_string(),
            format!("休息 {short_min} 分钟还没开始"),
        ),
        Phase::ShortBreak => ("专注完成".to_string(), format!("休息 {short_min} 分钟已开始")),
        Phase::LongBreak if waiting => (
            "完成一组番茄".to_string(),
            format!("长休息 {long_min} 分钟还没开始"),
        ),
        Phase::LongBreak => (
            "完成一组番茄".to_string(),
            format!("连续 {every} 个番茄，长休息 {long_min} 分钟已开始"),
        ),
        Phase::Work if waiting => ("休息结束".to_string(), "下一段专注还没开始".to_string()),
        Phase::Work => ("休息结束".to_string(), "下一段专注已开始".to_string()),
    };
    let mut note = app.notification().builder().title(title).body(body);
    if state.sound_enabled.load(Ordering::SeqCst) {
        note = note.sound("Default");
    }
    let r = note.show();
    eprintln!("[notify] show result: {r:?}");
    // 08-18 P2-3：完成事件是前端 toast/统计刷新的唯一通知路径，emit 失败不得静默
    if let Err(e) = app.emit("phase-completed", ev) {
        eprintln!("[emit] phase-completed 发送失败: {e}");
    }
}

fn commit_timer(app: &AppHandle, state: &AppState, f: impl FnOnce(&mut Timer)) -> TimerSnapshot {
    let snap = {
        let mut t = state.timer.lock().unwrap();
        f(&mut t);
        t.snapshot()
    };
    emit_snapshot(app, &snap);
    // 开始/暂停/重置/跳过都改变可恢复状态，立刻落盘。
    persist(state, || {});
    snap
}

#[tauri::command]
fn timer_start(app: AppHandle, state: State<AppState>) -> TimerSnapshot {
    commit_timer(&app, &state, |t| {
        t.start(mono_ms());
        anchor_running(t);
    })
}

#[tauri::command]
fn timer_pause(app: AppHandle, state: State<AppState>) -> TimerSnapshot {
    commit_timer(&app, &state, |t| t.pause_at(mono_ms(), wall_ms()))
}

#[tauri::command]
fn timer_resume(app: AppHandle, state: State<AppState>) -> TimerSnapshot {
    commit_timer(&app, &state, |t| {
        t.resume(mono_ms());
        anchor_running(t);
    })
}

#[tauri::command]
fn timer_reset(app: AppHandle, state: State<AppState>) -> TimerSnapshot {
    commit_timer(&app, &state, |t| t.reset())
}

#[tauri::command]
fn timer_skip(app: AppHandle, state: State<AppState>) -> TimerSnapshot {
    commit_timer(&app, &state, |t| { let _ = t.skip(); })
}

#[tauri::command]
fn timer_snapshot(state: State<AppState>) -> TimerSnapshot {
    state.timer.lock().unwrap().snapshot()
}

fn toggle_in_place(timer: &mut Timer) {
    match timer.snapshot().status {
        Status::Running => timer.pause_at(mono_ms(), wall_ms()),
        Status::Paused => {
            timer.resume(mono_ms());
            anchor_running(timer);
        }
        Status::Idle => {
            timer.start(mono_ms());
            anchor_running(timer);
        }
    }
}

/// 窗口藏着时只唤回。窗口可见时切换计时，并立刻广播快照。
fn hotkey_pressed(app: &AppHandle) {
    if !main_is_visible(app) {
        reveal_main(app);
        return;
    }
    let state = app.state::<AppState>();
    commit_timer(app, &state, toggle_in_place);
}

#[tauri::command]
fn set_config(
    app: AppHandle,
    state: State<AppState>,
    work_min: u64,
    short_break_min: u64,
    long_break_min: u64,
    long_break_every: u32,
    auto_start_next: bool,
    daily_goal: u32,
) -> TimerSnapshot {
    let snap = persist(&state, || {
        let mut t = state.timer.lock().unwrap();
        let mut cfg = *t.config();
        cfg.work_ms = work_min.clamp(1, 120) * 60_000;
        cfg.short_break_ms = short_break_min.clamp(1, 60) * 60_000;
        cfg.long_break_ms = long_break_min.clamp(1, 60) * 60_000;
        cfg.long_break_every = long_break_every.clamp(1, 10);
        cfg.auto_start_next = auto_start_next;
        cfg.daily_goal = daily_goal.clamp(1, 20);
        t.apply_prefs(cfg);
        t.snapshot()
    });
    emit_snapshot(&app, &snap);
    snap
}

#[tauri::command]
fn get_config(state: State<AppState>) -> AppConfig {
    current_appconfig(&state)
}

/// 材质切换只动 UI 不动计时：更新内存态并落盘（不触碰 Timer）
#[tauri::command]
fn set_material(state: State<AppState>, material: Material) {
    persist(&state, || {
        *state.material.lock().unwrap() = material;
    });
}

/// 三阶段自定义配色（08-16 v16）：同 set_material 模式——只动 UI 不动计时；
/// 前端已先行校验，这里再验一道防绕过（非法 hex 不落盘不生效）
#[tauri::command]
fn set_colors(state: State<AppState>, colors: appconfig::Colors) -> Result<(), String> {
    colors.validate()?;
    persist(&state, || {
        *state.colors.lock().unwrap() = colors;
    });
    Ok(())
}

#[derive(Clone, serde::Serialize)]
struct HotkeyState {
    enabled: bool,
    shortcut: String,
    /// 实际生效键（未生效为 null），前端「当前生效键」展示用
    effective: Option<String>,
}

fn current_appconfig(state: &AppState) -> AppConfig {
    let timer = state.timer.lock().unwrap();
    AppConfig {
        version: appconfig::SCHEMA_VERSION,
        timer: *timer.config(),
        completed_work_count: timer.completed_count(),
        material: *state.material.lock().unwrap(),
        hotkey: state.hotkey.lock().unwrap().clone(),
        colors: state.colors.lock().unwrap().clone(),
        theme: *state.theme.lock().unwrap(),
        sound_enabled: state.sound_enabled.load(Ordering::SeqCst),
        session: Some(timer.session()),
    }
}

#[tauri::command]
fn set_theme(state: State<AppState>, theme: Theme) {
    persist(&state, || {
        *state.theme.lock().unwrap() = theme;
    });
}

#[tauri::command]
fn set_sound(state: State<AppState>, enabled: bool) {
    persist(&state, || {
        state.sound_enabled.store(enabled, Ordering::SeqCst);
    });
}

#[tauri::command]
fn set_focus_label(state: State<AppState>, label: String) -> TimerSnapshot {
    let mut timer = state.timer.lock().unwrap();
    timer.set_focus_label(label);
    let snap = timer.snapshot();
    drop(timer);
    persist(&state, || {});
    snap
}

#[tauri::command]
fn export_stats(state: State<AppState>) -> Result<String, String> {
    let path = stats_path().with_file_name("stats-export.json");
    let stats = state.stats.lock().unwrap();
    stats.save(&path).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

fn current_hotkey_state(state: &AppState) -> HotkeyState {
    let hk = state.hotkey.lock().unwrap().clone();
    HotkeyState { enabled: hk.enabled, shortcut: hk.shortcut, effective: state.hotkey_effective.lock().unwrap().clone() }
}

#[tauri::command]
fn get_hotkey(state: State<AppState>) -> HotkeyState {
    current_hotkey_state(&state)
}

#[tauri::command]
fn set_hotkey(
    app: AppHandle,
    state: State<AppState>,
    enabled: bool,
    shortcut: Option<String>,
) -> Result<HotkeyState, String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
    let want = shortcut.unwrap_or_else(|| state.hotkey.lock().unwrap().shortcut.clone());
    if enabled {
        HotkeyConfig::validate(&want)?; // 校验失败：配置不动、旧键不动
    }
    let _guard = state.persist.lock().unwrap();
    let gs = app.global_shortcut();
    let old = state.hotkey_effective.lock().unwrap().clone();
    if enabled && old.as_deref() == Some(want.as_str()) {
        // 同键重复录入：已生效，仅落盘
    } else if enabled {
        let sc = want.parse::<Shortcut>().map_err(|e| format!("无法识别的快捷键（{e}）"))?;
        // 先注册新键再卸旧键：新键被占用时旧键仍在役（任务书：占用回退保留旧键）
        gs.register(sc).map_err(|e| format!("快捷键被占用或不可用（{e}），已保留原键"))?;
        if let Some(o) = &old {
            if let Ok(osc) = o.parse::<Shortcut>() { let _ = gs.unregister(osc); }
        }
        *state.hotkey_effective.lock().unwrap() = Some(want.clone());
    } else {
        if let Some(o) = &old {
            if let Ok(osc) = o.parse::<Shortcut>() { let _ = gs.unregister(osc); }
        }
        *state.hotkey_effective.lock().unwrap() = None;
    }
    *state.hotkey.lock().unwrap() = HotkeyConfig { enabled, shortcut: want };
    write_config(&state);
    Ok(current_hotkey_state(&state))
}

#[tauri::command]
fn stats_summary(state: State<AppState>) -> stats::StatsSummary {
    state.stats.lock().unwrap().summary(chrono::Local::now())
}

#[tauri::command]
fn stats_month(state: State<AppState>, year: i32, month: u32) -> stats::MonthSummary {
    state.stats.lock().unwrap().month(year, month)
}

/// 开机自启（HKCU Run，tauri-plugin-autostart）：勾选态跟随真实注册状态
#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let r = if enabled { al.enable() } else { al.disable() };
    if let Err(e) = &r {
        // 失败只记日志，不弹窗不崩溃（任务书界限）
        eprintln!("[autostart] set enabled={enabled} 失败: {e}");
    }
    // 返回真实注册状态（前端勾选态跟随事实而非意图）
    al.is_enabled().unwrap_or(false)
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    let state = app.state::<AppState>();
    persist(&state, || {});
    app.exit(0);
}

#[tauri::command]
fn log_js_error(message: String) {
    eprintln!("[js-error] {message}");
}

/* ---------------- v11 托盘自绘菜单配套命令 ---------------- */

/// 托盘菜单「开始/暂停」：切换计时，并显示主窗。返回并广播新快照。
#[tauri::command]
fn timer_toggle(app: AppHandle, state: State<AppState>) -> TimerSnapshot {
    reveal_main(&app);
    commit_timer(&app, &state, toggle_in_place)
}

/// 托盘菜单关闭：hide 本窗 + 通知主窗（主面板若展开且未聚焦→整收，瞬态语义补全）
#[tauri::command]
fn tray_menu_hide(window: tauri::WebviewWindow) {
    let _ = window.hide();
    if let Some(main) = window.app_handle().get_webview_window("main") {
        let _ = main.emit("tray-menu-closed", ());
    }
}

/// 菜单项动作后焦点回主窗
#[tauri::command]
fn focus_main(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// 托盘菜单页自适配：按前端实测菜单尺寸 set 窗口（免硬编码）
#[tauri::command]
fn tray_menu_fit(window: tauri::WebviewWindow, width: f64, height: f64) {
    let _ = window.set_size(tauri::LogicalSize::new(width, height));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 单实例保护（官方文档要求注册为第一个插件）：二次启动由插件转交 argv 给既有
    // 实例的回调后自退；回调把主窗 show+focus（Windows 桌面惯例：再开=唤起既有实例，
    // 免得双开时第二份全局热键静默注册失败）。
    // 仅 release 注册（BLOCKED #14 P2 裁决，2026-08-15）：dev 与安装版共 identifier，
    // dev 也注册会被驻留安装版顶掉（tauri dev 自退而 vite 占 1430，迷惑性最强）；
    // 代价=dev 双开无保护，release 行为零变化。
    let builder = tauri::Builder::default();
    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }));
    builder
        // 08-18 依赖审计：opener 前后端零调用已移除（JS 依赖/Cargo 依赖/权限一并下线）
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        hotkey_pressed(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // 窗口用代码创建：调试构建需要附加 --remote-debugging-port（CDP 自动化验证用）
            let mut builder = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
                .title("番茄钟")
                .inner_size(220.0, 76.0)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .resizable(false)
                .shadow(false)
                // v14.1 托盘化：不进任务栏（任务栏「关闭窗口」会销毁窗体、托盘唤不回的死态根治）
                .skip_taskbar(true);
            #[cfg(debug_assertions)]
            {
                builder = builder.additional_browser_args(
                    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-port=9223",
                );
            }
            builder.build()?;

            let stats = StatsStore::load(&stats_path());
            let loaded_config = AppConfig::load_reported(&config_path());
            let appcfg = loaded_config.config;
            let mut timer = Timer::new(appcfg.timer);
            timer.restore_completed(appcfg.completed_work_count);
            // 运行中被杀掉的番茄恢复成暂停：关机期间不算专注。
            let resume_as_paused = appcfg
                .session
                .as_ref()
                .is_some_and(|s| s.status == Status::Running);
            if let Some(session) = appcfg.session {
                timer.restore_session(session);
            }
            app.manage(AppState {
                timer: Mutex::new(timer),
                stats: Mutex::new(stats),
                material: Mutex::new(appcfg.material),
                hotkey: Mutex::new(appcfg.hotkey.clone()),
                colors: Mutex::new(appcfg.colors),
                theme: Mutex::new(appcfg.theme),
                hotkey_effective: Mutex::new(None),
                persist: Mutex::new(()),
                io_hold: AtomicBool::new(false),
                config_blocked: AtomicBool::new(loaded_config.save_blocked),
                sound_enabled: AtomicBool::new(appcfg.sound_enabled),
            });
            if resume_as_paused {
                eprintln!("[timer] 上次仍在计时，已恢复为暂停；退出期间不计入专注");
                let state = app.state::<AppState>();
                persist(&state, || {});
            }

            // ---------- 托盘自绘菜单窗（v11 弃用原生 muda 菜单）：hidden 启动，右键托盘图标贴图标弹出 ----------
            let mut menu_builder = tauri::WebviewWindowBuilder::new(
                app,
                "tray-menu",
                tauri::WebviewUrl::App("tray-menu.html".into()),
            )
            .title("番茄钟菜单")
            .inner_size(168.0, 156.0)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .resizable(false)
            .shadow(false)
            .skip_taskbar(true)
            .visible(false);
            // 实测（v11 排障）：WebView2 同 user data folder 的第二窗口必须以「与主窗完全一致」的
            // browser args 创建环境，否则 ERROR_INVALID_STATE(0x8007139F) 建窗被运行时吞掉（窗口在
            // 管理器注册但原生句柄永不创建）。参数一致 → 共用浏览器进程 → 托盘页出现在 9223 目标列表。
            #[cfg(debug_assertions)]
            {
                menu_builder = menu_builder.additional_browser_args(
                    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-port=9223",
                );
            }
            menu_builder.build()?;

            // ---------- 系统托盘：左键聚焦主窗，右键弹自绘菜单窗 ----------
            {
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
                let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.ico"))
                    .or_else(|_| tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")))
                    .ok()
                    .or_else(|| app.default_window_icon().cloned())
                    .expect("托盘图标加载失败");
                TrayIconBuilder::with_id("main-tray")
                    .icon(icon)
                    .tooltip("番茄钟")
                    .show_menu_on_left_click(false)
                    .on_tray_icon_event(|tray, event| {
                        match &event {
                            TrayIconEvent::Click {
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            } => {
                                if let Some(w) = tray.app_handle().get_webview_window("main") {
                                    let _ = w.show();
                                    let _ = w.set_focus();
                                }
                            }
                            TrayIconEvent::Click {
                                button: MouseButton::Right,
                                button_state: MouseButtonState::Up,
                                rect,
                                ..
                            } => {
                                let app = tray.app_handle();
                                let Some(m) = app.get_webview_window("tray-menu") else { return };
                                if m.is_visible().unwrap_or(false) { let _ = m.hide(); return; } // 再点=关
                                let (mw, mh) = m
                                    .inner_size()
                                    .map(|s| (s.width as i32, s.height as i32))
                                    .unwrap_or((160, 124));
                                // rect：托盘图标矩形（tauri::Rect 的 position/size 是 dpi 枚举；
                                // Windows 托盘恒 Physical，scale 1.0 对 Logical 是恒等兜底）
                                let pos = rect.position.to_physical::<f64>(1.0);
                                let size = rect.size.to_physical::<f64>(1.0);
                                let icon = (
                                    pos.x as i32,
                                    pos.y as i32,
                                    (pos.x + size.width) as i32,
                                    (pos.y + size.height) as i32,
                                );
                                let work = geom::tray_work_area(
                                    app,
                                    (icon.0 + icon.2) / 2,
                                    (icon.1 + icon.3) / 2,
                                );
                                let (x, y) = geom::traymenu::menu_origin(icon, work, mw, mh);
                                let _ = m.set_position(tauri::PhysicalPosition::new(x, y));
                                let _ = m.show();
                                let _ = m.set_focus();
                                let _ = app.emit_to("tray-menu", "tray-menu-opened", ());
                            }
                            _ => {}
                        }
                    })
                    .build(app)?;
            }

            // ---------- 全局快捷键：读 config.json hotkey 配置；注册失败降级 Ctrl+Alt+O，再失败记日志不崩 ----------
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
                let state = app.state::<AppState>();
                let loaded = state.hotkey.lock().unwrap().clone();
                let resolved = loaded.clone().resolved_for_startup();
                if resolved != loaded {
                    eprintln!(
                        "[shortcut] 配置快捷键 {} 未通过校验，回退 {}",
                        loaded.shortcut, resolved.shortcut
                    );
                    let _guard = state.persist.lock().unwrap();
                    *state.hotkey.lock().unwrap() = resolved.clone();
                    write_config(&state);
                }
                let hk = resolved;
                let mut effective: Option<String> = None;
                if hk.enabled {
                    match hk.shortcut.parse::<Shortcut>() {
                        Ok(sc) => match app.global_shortcut().register(sc) {
                            Ok(()) => effective = Some(hk.shortcut.clone()),
                            Err(e) => {
                                eprintln!("[shortcut] {} 注册失败（{e}），尝试降级 Ctrl+Alt+O", hk.shortcut);
                                if hk.shortcut != "Ctrl+Alt+O" {
                                    match "Ctrl+Alt+O".parse::<Shortcut>() {
                                        Ok(fb) => match app.global_shortcut().register(fb) {
                                            Ok(()) => effective = Some("Ctrl+Alt+O".to_string()),
                                            Err(e2) => eprintln!("[shortcut] 降级 Ctrl+Alt+O 也失败（{e2}），全局快捷键不可用——记日志不弹窗不崩溃"),
                                        },
                                        Err(e2) => eprintln!("[shortcut] 降级键解析失败（{e2}）"),
                                    }
                                } else {
                                    eprintln!("[shortcut] 配置键即 Ctrl+Alt+O 被占用，全局快捷键本次不可用");
                                }
                            }
                        },
                        Err(e) => eprintln!("[shortcut] 配置快捷键无法解析（{e}），全局快捷键本次不可用"),
                    }
                }
                *state.hotkey_effective.lock().unwrap() = effective;
            }

            driver::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            timer_start,
            timer_pause,
            timer_resume,
            timer_reset,
            timer_skip,
            timer_snapshot,
            set_config,
            get_config,
            set_material,
            set_colors,
            set_theme,
            set_sound,
            set_focus_label,
            export_stats,
            get_hotkey,
            set_hotkey,
            stats_summary,
            stats_month,
            get_autostart,
            set_autostart,
            quit_app,
            log_js_error,
            timer_toggle,
            tray_menu_hide,
            focus_main,
            tray_menu_fit,
            geom::drag_begin,
            geom::drag_end,
            geom::morph_begin,
            geom::morph_commit,
        ])
        // 失焦即收（苹果瞬态面板语义）：窗口 Focused(false) → 通知前端走既有
        // setExpanded(false) 链路（含 morph 动画）；mini 态前端自行忽略。
        // 托盘自绘菜单窗失焦 → 通知托盘页自动关（真失焦：点桌面/别窗）；
        // 主窗焦点让给自家托盘菜单窗不算「失焦即收」（否则开菜单即收面板）。
        .on_window_event(|window, event| {
            // 托盘化：关闭请求（Alt+F4/任务栏关闭）一律转为隐藏——窗体保活，托盘左键可唤回；
            // 退出只走托盘菜单「退出」（app.exit 不经 CloseRequested，不受影响）
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                return;
            }
            if let tauri::WindowEvent::Focused(false) = event {
                if window.label() == "tray-menu" {
                    let _ = window.emit("tray-menu-blurred", ());
                    return;
                }
                if window.label() == "main" {
                    if let Some(m) = window.app_handle().get_webview_window("tray-menu") {
                        if m.is_visible().unwrap_or(false) {
                            return;
                        }
                    }
                }
                let _ = window.emit("window-blurred", ());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod clock_tests {
    use super::mono_ms;

    #[test]
    fn monotonic_clock_moves_forward() {
        let a = mono_ms();
        std::thread::sleep(std::time::Duration::from_millis(30));
        let b = mono_ms();
        assert!(b >= a, "单调时钟回拨：{a} -> {b}");
        assert!(b - a < 5_000, "采样间隔异常偏大：{}", b - a);
    }
}
