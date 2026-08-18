mod appconfig;
mod stats;
mod timer;

use appconfig::{AppConfig, HotkeyConfig, Material};
use stats::StatsStore;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use timer::{Phase, Status, Timer, TimerSnapshot};

/// stats.json 放 exe 同目录（便携优先）；取不到则退回当前目录
fn stats_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("stats.json")
}

/// config.json 与 stats.json 同目录同策略：缺失/损坏回退默认（TimerConfig::load 保证不崩）
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
    /// 实际注册生效的快捷键字符串（注册失败/关闭时为 None；可能≠配置值——启动降级）
    hotkey_effective: Mutex<Option<String>>,
}

#[derive(Clone, serde::Serialize)]
struct TickPayload {
    snapshot: TimerSnapshot,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 阶段完成后的统一处理：落统计（仅工作番茄）、存盘、发系统通知
fn on_phase_completed(app: &AppHandle, state: &AppState, ev: timer::PhaseCompleted) {
    if ev.from == Phase::Work {
        let mut stats = state.stats.lock().unwrap();
        stats.add_record(
            chrono::Local::now().timestamp(),
            state.timer.lock().unwrap().work_duration_secs(),
        );
        let _ = stats.save(&stats_path());
    }
    let (title, body) = match ev.to {
        Phase::ShortBreak => ("专注完成 🍅", "辛苦了，休息 5 分钟吧"),
        Phase::LongBreak => ("完成一组番茄 🎉", "连续 4 个番茄，长休息 15 分钟"),
        Phase::Work => ("休息结束", "准备好开始下一段专注了吗"),
    };
    let r = app.notification().builder().title(title).body(body).show();
    eprintln!("[notify] show result: {r:?}");
    // 08-18 P2-3：完成事件是前端 toast/统计刷新的唯一通知路径，emit 失败不得静默
    if let Err(e) = app.emit("phase-completed", ev) {
        eprintln!("[emit] phase-completed 发送失败: {e}");
    }
}

#[tauri::command]
fn timer_start(state: State<AppState>) {
    state.timer.lock().unwrap().start(now_ms());
}

#[tauri::command]
fn timer_pause(state: State<AppState>) {
    state.timer.lock().unwrap().pause(now_ms());
}

#[tauri::command]
fn timer_resume(state: State<AppState>) {
    state.timer.lock().unwrap().resume(now_ms());
}

#[tauri::command]
fn timer_reset(state: State<AppState>) {
    state.timer.lock().unwrap().reset();
}

#[tauri::command]
fn timer_skip(state: State<AppState>) -> timer::PhaseCompleted {
    state.timer.lock().unwrap().skip()
}

#[tauri::command]
fn timer_snapshot(state: State<AppState>) -> TimerSnapshot {
    state.timer.lock().unwrap().snapshot()
}

/// 「开始/暂停」切换：托盘菜单与全局快捷键共用（与前端 toggleRun 同语义）。
/// 状态变化经 250ms tick 广播回前端，UI 无需额外事件。
fn toggle_run(state: &AppState) {
    let mut t = state.timer.lock().unwrap();
    match t.snapshot().status {
        Status::Running => t.pause(now_ms()),
        Status::Paused => t.resume(now_ms()),
        Status::Idle => t.start(now_ms()),
    }
}

#[tauri::command]
fn set_config(
    state: State<AppState>,
    work_min: u64,
    short_break_min: u64,
    long_break_min: u64,
    long_break_every: u32,
    auto_start_next: bool,
    daily_goal: u32,
) {
    {
        let mut t = state.timer.lock().unwrap();
        let mut cfg = *t.config();
        cfg.work_ms = work_min * 60_000;
        cfg.short_break_ms = short_break_min * 60_000;
        cfg.long_break_ms = long_break_min * 60_000;
        // every=0 会在 next_phase_after 取模 panic，下限钳 1
        cfg.long_break_every = long_break_every.max(1);
        cfg.auto_start_next = auto_start_next;
        // 目标刻度锚点：0 会让周/月图退化回纯相对刻度，钳 1–20（与前端步进器范围一致）
        cfg.daily_goal = daily_goal.clamp(1, 20);
        t.set_config(cfg);
    } // 先放锁再组 AppConfig（current_appconfig 内部要锁 timer，防自死锁）
    let _ = current_appconfig(&state).save(&config_path());
}

#[tauri::command]
fn get_config(state: State<AppState>) -> AppConfig {
    current_appconfig(&state)
}

/// 材质切换只动 UI 不动计时：更新内存态并落盘（不触碰 Timer）
#[tauri::command]
fn set_material(state: State<AppState>, material: Material) {
    *state.material.lock().unwrap() = material;
    let _ = current_appconfig(&state).save(&config_path());
}

/// 三阶段自定义配色（08-16 v16）：同 set_material 模式——只动 UI 不动计时；
/// 前端已先行校验，这里再验一道防绕过（非法 hex 不落盘不生效）
#[tauri::command]
fn set_colors(state: State<AppState>, colors: appconfig::Colors) -> Result<(), String> {
    colors.validate()?;
    *state.colors.lock().unwrap() = colors;
    let _ = current_appconfig(&state).save(&config_path());
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
    AppConfig {
        timer: *state.timer.lock().unwrap().config(),
        material: *state.material.lock().unwrap(),
        hotkey: state.hotkey.lock().unwrap().clone(),
        colors: state.colors.lock().unwrap().clone(),
    }
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
    let _ = current_appconfig(&state).save(&config_path());
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
    app.exit(0);
}

#[tauri::command]
fn log_js_error(message: String) {
    eprintln!("[js-error] {message}");
}

/* ---------------- v11 托盘自绘菜单配套命令 ---------------- */

/// 托盘菜单「开始/暂停」：与全局快捷键共用 toggle 链路（状态经 tick 广播回前端）
#[tauri::command]
fn timer_toggle(state: State<AppState>) {
    toggle_run(&state);
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

/* ---------------- 自研拖拽：begin 后线程跟随光标，end 停止 ---------------- */

#[cfg(windows)]
mod drag {
    use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};

    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static HWND_VAL: AtomicIsize = AtomicIsize::new(0);
    // 抓取点 CSS 坐标（f64 bits）：物理偏移在跟随循环里按「当前帧 DPI」现算——
    // 跨屏拖动到不同缩放显示器时 Windows 会改窗口 DPI，begin 时固化偏移会漂移（v10 审查立修）
    static GRAB_CSS_X: AtomicU64 = AtomicU64::new(0);
    static GRAB_CSS_Y: AtomicU64 = AtomicU64::new(0);

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetCursorPos(p: *mut (i32, i32)) -> i32;
        fn GetWindowRect(h: isize, r: *mut (i32, i32, i32, i32)) -> i32;
        fn SetWindowPos(h: isize, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
        fn GetDpiForWindow(h: isize) -> u32;
        fn GetAsyncKeyState(key: i32) -> i16;
    }

    /// grab_css_x/y：抓取点在窗口内的 CSS 像素坐标（由前端事件给出），
    /// 乘以当前帧 DPI 缩放得到物理偏移，真实用户光标正在抓取点上 → 零跳动。
    pub fn begin(hwnd: isize, grab_css_x: f64, grab_css_y: f64) {
        unsafe {
            let mut rect = (0i32, 0i32, 0i32, 0i32);
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return;
            }
            HWND_VAL.store(hwnd, Ordering::SeqCst);
            GRAB_CSS_X.store(grab_css_x.to_bits(), Ordering::SeqCst);
            GRAB_CSS_Y.store(grab_css_y.to_bits(), Ordering::SeqCst);
            ACTIVE.store(true, Ordering::SeqCst);
        }
        std::thread::spawn(|| {
            // 跟随直到 drag_end；物理左键松开兜底自终止（防 mouseup 丢失后窗口永远追光标）。
            // 宽限 1500ms：CDP 合成的拖拽无物理按键状态，宽限期内不误杀；
            // 真实拖拽按键按住期间 GetAsyncKeyState 恒为按下，不受影响。
            let started = std::time::Instant::now();
            while ACTIVE.load(Ordering::SeqCst) {
                unsafe {
                    if started.elapsed() > std::time::Duration::from_millis(1500)
                        && GetAsyncKeyState(0x01) & (0x8000u16 as i16) == 0
                    {
                        // VK_LBUTTON 已松开且过宽限期
                        break;
                    }
                    let hwnd = HWND_VAL.load(Ordering::SeqCst);
                    // 每帧现取 DPI：跨屏拖动到不同缩放显示器时偏移跟着换档，不漂移
                    let scale = GetDpiForWindow(hwnd) as f64 / 96.0;
                    let scale = if scale > 0.0 { scale } else { 1.0 };
                    let ox = (f64::from_bits(GRAB_CSS_X.load(Ordering::SeqCst)) * scale) as i32;
                    let oy = (f64::from_bits(GRAB_CSS_Y.load(Ordering::SeqCst)) * scale) as i32;
                    let mut cursor = (0i32, 0i32);
                    if GetCursorPos(&mut cursor) != 0 {
                        SetWindowPos(
                            hwnd,
                            0,
                            cursor.0 - ox,
                            cursor.1 - oy,
                            0,
                            0,
                            0x0001 | 0x0010, // SWP_NOSIZE | SWP_NOACTIVATE
                        );
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(8));
            }
            ACTIVE.store(false, Ordering::SeqCst);
        });
    }

    pub fn end() {
        ACTIVE.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
fn drag_begin(window: tauri::WebviewWindow, grab_x: f64, grab_y: f64) {
    #[cfg(windows)]
    {
        if let Ok(hwnd) = window.hwnd() {
            drag::begin(hwnd.0 as isize, grab_x, grab_y);
        }
    }
}

#[tauri::command]
fn drag_end() {
    #[cfg(windows)]
    drag::end();
}

/* ---------------- 窗口几何瞬时切换（morph 动画在 CSS 侧做，窗口只做一次性 set） ----------------
   灵动岛式 morph：窗口瞬时 set 到目标尺寸（透明窗口下不可见），卡片形变由前端
   WAAPI 在合成器/60fps 完成。begin 负责防出屏钳制并返回旧原点在新窗口内的偏移，
   commit 用于收起动画结束后把窗口落到迷你尺寸。 */

#[cfg(windows)]
mod morph {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetWindowRect(h: isize, r: *mut (i32, i32, i32, i32)) -> i32;
        fn SetWindowPos(h: isize, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
        fn GetDpiForWindow(h: isize) -> u32;
        fn MonitorFromWindow(h: isize, flags: u32) -> isize;
        fn GetMonitorInfoW(m: isize, info: *mut MonitorInfo) -> i32;
    }

    #[repr(C)]
    struct MonitorInfo {
        cb_size: u32,
        rc_monitor: (i32, i32, i32, i32),
        rc_work: (i32, i32, i32, i32),
        dw_flags: u32,
    }

    fn clamp(v: i32, lo: i32, hi: i32) -> i32 {
        v.max(lo).min(hi.max(lo))
    }

    /// 瞬时把窗口设为 (to_w, to_h)（逻辑像素），位置钳制在当前显示器工作区内。
    /// 返回 (dx, dy)：旧窗口原点在新窗口内的 CSS 像素偏移（卡片动画的起点锚）。
    pub fn begin(hwnd: isize, to_w: f64, to_h: f64) -> (f64, f64) {
        unsafe {
            let scale = GetDpiForWindow(hwnd) as f64 / 96.0;
            let scale = if scale > 0.0 { scale } else { 1.0 };
            let mut rect = (0i32, 0i32, 0i32, 0i32);
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return (0.0, 0.0);
            }
            let (x, y) = (rect.0, rect.1);
            let new_w = (to_w * scale) as i32;
            let new_h = (to_h * scale) as i32;
            let mon = MonitorFromWindow(hwnd, 2); // MONITOR_DEFAULTTONEAREST
            let mut info = MonitorInfo {
                cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                rc_monitor: (0, 0, 0, 0),
                rc_work: (0, 0, 0, 0),
                dw_flags: 0,
            };
            let (mut nx, mut ny) = (x, y);
            if GetMonitorInfoW(mon, &mut info) != 0 {
                let wa = info.rc_work;
                nx = clamp(x, wa.0, wa.2 - new_w);
                ny = clamp(y, wa.1, wa.3 - new_h);
            }
            SetWindowPos(hwnd, 0, nx, ny, new_w, new_h, 0x0010); // SWP_NOACTIVATE
            ((x - nx) as f64 / scale, (y - ny) as f64 / scale)
        }
    }

    /// 瞬时将窗口设为 (to_w, to_h)，位置不动（收起 morph 结束后的落点）。
    pub fn commit(hwnd: isize, to_w: f64, to_h: f64) {
        unsafe {
            let scale = GetDpiForWindow(hwnd) as f64 / 96.0;
            let scale = if scale > 0.0 { scale } else { 1.0 };
            SetWindowPos(
                hwnd,
                0,
                0,
                0,
                (to_w * scale) as i32,
                (to_h * scale) as i32,
                0x0002 | 0x0010, // SWP_NOMOVE | SWP_NOACTIVATE
            );
        }
    }
}

#[tauri::command]
fn morph_begin(window: tauri::WebviewWindow, width: f64, height: f64) -> (f64, f64) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        return morph::begin(hwnd.0 as isize, width, height);
    }
    #[allow(unreachable_code)]
    (0.0, 0.0)
}

