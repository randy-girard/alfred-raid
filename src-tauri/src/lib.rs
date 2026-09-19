mod chain;
mod config;
mod engine;
mod log_watcher;
mod parser;

use chain::{ChainSnapshot, ChainState};
use config::{geometry_on_a_monitor, AppConfig, Rect, WindowGeometry};
use engine::apply_lines;
use log_watcher::{spawn_watcher, EqDirectoryProbe, WatchEvent, WatchStatus, WatcherHandle};
use parser::{format_test_log_line, Parser, TestChannel};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Position, Size, State, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

struct AppState {
    config: Mutex<AppConfig>,
    chain: Mutex<ChainState>,
    rampage: Mutex<ChainState>,
    parser: Mutex<Parser>,
    status: Mutex<WatchStatus>,
    watcher: Mutex<Option<WatcherHandle>>,
    window_persist: WindowPersist,
}

struct WindowPersist {
    last_change: Mutex<Instant>,
    running: AtomicBool,
}

impl Default for WindowPersist {
    fn default() -> Self {
        Self {
            last_change: Mutex::new(Instant::now()),
            running: AtomicBool::new(false),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RaidSnapshot {
    chain: ChainSnapshot,
    rampage: ChainSnapshot,
}

impl AppState {
    fn new() -> Self {
        let mut config = AppConfig::load();
        if config.eq_directory.trim().is_empty() {
            if let Some(probe) = log_watcher::discover_eq_directory() {
                config.eq_directory = probe.path;
                let _ = config.save();
            }
        }
        let parser = Parser::with_chain_tag(&config.chain_tag);
        let ttl = config.alert_dismiss_ms();
        let mut chain = ChainState::new(config.interval_seconds, config.cast_time_seconds);
        let mut rampage =
            ChainState::new_rampage(config.interval_seconds, config.cast_time_seconds);
        chain.warning_ttl_ms = ttl;
        rampage.warning_ttl_ms = ttl;
        Self {
            config: Mutex::new(config),
            chain: Mutex::new(chain),
            rampage: Mutex::new(rampage),
            parser: Mutex::new(parser),
            status: Mutex::new(WatchStatus::idle(None)),
            watcher: Mutex::new(None),
            window_persist: WindowPersist::default(),
        }
    }
}

#[tauri::command]
fn get_config(state: State<AppState>) -> AppConfig {
    state.config.lock().expect("config").clone()
}

#[tauri::command]
fn get_config_path() -> String {
    config::config_file_path().display().to_string()
}

#[tauri::command]
fn get_snapshot(state: State<AppState>) -> RaidSnapshot {
    RaidSnapshot {
        chain: state.chain.lock().expect("chain").snapshot(),
        rampage: state.rampage.lock().expect("rampage").snapshot(),
    }
}

#[tauri::command]
fn get_watch_status(state: State<AppState>) -> WatchStatus {
    state.status.lock().expect("status").clone()
}

#[tauri::command]
fn inspect_eq_directory(path: String) -> EqDirectoryProbe {
    log_watcher::inspect_eq_directory(&path)
}

#[tauri::command]
fn save_settings(
    state: State<AppState>,
    app: AppHandle,
    patch: SettingsPatch,
) -> Result<AppConfig, String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let mut restart_watch = false;

    if let Some(dir) = patch.eq_directory {
        let dir = log_watcher::normalize_eq_path(&dir);
        if !dir.is_empty() {
            let probe = log_watcher::inspect_eq_directory(&dir);
            if !probe.ok {
                return Err(probe
                    .error
                    .unwrap_or_else(|| "Invalid EverQuest directory".into()));
            }
        }
        if config.eq_directory != dir {
            config.eq_directory = dir;
            restart_watch = true;
        }
    }
    if let Some(v) = patch.always_on_top {
        config.always_on_top = v;
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_always_on_top(v);
        }
    }
    if patch.sound_enabled.is_some() || patch.metronome_enabled.is_some() {
        let metronome = patch.metronome_enabled.unwrap_or(false);
        let sound = patch.sound_enabled.unwrap_or(false);
        if metronome {
            config.metronome_enabled = true;
            config.sound_enabled = false;
        } else if sound {
            config.metronome_enabled = false;
            config.sound_enabled = true;
        } else {
            config.metronome_enabled = false;
            config.sound_enabled = false;
        }
    }
    if let Some(v) = patch.sound_lead_seconds {
        config.sound_lead_seconds = v.max(0.0);
    }
    if let Some(v) = patch.interval_seconds {
        config.interval_seconds = v.max(0.1);
        state
            .chain
            .lock()
            .map_err(|e| e.to_string())?
            .interval_seconds = config.interval_seconds;
        state
            .rampage
            .lock()
            .map_err(|e| e.to_string())?
            .interval_seconds = config.interval_seconds;
    }
    if let Some(v) = patch.cast_time_seconds {
        config.cast_time_seconds = v.max(0.1);
        state
            .chain
            .lock()
            .map_err(|e| e.to_string())?
            .cast_time_seconds = config.cast_time_seconds;
        state
            .rampage
            .lock()
            .map_err(|e| e.to_string())?
            .cast_time_seconds = config.cast_time_seconds;
    }
    if let Some(v) = patch.tail_poll_ms {
        config.tail_poll_ms = v.max(50);
        restart_watch = true;
    }
    if let Some(v) = patch.setup_complete {
        config.setup_complete = v;
    }
    if let Some(tag) = patch.chain_tag {
        let tag = config::normalize_chain_tag(&tag);
        config.chain_tag = tag.clone();
        state
            .parser
            .lock()
            .map_err(|e| e.to_string())?
            .set_chain_tag(&tag);
    }
    if let Some(v) = patch.alert_slot_taken {
        config.alert_slot_taken = v;
    }
    if let Some(v) = patch.alert_wrong_target {
        config.alert_wrong_target = v;
    }
    if let Some(v) = patch.alert_auto_take_sound {
        config.alert_auto_take_sound = v;
    }
    if let Some(v) = patch.alert_start_chain_sound {
        config.alert_start_chain_sound = v;
    }
    if let Some(v) = patch.alert_dismiss_seconds {
        config.alert_dismiss_seconds = v.max(0.0);
        let ttl = config.alert_dismiss_ms();
        state
            .chain
            .lock()
            .map_err(|e| e.to_string())?
            .warning_ttl_ms = ttl;
        state
            .rampage
            .lock()
            .map_err(|e| e.to_string())?
            .warning_ttl_ms = ttl;
    }
    if let Some(v) = patch.overlay_opacity {
        config.overlay_opacity = AppConfig::clamp_overlay_opacity(v);
    }
    if let Some(v) = patch.overlay_clickthrough {
        config.overlay_clickthrough = v;
        if let Some(window) = app.get_webview_window("overlay") {
            apply_overlay_clickthrough(&window, v);
        }
    }

    config.save().map_err(|e| e.to_string())?;
    let cloned = config.clone();
    drop(config);

    if restart_watch {
        start_watcher(&app, &*state)?;
    }
    let _ = app.emit("config-updated", &cloned);
    Ok(cloned)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InjectResult {
    line: String,
    changed: bool,
}

#[tauri::command]
fn inject_test_line(
    state: State<AppState>,
    app: AppHandle,
    speaker: String,
    channel: TestChannel,
    message: String,
) -> Result<InjectResult, String> {
    let line = format_test_log_line(&speaker, channel, &message)?;
    let character = state
        .status
        .lock()
        .map_err(|e| e.to_string())?
        .character
        .clone();
    let parser = state.parser.lock().map_err(|e| e.to_string())?;
    let mut chain = state.chain.lock().map_err(|e| e.to_string())?;
    let mut rampage = state.rampage.lock().map_err(|e| e.to_string())?;
    let changed = apply_lines(
        &parser,
        &mut chain,
        &mut rampage,
        character.as_deref(),
        std::slice::from_ref(&line),
    );
    let snap = RaidSnapshot {
        chain: chain.snapshot(),
        rampage: rampage.snapshot(),
    };
    drop(parser);
    drop(chain);
    drop(rampage);
    let _ = app.emit("raid-updated", &snap);
    Ok(InjectResult { line, changed })
}

#[tauri::command]
async fn open_tester(app: AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window("tester") {
        let _ = existing.show();
        let _ = existing.unminimize();
        let _ = existing.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, "tester", WebviewUrl::App("tester.html".into()))
        .title("Alfred — Test log")
        .inner_size(440.0, 400.0)
        .min_inner_size(360.0, 320.0)
        .resizable(true)
        .always_on_top(true)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn open_overlay(app: AppHandle) -> Result<(), String> {
    let (width, height, position, clickthrough) = {
        let state = app.state::<AppState>();
        let config = state.config.lock().map_err(|e| e.to_string())?;
        let geom = config.overlay_geometry();
        (
            geom.map(|g| g.width).unwrap_or(480.0),
            geom.map(|g| g.height).unwrap_or(360.0),
            geom.map(|g| (g.x, g.y)),
            config.overlay_clickthrough,
        )
    };

    if let Some(existing) = app.get_webview_window("overlay") {
        apply_overlay_clickthrough(&existing, clickthrough);
        let _ = existing.show();
        let _ = existing.unminimize();
        if !clickthrough {
            let _ = existing.set_focus();
        }
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(&app, "overlay", WebviewUrl::App("overlay.html".into()))
        .title("Alfred — Overlay")
        .inner_size(width, height)
        .min_inner_size(config::MIN_OVERLAY_WIDTH, config::MIN_OVERLAY_HEIGHT)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .accept_first_mouse(true)
        .visible(false)
        .focused(!clickthrough)
        .build()
        .map_err(|e| e.to_string())?;

    if let Some((x, y)) = position {
        let geom = WindowGeometry {
            x,
            y,
            width,
            height,
        };
        let scale = window.scale_factor().unwrap_or(1.0);
        if geometry_on_a_monitor(geom, scale, &monitor_rects(&window)) {
            let _ = window.set_position(Position::Logical(LogicalPosition { x, y }));
        }
    }
    apply_overlay_clickthrough(&window, clickthrough);
    let _ = window.show();
    Ok(())
}

#[tauri::command]
fn hide_overlay(app: AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        persist_window_geometry(&window);
        let _ = window.hide();
    }
}

fn apply_overlay_clickthrough(window: &WebviewWindow, clickthrough: bool) {
    let _ = window.set_ignore_cursor_events(clickthrough);
}

fn toggle_overlay(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        if window.is_visible().unwrap_or(false) {
            persist_window_geometry(&window);
            let _ = window.hide();
            return;
        }
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = open_overlay(handle).await;
    });
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPatch {
    eq_directory: Option<String>,
    always_on_top: Option<bool>,
    sound_enabled: Option<bool>,
    metronome_enabled: Option<bool>,
    sound_lead_seconds: Option<f64>,
    interval_seconds: Option<f64>,
    cast_time_seconds: Option<f64>,
    tail_poll_ms: Option<u64>,
    setup_complete: Option<bool>,
    chain_tag: Option<String>,
    alert_slot_taken: Option<bool>,
    alert_wrong_target: Option<bool>,
    alert_auto_take_sound: Option<bool>,
    alert_start_chain_sound: Option<bool>,
    alert_dismiss_seconds: Option<f64>,
    overlay_opacity: Option<f64>,
    overlay_clickthrough: Option<bool>,
}

fn start_watcher(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let eq_dir = PathBuf::from(config.eq_directory.trim());
    let poll_ms = config.tail_poll_ms;
    drop(config);

    let mut watcher_slot = state.watcher.lock().map_err(|e| e.to_string())?;
    if let Some(mut existing) = watcher_slot.take() {
        existing.stop();
    }

    if eq_dir.as_os_str().is_empty() {
        *state.status.lock().map_err(|e| e.to_string())? =
            WatchStatus::idle(Some("Set your EverQuest directory in Settings.".into()));
        let status = state.status.lock().map_err(|e| e.to_string())?.clone();
        let _ = app.emit("watch-status", status);
        return Ok(());
    }

    let handle = app.clone();
    *watcher_slot = Some(spawn_watcher(eq_dir, poll_ms, move |event| match event {
        WatchEvent::Status(status) => {
            if let Some(state) = handle.try_state::<AppState>() {
                if let Some(name) = &status.character {
                    let mut changed = false;
                    if let Ok(mut chain) = state.chain.lock() {
                        let before = chain.your_name.clone();
                        chain.set_your_name(name.clone());
                        changed |= chain.your_name != before;
                    }
                    if let Ok(mut rampage) = state.rampage.lock() {
                        let before = rampage.your_name.clone();
                        rampage.set_your_name(name.clone());
                        changed |= rampage.your_name != before;
                    }
                    if changed {
                        if let (Ok(chain), Ok(rampage)) = (state.chain.lock(), state.rampage.lock())
                        {
                            let _ = handle.emit(
                                "raid-updated",
                                RaidSnapshot {
                                    chain: chain.snapshot(),
                                    rampage: rampage.snapshot(),
                                },
                            );
                        }
                    }
                }
                if let Ok(mut slot) = state.status.lock() {
                    *slot = status.clone();
                }
            }
            let _ = handle.emit("watch-status", status);
        }
        WatchEvent::Lines { character, lines } => {
            if let Some(state) = handle.try_state::<AppState>() {
                let parser = match state.parser.lock() {
                    Ok(p) => p,
                    Err(_) => return,
                };
                let mut chain = match state.chain.lock() {
                    Ok(c) => c,
                    Err(_) => return,
                };
                let mut rampage = match state.rampage.lock() {
                    Ok(c) => c,
                    Err(_) => return,
                };
                if apply_lines(
                    &parser,
                    &mut chain,
                    &mut rampage,
                    character.as_deref(),
                    &lines,
                ) {
                    let _ = handle.emit(
                        "raid-updated",
                        RaidSnapshot {
                            chain: chain.snapshot(),
                            rampage: rampage.snapshot(),
                        },
                    );
                }
            }
        }
    }));
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn monitor_rects(window: &WebviewWindow) -> Vec<Rect> {
    window
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|monitor| {
            let pos = monitor.position();
            let size = monitor.size();
            Rect {
                x: pos.x,
                y: pos.y,
                w: size.width as i32,
                h: size.height as i32,
            }
        })
        .collect()
}

fn current_window_geometry(window: &WebviewWindow) -> Option<WindowGeometry> {
    if window.is_minimized().unwrap_or(false) || window.is_maximized().unwrap_or(false) {
        return None;
    }
    let scale = window.scale_factor().ok()?;
    let pos = window.outer_position().ok()?.to_logical::<f64>(scale);
    let size = window.outer_size().ok()?.to_logical::<f64>(scale);
    let overlay = window.label() == "overlay";
    let width = if overlay {
        config::clamp_overlay_width(size.width)?
    } else {
        config::clamp_window_width(size.width)?
    };
    let height = if overlay {
        config::clamp_overlay_height(size.height)?
    } else {
        config::clamp_window_height(size.height)?
    };
    Some(WindowGeometry {
        x: pos.x,
        y: pos.y,
        width,
        height,
    })
}

fn persist_window_geometry(window: &WebviewWindow) {
    let Some(geom) = current_window_geometry(window) else {
        return;
    };
    let Some(state) = window.try_state::<AppState>() else {
        return;
    };
    let Ok(mut config) = state.config.lock() else {
        return;
    };
    match window.label() {
        "main" => {
            if config.window_geometry() == Some(geom) {
                return;
            }
            config.set_window_geometry(geom);
        }
        "overlay" => {
            if config.overlay_geometry() == Some(geom) {
                return;
            }
            config.set_overlay_geometry(geom);
        }
        _ => return,
    }
    let _ = config.save();
}

fn persist_labeled_windows(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        persist_window_geometry(&window);
    }
    if let Some(window) = app.get_webview_window("overlay") {
        persist_window_geometry(&window);
    }
}

fn schedule_window_save(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if let Ok(mut last) = state.window_persist.last_change.lock() {
        *last = Instant::now();
    }
    if state.window_persist.running.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(300));
        let Some(state) = app.try_state::<AppState>() else {
            break;
        };
        let elapsed = state
            .window_persist
            .last_change
            .lock()
            .ok()
            .map(|when| when.elapsed())
            .unwrap_or(Duration::from_secs(1));
        if elapsed >= Duration::from_millis(300) {
            persist_labeled_windows(&app);
            state.window_persist.running.store(false, Ordering::SeqCst);
            break;
        }
    });
}

