use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub eq_directory: String,
    pub always_on_top: bool,
    pub sound_enabled: bool,
    pub metronome_enabled: bool,
    pub sound_lead_seconds: f64,
    pub interval_seconds: f64,
    pub cast_time_seconds: f64,
    pub tail_poll_ms: u64,
    pub setup_complete: bool,
    pub chain_tag: String,
    pub alert_slot_taken: bool,
    pub alert_wrong_target: bool,
    pub alert_auto_take_sound: bool,
    pub window_x: Option<f64>,
    pub window_y: Option<f64>,
    pub window_width: Option<f64>,
    pub window_height: Option<f64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            eq_directory: String::new(),
            always_on_top: false,
            sound_enabled: true,
            metronome_enabled: false,
            sound_lead_seconds: 2.0,
            interval_seconds: 2.0,
            cast_time_seconds: 10.0,
            tail_poll_ms: 150,
            setup_complete: false,
            chain_tag: String::from("GG"),
            alert_slot_taken: true,
            alert_wrong_target: true,
            alert_auto_take_sound: true,
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
        }
    }
}

pub const MIN_WINDOW_WIDTH: f64 = 400.0;
pub const MIN_WINDOW_HEIGHT: f64 = 560.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl AppConfig {
    pub fn window_geometry(&self) -> Option<WindowGeometry> {
        let width = clamp_window_width(self.window_width?)?;
        let height = clamp_window_height(self.window_height?)?;
        let x = finite_coord(self.window_x?)?;
        let y = finite_coord(self.window_y?)?;
        Some(WindowGeometry {
            x,
            y,
            width,
            height,
        })
    }

    pub fn set_window_geometry(&mut self, geom: WindowGeometry) {
        self.window_x = Some(geom.x);
        self.window_y = Some(geom.y);
        self.window_width = Some(geom.width);
        self.window_height = Some(geom.height);
    }
}

