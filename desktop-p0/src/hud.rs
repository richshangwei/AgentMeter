use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use tauri::{Manager, PhysicalPosition, PhysicalSize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct Placement {
    monitor: String,
    x: i32,
    y: i32,
}

pub struct HudState {
    path: PathBuf,
    placement: Mutex<Option<Placement>>,
    initialized: Mutex<bool>,
    operation: Mutex<()>,
    dragging: AtomicBool,
    moved: mpsc::Sender<PhysicalPosition<i32>>,
}

#[derive(Clone, Copy)]
struct Area {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn clamp_position(area: Area, size: PhysicalSize<u32>, x: i32, y: i32) -> PhysicalPosition<i32> {
    let max_x = i64::from(area.x) + i64::from(area.width.saturating_sub(size.width));
    let max_y = i64::from(area.y) + i64::from(area.height.saturating_sub(size.height));
    PhysicalPosition::new(
        i64::from(x).clamp(i64::from(area.x), max_x) as i32,
        i64::from(y).clamp(i64::from(area.y), max_y) as i32,
    )
}

fn area(monitor: &tauri::Monitor) -> Area {
    let rect = monitor.work_area();
    Area {
        x: rect.position.x,
        y: rect.position.y,
        width: rect.size.width,
        height: rect.size.height,
    }
}

fn monitor_id(monitor: &tauri::Monitor) -> String {
    monitor
        .name()
        .cloned()
        .unwrap_or_else(|| format!("{}:{}", monitor.position().x, monitor.position().y))
}

fn load(path: &Path) -> Result<Option<Placement>, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| format!("invalid HUD position: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot read HUD position: {error}")),
    }
}

fn save(path: &Path, placement: &Placement) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(
        &temporary,
        serde_json::to_vec(placement).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(temporary, path).map_err(|e| format!("cannot save HUD position: {e}"))
}

pub fn initialize(app: &tauri::AppHandle, path: PathBuf) {
    let placement = load(&path).unwrap_or_else(|error| {
        eprintln!("{error}");
        None
    });
    let (sender, receiver) = mpsc::channel();
    app.manage(HudState {
        path,
        placement: Mutex::new(placement),
        initialized: Mutex::new(false),
        operation: Mutex::new(()),
        dragging: AtomicBool::new(false),
        moved: sender,
    });
    let app = app.clone();
    std::thread::spawn(move || {
        while receiver.recv().is_ok() {
            loop {
                match receiver.recv_timeout(Duration::from_millis(300)) {
                    Ok(_) => {}
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            let state = app.state::<HudState>();
            let Ok(_operation) = state.operation.lock() else {
                return;
            };
            let Some(window) = app.get_webview_window("hud") else {
                return;
            };
            let Ok(Some(monitor)) = window.current_monitor() else {
                continue;
            };
            let Ok(position) = window.outer_position() else {
                continue;
            };
            let placement = Placement {
                monitor: monitor_id(&monitor),
                x: position.x,
                y: position.y,
            };
            if let Err(error) = persist(&state, placement) {
                eprintln!("{error}");
            }
        }
    });
}

pub fn moved(app: &tauri::AppHandle, position: PhysicalPosition<i32>) {
    if let Some(state) = app.try_state::<HudState>()
        && state
            .initialized
            .try_lock()
            .is_ok_and(|initialized| *initialized)
    {
        let _ = state.moved.send(position);
    }
}

fn drag_in_progress(state: &HudState) -> bool {
    if !state.dragging.load(Ordering::Acquire) {
        return false;
    }
    #[cfg(windows)]
    let pressed = unsafe {
        // The native drag loop captures mouse release, so webview pointerup cannot end it.
        use windows_sys::Win32::UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON},
            WindowsAndMessaging::{GetSystemMetrics, SM_SWAPBUTTON},
        };
        let primary_button = if GetSystemMetrics(SM_SWAPBUTTON) != 0 {
            VK_RBUTTON
        } else {
            VK_LBUTTON
        };
        GetAsyncKeyState(i32::from(primary_button)) < 0
    };
    #[cfg(not(windows))]
    let pressed = false;
    if !pressed {
        state.dragging.store(false, Ordering::Release);
    }
    pressed
}

fn persist(state: &HudState, placement: Placement) -> Result<(), String> {
    let mut saved = state.placement.lock().map_err(|e| e.to_string())?;
    if saved.as_ref() != Some(&placement) {
        save(&state.path, &placement)?;
        *saved = Some(placement);
    }
    Ok(())
}

