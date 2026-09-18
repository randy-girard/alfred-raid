mod chain;
mod config;
mod engine;
mod log_watcher;
mod parser;

use chain::{ChainSnapshot, ChainState};
use serde::Serialize;
use config::AppConfig;
use engine::apply_lines;
use log_watcher::{spawn_watcher, EqDirectoryProbe, WatchEvent, WatchStatus, WatcherHandle};
use parser::Parser;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState {
    config: Mutex<AppConfig>,
    chain: Mutex<ChainState>,
    rampage: Mutex<ChainState>,
    parser: Mutex<Parser>,
    status: Mutex<WatchStatus>,
    watcher: Mutex<Option<WatcherHandle>>,
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
        let parser = Parser::new();
        let chain = ChainState::new(config.interval_seconds, config.cast_time_seconds);
        let rampage = ChainState::new_rampage(config.interval_seconds, config.cast_time_seconds);
        Self {
            config: Mutex::new(config),
            chain: Mutex::new(chain),
            rampage: Mutex::new(rampage),
            parser: Mutex::new(parser),
            status: Mutex::new(WatchStatus::idle(None)),
            watcher: Mutex::new(None),
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
fn save_settings(state: State<AppState>, app: AppHandle, patch: SettingsPatch) -> Result<AppConfig, String> {
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
        state.chain.lock().map_err(|e| e.to_string())?.interval_seconds = config.interval_seconds;
        state.rampage.lock().map_err(|e| e.to_string())?.interval_seconds = config.interval_seconds;
    }
    if let Some(v) = patch.cast_time_seconds {
        config.cast_time_seconds = v.max(0.1);
        state.chain.lock().map_err(|e| e.to_string())?.cast_time_seconds = config.cast_time_seconds;
        state.rampage.lock().map_err(|e| e.to_string())?.cast_time_seconds = config.cast_time_seconds;
    }
    if let Some(v) = patch.tail_poll_ms {
        config.tail_poll_ms = v.max(50);
        restart_watch = true;
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
    *watcher_slot = Some(spawn_watcher(eq_dir, poll_ms, move |event| {
        match event {
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
                            if let (Ok(chain), Ok(rampage)) =
                                (state.chain.lock(), state.rampage.lock())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_config_path,
            get_snapshot,
            get_watch_status,
            inspect_eq_directory,
            save_settings
        ])
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Show Alfred", true, None::<&str>)?;
            let chain = MenuItem::with_id(app, "chain", "Cleric Chain", true, None::<&str>)?;
            let rampage = MenuItem::with_id(app, "rampage", "Rampage Chain", true, None::<&str>)?;
            let commands = MenuItem::with_id(app, "commands", "Commands", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&show, &chain, &rampage, &commands, &settings, &quit],
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
                let always_on_top = state.config.lock().map(|c| c.always_on_top).unwrap_or(false);
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.set_always_on_top(always_on_top);
                }
                start_watcher(handle, state.inner())?;
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Alfred");
}
