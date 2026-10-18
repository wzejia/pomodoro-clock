//! 计时驱动线程。窗口几何在 geom.rs，状态机在 timer.rs。

use std::sync::atomic::Ordering;
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};

use crate::timer::Sample;
use crate::{mono_ms, on_phase_completed, persist, wall_ms, AppState, TickPayload};

/// 250ms 一拍。刷盘期间不采样。间隔异常由 sample 暂停。
pub fn spawn(handle: AppHandle) {
    std::thread::spawn(move || {
        let mut last_checkpoint = Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(250));
            let tick_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let state = handle.state::<AppState>();
                if state.io_hold.load(Ordering::SeqCst) {
                    return;
                }
                let sample = {
                    let mut t = match state.timer.lock() {
                        Ok(g) => g,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    let sample = t.sample(mono_ms(), wall_ms());
                    let snap = t.snapshot();
                    drop(t);
                    let _ = handle.emit("timer-tick", TickPayload { snapshot: snap });
                    sample
                };
                let saved = match sample {
                    Sample::Ticked(Some(ev)) => {
                        on_phase_completed(&handle, &state, ev);
                        true
                    }
                    Sample::AutoPaused => {
                        persist(&state, || {});
                        true
                    }
                    Sample::Ticked(None) if last_checkpoint.elapsed() >= std::time::Duration::from_secs(5) => {
                        persist(&state, || {});
                        true
                    }
                    Sample::Ticked(None) | Sample::Idle => false,
                };
                if saved {
                    // persist 自己会按墙钟判断刷盘期间的睡眠，并在没有跳变时重新锚定。
                    last_checkpoint = Instant::now();
                }
            }));
            if tick_result.is_err() {
                eprintln!("[tick] 计时线程一轮 panic，已兜底继续（详见上方 panic 输出）");
            }
        }
    });
}