/* ---------------- 托盘自绘菜单定位（v11）：纯函数可单测 ---------------- */

/// 托盘自绘菜单定位：纯函数可单测
pub mod traymenu {
    /// icon=(l,t,r,b) 物理 px；work=(l,t,r,b) 工作区；mw/mh 菜单物理尺寸 → 菜单左上点（物理 px）
    /// 任务栏边判定：图标矩形「签名距离」最小的工作区边即任务栏所在边（边上为 0 或负）
    pub fn menu_origin(icon: (i32, i32, i32, i32), work: (i32, i32, i32, i32), mw: i32, mh: i32) -> (i32, i32) {
        let d_bottom = work.3 - icon.3;
        let d_top = icon.1 - work.1;
        let d_right = work.2 - icon.2;
        let d_left = icon.0 - work.0;
        let (mut x, mut y);
        if d_bottom.min(d_top) <= d_right.min(d_left) {
            // 任务栏在底/顶：水平对图标居中，垂直贴图标外侧 6px
            x = (icon.0 + icon.2) / 2 - mw / 2;
            y = if d_bottom <= d_top { icon.1 - mh - 6 } else { icon.3 + 6 };
        } else {
            // 任务栏在左/右：垂直对图标居中，水平贴图标外侧 6px
            y = (icon.1 + icon.3) / 2 - mh / 2;
            x = if d_right <= d_left { icon.0 - mw - 6 } else { icon.2 + 6 };
        }
        // 屏幕边缘钳制（hi<lo 兜底不 panic）
        x = x.max(work.0 + 4).min((work.2 - mw - 4).max(work.0 + 4));
        y = y.max(work.1 + 4).min((work.3 - mh - 4).max(work.1 + 4));
        (x, y)
    }

