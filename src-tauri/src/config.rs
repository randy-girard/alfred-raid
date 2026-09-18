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
        }
    }
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
            _ => {}
        }
    }

    if cfg.metronome_enabled {
        cfg.sound_enabled = false;
    }
    cfg
}

fn serialize_ini(cfg: &AppConfig) -> String {
    format!(
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

[chain]
interval_seconds = {interval_seconds}
cast_time_seconds = {cast_time_seconds}
"#,
        eq_directory = cfg.eq_directory,
        always_on_top = cfg.always_on_top,
        sound_enabled = cfg.sound_enabled,
        metronome_enabled = cfg.metronome_enabled,
        sound_lead_seconds = cfg.sound_lead_seconds,
        tail_poll_ms = cfg.tail_poll_ms,
        interval_seconds = cfg.interval_seconds,
        cast_time_seconds = cfg.cast_time_seconds,
    )
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
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
        let parsed = parse_ini(&serialize_ini(&cfg));
        assert_eq!(parsed.eq_directory, cfg.eq_directory);
        assert!(!parsed.sound_enabled);
        assert!(parsed.metronome_enabled);
        assert!(parsed.always_on_top);
        assert_eq!(parsed.interval_seconds, 2.5);
        assert_eq!(parsed.cast_time_seconds, 9.0);
        assert_eq!(parsed.sound_lead_seconds, 1.5);
        assert_eq!(parsed.tail_poll_ms, 80);
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
        assert!(!text.contains("[patterns]"));
        assert!(!text.contains("ch_1"));
    }
}