fn finite_coord(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

pub fn clamp_window_width(width: f64) -> Option<f64> {
    if !width.is_finite() {
        return None;
    }
    Some(width.max(MIN_WINDOW_WIDTH).min(10_000.0))
}

pub fn clamp_window_height(height: f64) -> Option<f64> {
    if !height.is_finite() {
        return None;
    }
    Some(height.max(MIN_WINDOW_HEIGHT).min(10_000.0))
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub fn geometry_on_a_monitor(geom: WindowGeometry, scale: f64, monitors: &[Rect]) -> bool {
    if monitors.is_empty() {
        return true;
    }
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let window = Rect {
        x: (geom.x * scale).round() as i32,
        y: (geom.y * scale).round() as i32,
        w: (geom.width * scale).round() as i32,
        h: (geom.height * scale).round() as i32,
    };
    monitors.iter().any(|monitor| overlap_at_least(window, *monitor, 80))
}

fn overlap_at_least(a: Rect, b: Rect, min: i32) -> bool {
    let w = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
    let h = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
    w >= min && h >= min
}

impl AppConfig {
    pub fn load() -> Self {
        let path = config_file_path();
        match fs::read_to_string(&path) {
            Ok(text) => parse_ini(&text),
            Err(_) => {
                let cfg = Self::default();
                let _ = cfg.save();
                cfg
            }
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serialize_ini(self))
    }
}

pub fn config_file_path() -> PathBuf {
    config_dir().join("config.ini")
}

fn config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    match std::env::consts::OS {
        "macos" => base.join("Alfred"),
        "windows" => base.join("Alfred"),
        _ => base.join("alfred"),
    }
}

fn parse_ini(text: &str) -> AppConfig {
    let mut cfg = AppConfig::default();
    // Configs from before the walkthrough already went through first launch.
    cfg.setup_complete = true;
    let mut section = String::new();

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[') {
            if let Some(name) = name.strip_suffix(']') {
                section = name.trim().to_ascii_lowercase();
                continue;
            }
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = unquote(value.trim());

        match (section.as_str(), key.as_str()) {
            ("general", "eq_directory") => cfg.eq_directory = value,
            ("general", "always_on_top") => cfg.always_on_top = parse_bool(&value),
            ("general", "sound_enabled") => cfg.sound_enabled = parse_bool(&value),
            ("general", "metronome_enabled") => cfg.metronome_enabled = parse_bool(&value),
            ("general", "sound_lead_seconds") => {
                if let Ok(v) = value.parse() {
                    cfg.sound_lead_seconds = v;
                }
            }
            ("general", "tail_poll_ms") => {
                if let Ok(v) = value.parse() {
                    cfg.tail_poll_ms = v;
                }
            }
            ("general", "setup_complete") => cfg.setup_complete = parse_bool(&value),
            ("general", "alert_slot_taken") => cfg.alert_slot_taken = parse_bool(&value),
            ("general", "alert_wrong_target") => cfg.alert_wrong_target = parse_bool(&value),
            ("general", "alert_auto_take_sound") => cfg.alert_auto_take_sound = parse_bool(&value),
            ("chain", "interval_seconds") => {
                if let Ok(v) = value.parse() {
                    cfg.interval_seconds = v;
                }
            }
            ("chain", "cast_time_seconds") => {
                if let Ok(v) = value.parse() {
                    cfg.cast_time_seconds = v;
                }
            }
            ("chain", "tag") | ("chain", "chain_tag") => {
                cfg.chain_tag = normalize_chain_tag(&value);
            }
            ("window", "x") => cfg.window_x = parse_coord(&value),
            ("window", "y") => cfg.window_y = parse_coord(&value),
            ("window", "width") => cfg.window_width = parse_coord(&value),
            ("window", "height") => cfg.window_height = parse_coord(&value),
            _ => {}
        }
    }

    if cfg.metronome_enabled {
        cfg.sound_enabled = false;
    }
    cfg
}

fn serialize_ini(cfg: &AppConfig) -> String {
    let mut text = format!(
        r#"# Alfred — GoodGuys raid assistant
# Config lives in the OS application-support / AppData / XDG config directory.

[general]
eq_directory = {eq_directory}
always_on_top = {always_on_top}
sound_enabled = {sound_enabled}
metronome_enabled = {metronome_enabled}
sound_lead_seconds = {sound_lead_seconds}
# How often to re-read the active log length if the OS misses a write event.
# Native directory events are still used as the primary signal.
tail_poll_ms = {tail_poll_ms}
setup_complete = {setup_complete}
alert_slot_taken = {alert_slot_taken}
alert_wrong_target = {alert_wrong_target}
alert_auto_take_sound = {alert_auto_take_sound}

[chain]
interval_seconds = {interval_seconds}
cast_time_seconds = {cast_time_seconds}
tag = {chain_tag}
"#,
        eq_directory = cfg.eq_directory,
        always_on_top = cfg.always_on_top,
        sound_enabled = cfg.sound_enabled,
        metronome_enabled = cfg.metronome_enabled,
        sound_lead_seconds = cfg.sound_lead_seconds,
        tail_poll_ms = cfg.tail_poll_ms,
        setup_complete = cfg.setup_complete,
        alert_slot_taken = cfg.alert_slot_taken,
        alert_wrong_target = cfg.alert_wrong_target,
        alert_auto_take_sound = cfg.alert_auto_take_sound,
        interval_seconds = cfg.interval_seconds,
        cast_time_seconds = cfg.cast_time_seconds,
        chain_tag = cfg.chain_tag,
    );
    if cfg.window_x.is_some()
        || cfg.window_y.is_some()
        || cfg.window_width.is_some()
        || cfg.window_height.is_some()
    {
        text.push_str("\n[window]\n");
        if let Some(v) = cfg.window_x {
            text.push_str(&format!("x = {v:.0}\n"));
        }
        if let Some(v) = cfg.window_y {
            text.push_str(&format!("y = {v:.0}\n"));
        }
        if let Some(v) = cfg.window_width {
            text.push_str(&format!("width = {v:.0}\n"));
        }
        if let Some(v) = cfg.window_height {
            text.push_str(&format!("height = {v:.0}\n"));
        }
    }
    text
}

fn parse_coord(value: &str) -> Option<f64> {
    value.parse().ok().filter(|v: &f64| v.is_finite())
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub fn normalize_chain_tag(raw: &str) -> String {
    let tag: String = raw
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .take(4)
        .collect::<String>()
        .to_ascii_uppercase();
    if (1..=4).contains(&tag.len()) {
        tag
    } else {
        String::from("GG")
    }
}

fn unquote(value: &str) -> String {
    if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
        || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_ini() {
        let mut cfg = AppConfig::default();
        cfg.eq_directory = r"C:\Games\EQ".into();
        cfg.sound_enabled = false;
        cfg.metronome_enabled = true;
        cfg.always_on_top = true;
        cfg.interval_seconds = 2.5;
        cfg.cast_time_seconds = 9.0;
        cfg.sound_lead_seconds = 1.5;
        cfg.tail_poll_ms = 80;
        cfg.set_window_geometry(WindowGeometry {
            x: 120.0,
            y: 80.0,
            width: 500.0,
            height: 820.0,
        });
        let parsed = parse_ini(&serialize_ini(&cfg));
        assert_eq!(parsed.eq_directory, cfg.eq_directory);
        assert!(!parsed.sound_enabled);
        assert!(parsed.metronome_enabled);
        assert!(parsed.always_on_top);
        assert_eq!(parsed.interval_seconds, 2.5);
        assert_eq!(parsed.cast_time_seconds, 9.0);
        assert_eq!(parsed.sound_lead_seconds, 1.5);
        assert_eq!(parsed.tail_poll_ms, 80);
        assert_eq!(parsed.chain_tag, "GG");
        assert!(!cfg.setup_complete);
        assert!(!parsed.setup_complete);
        assert!(parsed.alert_slot_taken);
        assert!(parsed.alert_wrong_target);
        assert!(parsed.alert_auto_take_sound);
        assert_eq!(parsed.window_geometry(), cfg.window_geometry());
    }

    #[test]
    fn existing_ini_without_setup_complete_is_already_done() {
        let parsed = parse_ini(
            r#"
[general]
eq_directory = /games/eq
sound_enabled = true
"#,
        );
        assert!(parsed.setup_complete);
        let unfinished = parse_ini(
            r#"
[general]
setup_complete = false
"#,
        );
        assert!(!unfinished.setup_complete);
        assert!(!AppConfig::default().setup_complete);
        assert_eq!(AppConfig::default().chain_tag, "GG");
        assert!(AppConfig::default().alert_slot_taken);
        assert!(AppConfig::default().alert_wrong_target);
        assert!(AppConfig::default().alert_auto_take_sound);
        assert!(parsed.alert_slot_taken);
        assert!(parsed.alert_wrong_target);
        assert!(parsed.alert_auto_take_sound);
        let alerts_off = parse_ini(
            r#"
[general]
alert_slot_taken = false
alert_wrong_target = false
alert_auto_take_sound = false
"#,
        );
        assert!(!alerts_off.alert_slot_taken);
        assert!(!alerts_off.alert_wrong_target);
        assert!(!alerts_off.alert_auto_take_sound);
    }

    #[test]
    fn chain_tag_normalizes_and_defaults_to_gg() {
        assert_eq!(normalize_chain_tag("gg"), "GG");
        assert_eq!(normalize_chain_tag("  ca "), "CA");
        assert_eq!(normalize_chain_tag(""), "GG");
        assert_eq!(normalize_chain_tag("12"), "GG");
        assert_eq!(normalize_chain_tag("goodguys"), "GOOD");
        let parsed = parse_ini(
            r#"
[chain]
tag = ss
"#,
        );
        assert_eq!(parsed.chain_tag, "SS");
    }

    #[test]
    fn comments_blank_lines_and_unknown_keys_are_ignored() {
        let parsed = parse_ini(
            r#"
# heading
; also a comment

[general]
eq_directory = /games/eq
unknown = nope
[other]
interval_seconds = 9
[chain]
interval_seconds = 3
"#,
        );
        assert_eq!(parsed.eq_directory, "/games/eq");
        assert_eq!(parsed.interval_seconds, 3.0);
    }

    #[test]
    fn boolean_and_quoted_values() {
        let parsed = parse_ini(
            r#"
[general]
eq_directory = "/Users/Me/EverQuest Folder"
always_on_top = YES
sound_enabled = Off
"#,
        );
        assert_eq!(parsed.eq_directory, "/Users/Me/EverQuest Folder");
        assert!(parsed.always_on_top);
        assert!(!parsed.sound_enabled);
        assert!(parse_bool("true"));
        assert!(parse_bool("1"));
        assert!(parse_bool("on"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("maybe"));
        assert_eq!(unquote(r#"'C:\EQ'"#), r"C:\EQ");
    }

    #[test]
    fn invalid_numbers_keep_defaults() {
        let parsed = parse_ini(
            r#"
[general]
sound_lead_seconds = nope
tail_poll_ms = xyz
[chain]
interval_seconds = abc
cast_time_seconds =
"#,
        );
        let default = AppConfig::default();
        assert_eq!(parsed.sound_lead_seconds, default.sound_lead_seconds);
        assert_eq!(parsed.tail_poll_ms, default.tail_poll_ms);
        assert_eq!(parsed.interval_seconds, default.interval_seconds);
        assert_eq!(parsed.cast_time_seconds, default.cast_time_seconds);
    }

    #[test]
    fn metronome_and_next_up_sound_are_exclusive() {
        let parsed = parse_ini(
            r#"
[general]
sound_enabled = true
metronome_enabled = true
"#,
        );
        assert!(parsed.metronome_enabled);
        assert!(!parsed.sound_enabled);

        let none = parse_ini(
            r#"
[general]
sound_enabled = false
metronome_enabled = false
"#,
        );
        assert!(!none.sound_enabled);
        assert!(!none.metronome_enabled);
    }

    #[test]
    fn leftover_pattern_section_is_ignored() {
        let parsed = parse_ini(
            r#"
[general]
eq_directory = /games/eq
[patterns]
ch_1 = this should not be loaded
ch_2 = (?i)HEAL (\d+) on (\S+)
"#,
        );
        assert_eq!(parsed.eq_directory, "/games/eq");
        let text = serialize_ini(&parsed);
        assert!(!text.contains("[patterns]"));
        assert!(!text.contains("ch_1"));
    }

    #[test]
    fn config_path_uses_os_specific_folder() {
        let path = config_file_path();
        assert!(path.ends_with("config.ini"));
        let dir = config_dir();
        match std::env::consts::OS {
            "linux" => assert!(dir.ends_with("alfred")),
            _ => assert!(dir.ends_with("Alfred")),
        }
    }

    #[test]
    fn serialize_contains_sections_and_comments() {
        let text = serialize_ini(&AppConfig::default());
        assert!(text.contains("[general]"));
        assert!(text.contains("[chain]"));
        assert!(text.contains("alert_slot_taken"));
        assert!(text.contains("alert_wrong_target"));
        assert!(text.contains("alert_auto_take_sound"));
        assert!(!text.contains("[window]"));
        assert!(!text.contains("[patterns]"));
        assert!(!text.contains("ch_1"));
    }

    #[test]
    fn window_geometry_round_trips_and_rejects_junk() {
        let parsed = parse_ini(
            r#"
[window]
x = 40
y = -120
width = 480
height = 900
"#,
        );
        assert_eq!(
            parsed.window_geometry(),
            Some(WindowGeometry {
                x: 40.0,
                y: -120.0,
                width: 480.0,
                height: 900.0,
            })
        );

        let bad = parse_ini(
            r#"
[window]
x = nope
width = 12
height = 20
"#,
        );
        assert!(bad.window_x.is_none());
        assert_eq!(clamp_window_width(12.0), Some(MIN_WINDOW_WIDTH));
        assert_eq!(clamp_window_height(20.0), Some(MIN_WINDOW_HEIGHT));
        assert!(bad.window_geometry().is_none());
    }

    #[test]
    fn window_restore_skips_geometry_that_misses_every_monitor() {
        let geom = WindowGeometry {
            x: 8000.0,
            y: 8000.0,
            width: 460.0,
            height: 800.0,
        };
        let main = Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        };
        assert!(geometry_on_a_monitor(
            WindowGeometry {
                x: 100.0,
                y: 80.0,
                width: 460.0,
                height: 800.0,
            },
            1.0,
            &[main]
        ));
        assert!(!geometry_on_a_monitor(geom, 1.0, &[main]));
        assert!(geometry_on_a_monitor(geom, 1.0, &[]));
    }
}