fn window(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, String> {
    app.get_webview_window("hud")
        .ok_or_else(|| "hud window unavailable".into())
}

fn chosen_monitor(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
) -> Result<tauri::Monitor, String> {
    let state = app.state::<HudState>();
    let initialized = *state.initialized.lock().map_err(|e| e.to_string())?;
    let preferred = if initialized {
        window
            .current_monitor()
            .map_err(|e| e.to_string())?
            .map(|monitor| monitor_id(&monitor))
    } else {
        state
            .placement
            .lock()
            .map_err(|e| e.to_string())?
            .as_ref()
            .map(|saved| saved.monitor.clone())
    };
    let primary = window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .map(|monitor| monitor_id(&monitor));
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let ids: Vec<_> = monitors.iter().map(monitor_id).collect();
    select_monitor_index(&ids, preferred.as_deref(), primary.as_deref())
        .map(|index| monitors[index].clone())
        .ok_or_else(|| "no monitor available".into())
}

fn select_monitor_index(
    ids: &[String],
    preferred: Option<&str>,
    primary: Option<&str>,
) -> Option<usize> {
    preferred
        .and_then(|id| ids.iter().position(|candidate| candidate == id))
        .or_else(|| primary.and_then(|id| ids.iter().position(|candidate| candidate == id)))
        .or_else(|| (!ids.is_empty()).then_some(0))
}

fn corner(monitor: &tauri::Monitor, size: PhysicalSize<u32>) -> PhysicalPosition<i32> {
    bottom_right(area(monitor), size, monitor.scale_factor())
}

fn physical_size(width: f64, height: f64, scale: f64) -> PhysicalSize<u32> {
    PhysicalSize::new(
        (width.clamp(220.0, 300.0) * scale).round() as u32,
        (height.clamp(48.0, 160.0) * scale).round() as u32,
    )
}

fn restore_position(
    saved: Option<&Placement>,
    id: &str,
    rect: Area,
    size: PhysicalSize<u32>,
    scale: f64,
) -> PhysicalPosition<i32> {
    let position = saved
        .filter(|saved| saved.monitor == id)
        .map(|saved| PhysicalPosition::new(saved.x, saved.y))
        .unwrap_or_else(|| bottom_right(rect, size, scale));
    clamp_position(rect, size, position.x, position.y)
}

fn bottom_right(rect: Area, size: PhysicalSize<u32>, scale: f64) -> PhysicalPosition<i32> {
    let margin = (12.0 * scale).round() as i64;
    clamp_position(
        rect,
        size,
        (i64::from(rect.x) + i64::from(rect.width) - i64::from(size.width) - margin)
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        (i64::from(rect.y) + i64::from(rect.height) - i64::from(size.height) - margin)
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
    )
}

#[tauri::command]
pub async fn configure_hud(
    app: tauri::AppHandle,
    enabled: bool,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let state = app.state::<HudState>();
    let _operation = state.operation.lock().map_err(|e| e.to_string())?;
    let window = window(&app)?;
    if !enabled {
        return window.hide().map_err(|e| e.to_string());
    }
    if !width.is_finite() || !height.is_finite() {
        return Err("invalid hud geometry".into());
    }
    if drag_in_progress(&state) {
        return Ok(());
    }
    let monitor = chosen_monitor(&app, &window)?;
    let size = physical_size(width, height, monitor.scale_factor());
    let initialized = *state.initialized.lock().map_err(|e| e.to_string())?;
    let position = if initialized {
        window.outer_position().map_err(|e| e.to_string())?
    } else {
        restore_position(
            state.placement.lock().map_err(|e| e.to_string())?.as_ref(),
            &monitor_id(&monitor),
            area(&monitor),
            size,
            monitor.scale_factor(),
        )
    };
    let position = clamp_position(area(&monitor), size, position.x, position.y);
    if window.outer_position().map_err(|e| e.to_string())? != position {
        window.set_position(position).map_err(|e| e.to_string())?;
    }
    // Moving between DPI scales can resize the native window; apply target size afterwards.
    if window.outer_size().map_err(|e| e.to_string())? != size {
        window.set_size(size).map_err(|e| e.to_string())?;
    }
    window.set_focusable(false).map_err(|e| e.to_string())?;
    window
        .set_ignore_cursor_events(false)
        .map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    *state.initialized.lock().map_err(|e| e.to_string())? = true;
    persist(
        &state,
        Placement {
            monitor: monitor_id(&monitor),
            x: position.x,
            y: position.y,
        },
    )
}

#[derive(Serialize)]
pub struct MonitorOption {
    id: String,
    label: String,
    primary: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorOptions {
    monitors: Vec<MonitorOption>,
    selected_monitor: Option<String>,
}

#[tauri::command]
pub async fn hud_monitors(app: tauri::AppHandle) -> Result<MonitorOptions, String> {
    let window = window(&app)?;
    let primary = window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .map(|m| monitor_id(&m));
    let selected_monitor = chosen_monitor(&app, &window).ok().map(|m| monitor_id(&m));
    let monitors = window
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| {
            let id = monitor_id(&monitor);
            MonitorOption {
                primary: primary.as_ref() == Some(&id),
                label: format!(
                    "螢幕 {} · {} × {}",
                    index + 1,
                    monitor.size().width,
                    monitor.size().height
                ),
                id,
            }
        })
        .collect();
    Ok(MonitorOptions {
        monitors,
        selected_monitor,
    })
}

fn move_to(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    monitor: tauri::Monitor,
) -> Result<(), String> {
    let current_size = window.outer_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let size = PhysicalSize::new(
        (f64::from(current_size.width) / scale * monitor.scale_factor()).round() as u32,
        (f64::from(current_size.height) / scale * monitor.scale_factor()).round() as u32,
    );
    let position = corner(&monitor, size);
    window.set_position(position).map_err(|e| e.to_string())?;
    window.set_size(size).map_err(|e| e.to_string())?;
    let state = app.state::<HudState>();
    *state.initialized.lock().map_err(|e| e.to_string())? = true;
    persist(
        &state,
        Placement {
            monitor: monitor_id(&monitor),
            x: position.x,
            y: position.y,
        },
    )
}

#[tauri::command]
pub async fn move_hud_monitor(app: tauri::AppHandle, monitor_id: String) -> Result<(), String> {
    let state = app.state::<HudState>();
    let _operation = state.operation.lock().map_err(|e| e.to_string())?;
    let window = window(&app)?;
    let monitor = window
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|m| self::monitor_id(m) == monitor_id)
        .ok_or_else(|| "selected monitor is no longer available".to_string())?;
    move_to(&app, &window, monitor)
}

