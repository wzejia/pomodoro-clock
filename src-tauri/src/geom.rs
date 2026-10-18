//! 窗口几何：拖拽、展开钳制、托盘菜单位置。计时驱动不在这里。

use tauri::AppHandle;


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
pub fn drag_begin(window: tauri::WebviewWindow, grab_x: f64, grab_y: f64) {
    #[cfg(windows)]
    {
        if let Ok(hwnd) = window.hwnd() {
            drag::begin(hwnd.0 as isize, grab_x, grab_y);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (window, grab_x, grab_y);
    }
}

#[tauri::command]
pub fn drag_end() {
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

fn clamp_i32(v: i32, lo: i32, hi: i32) -> i32 {
    v.max(lo).min(hi.max(lo))
}

/// 非 Windows（以及 Windows 上拿不到 hwnd 时）用 Tauri monitor 的工作区钳制窗口。
/// 取不到显示器时只改尺寸，偏移返回 (0, 0)，并记日志。
fn morph_with_monitor(window: &tauri::WebviewWindow, to_w: f64, to_h: f64) -> (f64, f64) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let Ok(origin) = window.outer_position() else {
        let _ = window.set_size(tauri::LogicalSize::new(to_w, to_h));
        return (0.0, 0.0);
    };
    let new_w = (to_w * scale).round() as i32;
    let new_h = (to_h * scale).round() as i32;
    let mut nx = origin.x;
    let mut ny = origin.y;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    if let Some(mon) = monitor {
        let wa = mon.work_area();
        let left = wa.position.x;
        let top = wa.position.y;
        let right = left.saturating_add(wa.size.width as i32);
        let bottom = top.saturating_add(wa.size.height as i32);
        nx = clamp_i32(origin.x, left, right - new_w);
        ny = clamp_i32(origin.y, top, bottom - new_h);
    } else {
        eprintln!("[morph] 没有取到显示器工作区，只改尺寸，不钳位置");
    }
    let _ = window.set_position(tauri::PhysicalPosition::new(nx, ny));
    let _ = window.set_size(tauri::LogicalSize::new(to_w, to_h));
    (
        (origin.x - nx) as f64 / scale,
        (origin.y - ny) as f64 / scale,
    )
}

#[tauri::command]
pub fn morph_begin(window: tauri::WebviewWindow, width: f64, height: f64) -> (f64, f64) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        return morph::begin(hwnd.0 as isize, width, height);
    }
    morph_with_monitor(&window, width, height)
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

    /// 物理像素矩形 (left, top, right, bottom)。
    pub fn work_area_rect(x: i32, y: i32, width: u32, height: u32) -> (i32, i32, i32, i32) {
        (
            x,
            y,
            x.saturating_add(width as i32),
            y.saturating_add(height as i32),
        )
    }

    /// 点落在某块工作区内就用它；否则选中心最近的一块。空列表返回 None。
    pub fn pick_work_area(px: i32, py: i32, areas: &[(i32, i32, i32, i32)]) -> Option<(i32, i32, i32, i32)> {
        if let Some(hit) = areas
            .iter()
            .copied()
            .find(|&a| px >= a.0 && py >= a.1 && px < a.2 && py < a.3)
        {
            return Some(hit);
        }
        areas.iter().copied().min_by_key(|a| {
            let cx = (a.0 as i64 + a.2 as i64) / 2;
            let cy = (a.1 as i64 + a.3 as i64) / 2;
            let dx = cx - px as i64;
            let dy = cy - py as i64;
            dx * dx + dy * dy
        })
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

        #[test]
        fn pick_work_area_hits_the_monitor_containing_the_point() {
            let areas = [(0, 0, 1920, 1080), (1920, 0, 3840, 1080)];
            assert_eq!(super::pick_work_area(2000, 40, &areas), Some(areas[1]));
        }

        #[test]
        fn pick_work_area_uses_nearest_when_point_is_outside() {
            let areas = [(0, 0, 800, 600), (2000, 0, 2800, 600)];
            assert_eq!(super::pick_work_area(1500, 10, &areas), Some(areas[1]));
            assert_eq!(super::pick_work_area(10, 10, &[]), None);
        }
    }
}

/// 托盘图标所在显示器的工作区（物理 px，left/top/right/bottom）。
/// Windows 走 Win32。其他平台用 Tauri 的 monitor API；都取不到时才回退 1920×1080。
pub(crate) fn tray_work_area(app: &AppHandle, px: i32, py: i32) -> (i32, i32, i32, i32) {
    #[cfg(windows)]
    {
        return traymenu::work_area_of_point(px, py);
    }
    #[cfg(not(windows))]
    {
        let rect_of = |m: &tauri::Monitor| {
            let wa = m.work_area();
            traymenu::work_area_rect(wa.position.x, wa.position.y, wa.size.width, wa.size.height)
        };
        if let Ok(Some(monitor)) = app.monitor_from_point(px as f64, py as f64) {
            return rect_of(&monitor);
        }
        if let Ok(list) = app.available_monitors() {
            let areas: Vec<_> = list.iter().map(rect_of).collect();
            if let Some(area) = traymenu::pick_work_area(px, py, &areas) {
                return area;
            }
        }
        eprintln!("[tray-menu] 没有取到显示器工作区，回退 1920x1080");
        (0, 0, 1920, 1080)
    }
}

#[tauri::command]
pub fn morph_commit(window: tauri::WebviewWindow, width: f64, height: f64) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        morph::commit(hwnd.0 as isize, width, height);
        return;
    }
    let _ = window.set_size(tauri::LogicalSize::new(width, height));
}