    /// 含某点的显示器工作区（物理 px）；失败回退主屏全屏矩形并记日志（不崩不弹窗）
    #[cfg(windows)]
    pub fn work_area_of_point(px: i32, py: i32) -> (i32, i32, i32, i32) {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MonitorFromPoint(pt: (i32, i32), flags: u32) -> isize;
            fn GetMonitorInfoW(m: isize, info: *mut MonitorInfo) -> i32;
            fn GetSystemMetrics(idx: i32) -> i32;
        }
        #[repr(C)]
        struct MonitorInfo {
            cb_size: u32,
            rc_monitor: (i32, i32, i32, i32),
            rc_work: (i32, i32, i32, i32),
            dw_flags: u32,
        }
        unsafe {
            let mon = MonitorFromPoint((px, py), 2); // DEFAULTTONEAREST
            let mut info = MonitorInfo { cb_size: std::mem::size_of::<MonitorInfo>() as u32, rc_monitor: (0,0,0,0), rc_work: (0,0,0,0), dw_flags: 0 };
            if GetMonitorInfoW(mon, &mut info) != 0 {
                return info.rc_work;
            }
            eprintln!("[tray-menu] GetMonitorInfoW 失败，回退主屏矩形");
            (0, 0, GetSystemMetrics(0), GetSystemMetrics(1))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::menu_origin;

        #[test]
        fn bottom_taskbar_clamped_to_right_edge() {
            // 底部任务栏：贴图标上方，水平居中越右界 → 钳到 work.right-mw-4
            let (x, y) = menu_origin((1870, 1000, 1902, 1032), (0, 0, 1920, 1040), 160, 124);
            assert_eq!((x, y), (1756, 870)); // y=1000-124-6；x=1886-80=1806→钳 1920-160-4
        }

        #[test]
        fn top_taskbar() {
            // 顶部任务栏：贴图标下方，水平对图标居中
            let (x, y) = menu_origin((900, 44, 932, 76), (0, 40, 1920, 1080), 160, 124);
            assert_eq!((x, y), (836, 82)); // y=76+6；x=916-80
        }

        #[test]
        fn left_taskbar() {
            // 左侧任务栏：贴图标右侧，垂直对图标居中；x=56+6=62 < work.left+4=64 → 钳 64
            let (x, y) = menu_origin((4, 500, 56, 532), (60, 0, 1920, 1080), 160, 124);
            assert_eq!((x, y), (64, 454)); // y=516-62
        }

        #[test]
        fn right_taskbar() {
            // 右侧任务栏：贴图标左侧；x=1864-160-6=1698 > work.right-mw-4=1696 → 钳 1696
            let (x, y) = menu_origin((1864, 500, 1916, 532), (0, 0, 1860, 1080), 160, 124);
            assert_eq!((x, y), (1696, 454));
        }

        #[test]
        fn clamp_never_panics_when_menu_bigger_than_workarea() {
            // 菜单比工作区大：hi<lo 兜底（max(lo)），返回 (4,4) 不 panic
            let (x, y) = menu_origin((34, 34, 66, 66), (0, 0, 100, 100), 160, 124);
            assert_eq!((x, y), (4, 4));
        }

        #[test]
        fn bottom_taskbar_centered_no_clamp() {
            // 底部任务栏居中图标：不触发钳制的常规路径
            let (x, y) = menu_origin((944, 1000, 976, 1032), (0, 0, 1920, 1040), 160, 124);
            assert_eq!((x, y), (880, 870)); // x=960-80
        }
    }
}