#[tauri::command]
pub async fn reset_hud_position(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<HudState>();
    let _operation = state.operation.lock().map_err(|e| e.to_string())?;
    let window = window(&app)?;
    let monitor = chosen_monitor(&app, &window)?;
    move_to(&app, &window, monitor)
}

#[tauri::command]
pub async fn start_hud_drag(window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "hud" {
        return Err("only the HUD can start dragging".into());
    }
    let state = window.state::<HudState>();
    let _operation = state.operation.lock().map_err(|e| e.to_string())?;
    state.dragging.store(true, Ordering::Release);
    window.start_dragging().map_err(|e| {
        state.dragging.store(false, Ordering::Release);
        e.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monitor_selection_retains_available_preference_and_recovers_from_disconnect() {
        let ids = vec!["secondary".to_string(), "primary".to_string()];
        assert_eq!(
            select_monitor_index(&ids, Some("secondary"), Some("primary")),
            Some(0)
        );
        assert_eq!(
            select_monitor_index(&ids, Some("removed"), Some("primary")),
            Some(1)
        );
        assert_eq!(select_monitor_index(&ids, None, None), Some(0));
        assert_eq!(
            select_monitor_index(&[], Some("removed"), Some("primary")),
            None
        );
    }
    #[test]
    fn mixed_dpi_uses_target_scale_for_size_and_work_area_margin() {
        let size = physical_size(240.0, 132.0, 1.5);
        assert_eq!(size, PhysicalSize::new(360, 198));
        assert_eq!(
            bottom_right(
                Area {
                    x: 1920,
                    y: -100,
                    width: 2560,
                    height: 1400
                },
                size,
                1.5
            ),
            PhysicalPosition::new(4102, 1084)
        );
    }

    #[test]
    fn disconnected_monitor_restores_on_available_screen_and_reconfigured_screen_clamps() {
        let saved = Placement {
            monitor: "removed".into(),
            x: -1500,
            y: 900,
        };
        let rect = Area {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let size = PhysicalSize::new(240, 132);
        assert_eq!(
            restore_position(Some(&saved), "primary", rect, size, 1.0),
            PhysicalPosition::new(1668, 896)
        );
        assert_eq!(
            restore_position(Some(&saved), "removed", rect, size, 1.0),
            PhysicalPosition::new(0, 900)
        );
        let same = Placement {
            monitor: "primary".into(),
            x: 700,
            y: 400,
        };
        assert_eq!(
            restore_position(Some(&same), "primary", rect, size, 1.0),
            PhysicalPosition::new(700, 400)
        );
    }

    #[test]
    fn clamps_negative_monitor_and_taskbar_work_area() {
        let area = Area {
            x: -1920,
            y: -200,
            width: 1920,
            height: 1040,
        };
        let size = PhysicalSize::new(240, 132);
        assert_eq!(
            clamp_position(area, size, -3000, -400),
            PhysicalPosition::new(-1920, -200)
        );
        assert_eq!(
            clamp_position(area, size, 4000, 4000),
            PhysicalPosition::new(-240, 708)
        );
        assert_eq!(
            clamp_position(area, size, -900, 500),
            PhysicalPosition::new(-900, 500)
        );
    }
    #[test]
    fn oversized_window_anchors_at_work_area_origin() {
        assert_eq!(
            clamp_position(
                Area {
                    x: 30,
                    y: 40,
                    width: 100,
                    height: 50
                },
                PhysicalSize::new(240, 132),
                900,
                800
            ),
            PhysicalPosition::new(30, 40)
        );
    }
    #[test]
    fn position_roundtrip_missing_and_corrupt_file() {
        let path = std::env::temp_dir().join(format!(
            "agentmeter-hud-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert_eq!(load(&path).unwrap(), None);
        let placement = Placement {
            monitor: "display2".into(),
            x: -1500,
            y: 400,
        };
        save(&path, &placement).unwrap();
        assert_eq!(load(&path).unwrap(), Some(placement.clone()));
        save(&path, &placement).unwrap();
        std::fs::write(&path, "bad json").unwrap();
        assert!(load(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