fn apply_saved_window(window: &WebviewWindow, state: &AppState) {
    let Ok(config) = state.config.lock() else {
        return;
    };
    let Some(geom) = config.window_geometry() else {
        return;
    };
    drop(config);
    let _ = window.set_size(Size::Logical(LogicalSize {
        width: geom.width,
        height: geom.height,
    }));
    let scale = window.scale_factor().unwrap_or(1.0);
    if geometry_on_a_monitor(geom, scale, &monitor_rects(window)) {
        let _ = window.set_position(Position::Logical(LogicalPosition {
            x: geom.x,
            y: geom.y,
        }));
    }
}

fn main_window(window: &tauri::Window) -> Option<WebviewWindow> {
    window.app_handle().get_webview_window(window.label())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_config_path,
            get_snapshot,
            get_watch_status,
            inspect_eq_directory,
            save_settings,
            inject_test_line,
            open_tester,
            open_overlay,
            hide_overlay
        ])
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Show Alfred", true, None::<&str>)?;
            let chain = MenuItem::with_id(app, "chain", "Cleric Chain", true, None::<&str>)?;
            let rampage = MenuItem::with_id(app, "rampage", "Rampage Chain", true, None::<&str>)?;
            let commands = MenuItem::with_id(app, "commands", "Commands", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let tester = MenuItem::with_id(app, "tester", "Test log", true, None::<&str>)?;
            let overlay = MenuItem::with_id(app, "overlay", "Overlay", true, None::<&str>)?;
            let updates =
                MenuItem::with_id(app, "updates", "Check for updates", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &show, &chain, &rampage, &commands, &settings, &tester, &overlay, &updates,
                    &quit,
                ],
            )?;

            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or("missing window icon")?;

            TrayIconBuilder::new()
                .icon(icon)
                .icon_as_template(false)
                .tooltip("Alfred — GoodGuys")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" | "chain" => {
                        show_main_window(app);
                        let _ = app.emit("open-view", "chain");
                    }
                    "rampage" => {
                        show_main_window(app);
                        let _ = app.emit("open-view", "rampage");
                    }
                    "commands" => {
                        show_main_window(app);
                        let _ = app.emit("open-view", "commands");
                    }
                    "settings" => {
                        show_main_window(app);
                        let _ = app.emit("open-view", "settings");
                    }
                    "tester" => {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = open_tester(handle).await;
                        });
                    }
                    "overlay" => toggle_overlay(app),
                    "updates" => {
                        show_main_window(app);
                        let _ = app.emit("check-updates", ());
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            {
                let handle = app.handle();
                let state = handle.state::<AppState>();
                let always_on_top = state
                    .config
                    .lock()
                    .map(|c| c.always_on_top)
                    .unwrap_or(false);
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.set_always_on_top(always_on_top);
                    apply_saved_window(&window, state.inner());
                    let _ = window.show();
                }
                start_watcher(handle, state.inner())?;
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                if window.label() != "main" && window.label() != "overlay" {
                    return;
                }
                schedule_window_save(window.app_handle());
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if let Some(webview) = main_window(window) {
                    persist_window_geometry(&webview);
                    let _ = webview.hide();
                } else {
                    let _ = window.hide();
                }
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while starting Alfred")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                persist_labeled_windows(app);
            }
        });
}