#[tauri::command]
fn morph_commit(window: tauri::WebviewWindow, width: f64, height: f64) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        morph::commit(hwnd.0 as isize, width, height);
    }
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
                    // 全局快捷键与托盘「开始/暂停」共用 toggle 链路（前端 toggleRun 同语义）
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        toggle_run(&app.state::<AppState>());
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
            let appcfg = AppConfig::load(&config_path());
            app.manage(AppState {
                timer: Mutex::new(Timer::new(appcfg.timer)),
                stats: Mutex::new(stats),
                material: Mutex::new(appcfg.material),
                hotkey: Mutex::new(appcfg.hotkey.clone()),
                colors: Mutex::new(appcfg.colors),
                hotkey_effective: Mutex::new(None),
            });

            // ---------- 托盘自绘菜单窗（v11 弃用原生 muda 菜单）：hidden 启动，右键托盘图标贴图标弹出 ----------
            let mut menu_builder = tauri::WebviewWindowBuilder::new(
                app,
                "tray-menu",
                tauri::WebviewUrl::App("tray-menu.html".into()),
            )
            .title("番茄钟菜单")
            .inner_size(160.0, 124.0)
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
                                let work = traymenu::work_area_of_point(
                                    (icon.0 + icon.2) / 2,
                                    (icon.1 + icon.3) / 2,
                                );
                                let (x, y) = traymenu::menu_origin(icon, work, mw, mh);
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
                let hk = appcfg.hotkey.clone();
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
                let state = app.state::<AppState>();
                *state.hotkey_effective.lock().unwrap() = effective;
            }

            // 计时驱动线程：唯一 tick 来源，250ms 粒度。
            // 08-18 R2：本线程是唯一计时驱动源，任何 panic 若炸掉线程 = 计时器无声冻结——
            // catch_unwind 兜底记日志后继续循环；锁毒化降级取数据（好过整计时冻结）。
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_millis(250));
                let tick_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let state = handle.state::<AppState>();
                    let (ev, snap) = {
                        // 毒锁：持有期间 panic 过的 mutex——into_inner 取回数据继续用
                        let mut t = match state.timer.lock() {
                            Ok(g) => g,
                            Err(poisoned) => poisoned.into_inner(),
                        };
                        let ev = t.tick(now_ms());
                        (ev, t.snapshot())
                    };
                    let _ = handle.emit("timer-tick", TickPayload { snapshot: snap });
                    if let Some(ev) = ev {
                        on_phase_completed(&handle, &state, ev);
                    }
                }));
                if tick_result.is_err() {
                    eprintln!("[tick] 计时线程一轮 panic，已兜底继续（详见上方 panic 输出）");
                }
            });
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
            drag_begin,
            drag_end,
            morph_begin,
            morph_commit,
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
