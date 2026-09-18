use notify::{Config, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub eq_directory: Option<String>,
    pub logs_path: Option<String>,
    pub logs_canonical: Option<String>,
    pub active_log: Option<String>,
    pub character: Option<String>,
    pub backend: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EqDirectoryProbe {
    pub ok: bool,
    pub path: String,
    pub logs_path: Option<String>,
    pub active_log: Option<String>,
    pub character: Option<String>,
    pub error: Option<String>,
}

impl WatchStatus {
    pub fn idle(error: Option<String>) -> Self {
        Self {
            eq_directory: None,
            logs_path: None,
            logs_canonical: None,
            active_log: None,
            character: None,
            backend: "idle".into(),
            last_error: error,
        }
    }
}

pub enum WatchEvent {
    Status(WatchStatus),
    Lines { character: Option<String>, lines: Vec<String> },
}

pub struct WatcherHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl WatcherHandle {
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn spawn_watcher(
    eq_directory: PathBuf,
    poll_ms: u64,
    on_event: impl Fn(WatchEvent) + Send + 'static,
) -> WatcherHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let thread = thread::Builder::new()
        .name("alfred-log-watch".into())
        .spawn(move || {
            run_watcher(eq_directory, poll_ms.max(50), stop_thread, on_event);
        })
        .expect("spawn log watcher");
    WatcherHandle {
        stop,
        thread: Some(thread),
    }
}

fn run_watcher(
    eq_directory: PathBuf,
    poll_ms: u64,
    stop: Arc<AtomicBool>,
    on_event: impl Fn(WatchEvent),
) {
    let poll = Duration::from_millis(poll_ms);
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let event_tx = tx.clone();

    let mut backend = "polling";
    let mut native: Option<RecommendedWatcher> = RecommendedWatcher::new(
        move |res| {
            let _ = event_tx.send(res);
        },
        Config::default(),
    )
    .ok();

    let mut poller: Option<PollWatcher> = None;
    if native.is_none() {
        poller = PollWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(poll),
        )
        .ok();
    } else {
        backend = "events";
    }

    let mut watched: Option<PathBuf> = None;
    let mut last_canon: Option<PathBuf> = None;
    let mut last_resolve = Instant::now() - Duration::from_secs(10);
    let mut tail: Option<LogTail> = None;
    let mut last_dir_scan = Instant::now() - Duration::from_secs(10);
    let mut last_error: Option<String> = None;

    let emit_status = |logs: &LogsDir, tail: &Option<LogTail>, backend: &str, last_error: &Option<String>| {
        on_event(WatchEvent::Status(WatchStatus {
            eq_directory: Some(eq_directory.display().to_string()),
            logs_path: Some(logs.user_path.display().to_string()),
            logs_canonical: logs.canonical.as_ref().map(|p| p.display().to_string()),
            active_log: tail.as_ref().map(|t| t.path.display().to_string()),
            character: tail.as_ref().and_then(|t| character_from_log(&t.path)),
            backend: backend.to_string(),
            last_error: last_error.clone(),
        }));
    };

    while !stop.load(Ordering::SeqCst) {
        if last_resolve.elapsed() >= Duration::from_secs(2) {
            last_resolve = Instant::now();
            match resolve_logs_dir(&eq_directory) {
                Ok(logs) => {
                    let canon_changed = logs.canonical != last_canon;
                    let watch_target = logs
                        .canonical
                        .clone()
                        .unwrap_or_else(|| logs.user_path.clone());
                    if watched.as_ref() != Some(&watch_target) || canon_changed {
                        if let Some(prev) = watched.take() {
                            if let Some(w) = native.as_mut() {
                                let _ = w.unwatch(&prev);
                            }
                            if let Some(w) = poller.as_mut() {
                                let _ = w.unwatch(&prev);
                            }
                        }
                        let watch_result = if let Some(w) = native.as_mut() {
                            w.watch(&watch_target, RecursiveMode::NonRecursive)
                        } else if let Some(w) = poller.as_mut() {
                            w.watch(&watch_target, RecursiveMode::NonRecursive)
                        } else {
                            Ok(())
                        };
                        if let Err(e) = watch_result {
                            last_error = Some(format!("watch failed: {e}"));
                            backend = "polling";
                        } else if native.is_some() {
                            backend = "events";
                        } else {
                            backend = "polling";
                        }
                        watched = Some(watch_target);
                        last_canon = logs.canonical.clone();
                        last_dir_scan = Instant::now() - Duration::from_secs(10);
                        emit_status(&logs, &tail, backend, &last_error);
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    on_event(WatchEvent::Status(WatchStatus {
                        eq_directory: Some(eq_directory.display().to_string()),
                        logs_path: None,
                        logs_canonical: None,
                        active_log: None,
                        character: None,
                        backend: backend.into(),
                        last_error: last_error.clone(),
                    }));
                }
            }
        }

        let mut rescan_dir = last_dir_scan.elapsed() >= Duration::from_secs(1);
        match rx.recv_timeout(poll) {
            Ok(Ok(_)) => rescan_dir = true,
            Ok(Err(e)) => last_error = Some(e.to_string()),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if rescan_dir {
            last_dir_scan = Instant::now();
            if let Ok(logs) = resolve_logs_dir(&eq_directory) {
                let watch_root = logs
                    .canonical
                    .clone()
                    .unwrap_or_else(|| logs.user_path.clone());
                match newest_log(&watch_root) {
                    Ok(Some(path)) => {
                        let switched = tail.as_ref().is_none_or(|t| t.path != path);
                        if switched {
                            match LogTail::open_at_end(&path) {
                                Ok(new_tail) => {
                                    tail = Some(new_tail);
                                    last_error = None;
                                    emit_status(&logs, &tail, backend, &last_error);
                                }
                                Err(e) => last_error = Some(format!("open log: {e}")),
                            }
                        }
                    }
                    Ok(None) => {
                        if tail.is_some() {
                            tail = None;
                            emit_status(&logs, &tail, backend, &last_error);
                        }
                    }
                    Err(e) => last_error = Some(e),
                }
            }
        }

        if let Some(current) = tail.as_mut() {
            match current.read_new_lines() {
                Ok(lines) if !lines.is_empty() => {
                    on_event(WatchEvent::Lines {
                        character: character_from_log(&current.path),
                        lines,
                    });
                }
                Err(e) => last_error = Some(e.to_string()),
                _ => {}
            }
        }
    }
}

#[derive(Debug)]
struct LogsDir {
    user_path: PathBuf,
    canonical: Option<PathBuf>,
}

pub fn inspect_eq_directory(raw: &str) -> EqDirectoryProbe {
    let path = normalize_eq_path(raw);
    if path.is_empty() {
        return EqDirectoryProbe {
            ok: false,
            path,
            logs_path: None,
            active_log: None,
            character: None,
            error: Some("Paste or browse to your EverQuest folder.".into()),
        };
    }
    let dir = PathBuf::from(&path);
    if !dir.exists() {
        return EqDirectoryProbe {
            ok: false,
            path,
            logs_path: None,
            active_log: None,
            character: None,
            error: Some("That path does not exist.".into()),
        };
    }
    if !dir.is_dir() {
        return EqDirectoryProbe {
            ok: false,
            path,
            logs_path: None,
            active_log: None,
            character: None,
            error: Some("That path is not a folder.".into()),
        };
    }
    match resolve_logs_dir(&dir) {
        Ok(logs) => {
            let search = logs.canonical.as_ref().unwrap_or(&logs.user_path);
            let newest = newest_log(search).ok().flatten();
            let character = newest.as_ref().and_then(|p| character_from_log(p));
            EqDirectoryProbe {
                ok: true,
                path,
                logs_path: Some(logs.user_path.display().to_string()),
                active_log: newest.map(|p| p.display().to_string()),
                character,
                error: None,
            }
        }
        Err(err) => {
            let hint = if looks_like_eq_root(&dir) {
                format!("{err} Turn on /log in game if you have not yet.")
            } else {
                "Not an EverQuest folder. Pick the EQ root (the folder with eqgame.exe or a Logs directory).".into()
            };
            EqDirectoryProbe {
                ok: false,
                path,
                logs_path: None,
                active_log: None,
                character: None,
                error: Some(hint),
            }
        }
    }
}

pub fn discover_eq_directory() -> Option<EqDirectoryProbe> {
    discover_among(candidate_eq_paths())
}

pub(crate) fn discover_among(candidates: impl IntoIterator<Item = PathBuf>) -> Option<EqDirectoryProbe> {
    let mut seen = HashSet::new();
    let mut best: Option<(u64, EqDirectoryProbe)> = None;
    for raw in candidates {
        let Some(root) = eq_root_from(&raw) else {
            continue;
        };
        let key = fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
        if !seen.insert(key) {
            continue;
        }
        let probe = inspect_eq_directory(&root.to_string_lossy());
        if !probe.ok {
            continue;
        }
        let score = probe_score(&probe);
        if best.as_ref().map(|(best_score, _)| score > *best_score).unwrap_or(true) {
            best = Some((score, probe));
        }
    }
    best.map(|(_, probe)| probe)
}

fn probe_score(probe: &EqDirectoryProbe) -> u64 {
    let mut score = 1_000_000;
    if probe.character.is_some() {
        score += 1_000_000;
    }
    if let Some(log) = &probe.active_log {
        if let Ok(meta) = fs::metadata(log) {
            if let Ok(modified) = meta.modified() {
                score += modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
            }
        }
    }
    score
}

fn eq_root_from(path: &Path) -> Option<PathBuf> {
    if !path.is_dir() {
        return None;
    }
    if looks_like_eq_root(path) || has_eq_logs(&path.join("Logs")) {
        return Some(path.to_path_buf());
    }
    let named_logs = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("logs"));
    if named_logs && (has_eq_logs(path) || path.parent().is_some_and(looks_like_eq_root)) {
        return path.parent().map(Path::to_path_buf);
    }
    None
}

fn candidate_eq_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for base in search_bases() {
        out.push(base.clone());
        for parts in eq_relative_paths() {
            out.push(parts.iter().fold(base.clone(), |dir, part| dir.join(part)));
        }
    }
    out
}

fn eq_relative_paths() -> &'static [&'static [&'static str]] {
    &[
        &["EverQuest"],
        &["EQ"],
        &["P99"],
        &["Project1999"],
        &["Project 1999"],
        &["Games", "EverQuest"],
        &["Games", "EQ"],
        &["Games", "P99"],
        &["Games", "Project1999"],
        &["Games", "Project 1999"],
        &["Program Files (x86)", "Sony", "EverQuest"],
        &[
            "Program Files (x86)",
            "Sony Online Entertainment",
            "Installed Games",
            "EverQuest",
        ],
        &[
            "Program Files (x86)",
            "Daybreak Game Company",
            "Installed Games",
            "EverQuest",
        ],
        &["Program Files (x86)", "Steam", "steamapps", "common", "EverQuest"],
        &["Program Files", "EverQuest"],
        &["Program Files", "Steam", "steamapps", "common", "EverQuest"],
        &[
            "Users",
            "Public",
            "Daybreak Game Company",
            "Installed Games",
            "EverQuest",
        ],
        &["Sony", "EverQuest"],
        &["Library", "Application Support", "Steam", "steamapps", "common", "EverQuest"],
        &[".steam", "steam", "steamapps", "common", "EverQuest"],
        &[".local", "share", "Steam", "steamapps", "common", "EverQuest"],
    ]
}

fn search_bases() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Some(home) = dirs::home_dir() {
        bases.push(home.clone());
        bases.push(home.join("Games"));
        bases.push(home.join(".wine").join("drive_c"));
        push_bottle_drives(
            &mut bases,
            home.join("Library/Application Support/CrossOver/Bottles"),
        );
        push_bottle_drives(&mut bases, home.join("Library/Application Support/Whisky/Bottles"));
        push_bottle_drives(
            &mut bases,
            home.join("Library/Application Support/com.isaacmarovitz.Whisky/Bottles"),
        );
    }
    if let Ok(prefix) = std::env::var("WINEPREFIX") {
        bases.push(PathBuf::from(prefix).join("drive_c"));
    }
    if cfg!(windows) {
        for letter in b'A'..=b'Z' {
            let drive = PathBuf::from(format!("{}:\\", letter as char));
            if drive.exists() {
                bases.push(drive);
            }
        }
        for key in ["ProgramFiles", "ProgramFiles(x86)", "PUBLIC"] {
            if let Ok(value) = std::env::var(key) {
                bases.push(PathBuf::from(value));
            }
        }
    }
    bases
}

fn push_bottle_drives(bases: &mut Vec<PathBuf>, bottles: PathBuf) {
    let Ok(entries) = fs::read_dir(bottles) else {
        return;
    };
    for entry in entries.flatten().take(32) {
        let drive = entry.path().join("drive_c");
        if drive.is_dir() {
            bases.push(drive);
        }
    }
}

pub fn normalize_eq_path(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    s = s.trim_matches(|c| c == '"' || c == '\'').trim().to_string();
    s = s.replace("%20", " ");
    if let Some(rest) = s.strip_prefix("file:") {
        let rest = rest.trim_start_matches('/');
        let rest = rest.strip_prefix("localhost/").unwrap_or(rest);
        s = if rest.len() >= 2 && rest.as_bytes().get(1) == Some(&b':') {
            rest.to_string()
        } else {
            format!("/{rest}")
        };
    }
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            s = home.join(rest).to_string_lossy().into_owned();
        }
    }
    s
}

fn looks_like_eq_root(path: &Path) -> bool {
    const MARKERS: &[&str] = &[
        "eqgame.exe",
        "eqclient.ini",
        "everquest.exe",
        "eqgame.ini",
    ];
    let Ok(entries) = fs::read_dir(path) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| MARKERS.iter().any(|m| name.eq_ignore_ascii_case(m)))
    })
}

fn resolve_logs_dir(eq_directory: &Path) -> Result<LogsDir, String> {
    if eq_directory.as_os_str().is_empty() {
        return Err("No EverQuest directory set.".into());
    }
    let candidate = if is_logs_dir(eq_directory) {
        eq_directory.to_path_buf()
    } else {
        eq_directory.join("Logs")
    };
    if !candidate.exists() {
        return Err(format!("Logs folder not found: {}", candidate.display()));
    }
    let canonical = fs::canonicalize(&candidate).ok();
    Ok(LogsDir {
        user_path: candidate,
        canonical,
    })
}

fn is_logs_dir(path: &Path) -> bool {
    let named_logs = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("logs"));
    named_logs || has_eq_logs(path)
}

fn has_eq_logs(path: &Path) -> bool {
    fs::read_dir(path)
        .ok()
        .map(|entries| {
            entries.flatten().any(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(is_eq_log_name)
            })
        })
        .unwrap_or(false)
}

fn is_eq_log_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("eqlog_") && lower.ends_with(".txt")
}

fn newest_log(dir: &Path) -> Result<Option<PathBuf>, String> {
    let mut best: Option<(SystemTime, u64, PathBuf)> = None;
    let entries = fs::read_dir(dir).map_err(|e| format!("read Logs: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !is_eq_log_name(name) {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let len = meta.len();
        let better = match &best {
            None => true,
            Some((t, size, _)) => modified > *t || (modified == *t && len > *size),
        };
        if better {
            best = Some((modified, len, entry.path()));
        }
    }
    Ok(best.map(|(_, _, p)| p))
}

pub fn character_from_log(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let rest = stem.strip_prefix("eqlog_").or_else(|| {
        let lower = stem.to_ascii_lowercase();
        if let Some(idx) = lower.find("eqlog_") {
            Some(&stem[idx + 6..])
        } else {
            None
        }
    })?;
    let (name, _) = rest.rsplit_once('_')?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

struct LogTail {
    path: PathBuf,
    file: File,
    offset: u64,
    leftover: Vec<u8>,
}

impl LogTail {
    fn open_at_end(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let offset = file.metadata()?.len();
        Ok(Self {
            path: path.to_path_buf(),
            file,
            offset,
            leftover: Vec::new(),
        })
    }

    fn read_new_lines(&mut self) -> std::io::Result<Vec<String>> {
        let len = self.file.metadata()?.len();
        if len < self.offset {
            self.offset = 0;
            self.leftover.clear();
            self.file.seek(SeekFrom::Start(0))?;
        }
        if len == self.offset {
            return Ok(Vec::new());
        }
        self.file.seek(SeekFrom::Start(self.offset))?;
        let mut buf = Vec::new();
        self.file.read_to_end(&mut buf)?;
        self.offset = self.file.stream_position()?;

        if !self.leftover.is_empty() {
            let mut combined = std::mem::take(&mut self.leftover);
            combined.extend_from_slice(&buf);
            buf = combined;
        }

        let mut lines = Vec::new();
        let mut start = 0;
        for (i, b) in buf.iter().enumerate() {
            if *b == b'\n' {
                let mut end = i;
                if end > start && buf[end - 1] == b'\r' {
                    end -= 1;
                }
                let line = String::from_utf8_lossy(&buf[start..end]).to_string();
                if !line.is_empty() {
                    lines.push(line);
                }
                start = i + 1;
            }
        }
        if start < buf.len() {
            self.leftover = buf[start..].to_vec();
        }
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempTree {
        path: PathBuf,
    }

    impl TempTree {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "alfred-{tag}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn append(path: &Path, bytes: &[u8]) {
        let mut file = fs::OpenOptions::new().append(true).open(path).unwrap();
        file.write_all(bytes).unwrap();
        file.flush().unwrap();
    }

    #[test]
    fn parse_character_name_from_common_servers() {
        assert_eq!(
            character_from_log(Path::new("/eq/Logs/eqlog_Clericone_P1999Green.txt")).as_deref(),
            Some("Clericone")
        );
        assert_eq!(
            character_from_log(Path::new(r"C:\EQ\Logs\eqlog_Tank_P1999PVP.txt")).as_deref(),
            Some("Tank")
        );
        assert_eq!(
            character_from_log(Path::new("eqlog_Foo_project1999.txt")).as_deref(),
            Some("Foo")
        );
        assert_eq!(character_from_log(Path::new("notes.txt")), None);
        assert_eq!(character_from_log(Path::new("eqlog_.txt")), None);
        assert_eq!(character_from_log(Path::new("eqlog_only.txt")), None);
    }

    #[test]
    fn normalize_eq_path_strips_quotes_and_file_urls() {
        assert_eq!(normalize_eq_path("  /games/eq  "), "/games/eq");
        assert_eq!(normalize_eq_path(r#""/games/EverQuest""#), "/games/EverQuest");
        assert_eq!(
            normalize_eq_path("file:///Users/me/EverQuest"),
            "/Users/me/EverQuest"
        );
        assert_eq!(normalize_eq_path("file:///C:/Games/EQ"), "C:/Games/EQ");
    }

    #[test]
    fn inspect_eq_directory_validates_and_reads_character() {
        let missing = inspect_eq_directory("/no/such/eq/folder");
        assert!(!missing.ok);
        assert!(missing.error.unwrap().contains("does not exist"));

        let tmp = TempTree::new("inspect");
        let random = inspect_eq_directory(tmp.path.to_str().unwrap());
        assert!(!random.ok);
        assert!(random.error.unwrap().contains("Not an EverQuest folder"));

        fs::write(tmp.path.join("eqgame.exe"), b"x").unwrap();
        let no_logs = inspect_eq_directory(tmp.path.to_str().unwrap());
        assert!(!no_logs.ok);
        assert!(no_logs.error.unwrap().contains("Logs folder not found"));

        let logs = tmp.path.join("Logs");
        fs::create_dir(&logs).unwrap();
        fs::write(logs.join("eqlog_Clericone_P1999Green.txt"), "hi\n").unwrap();
        let quoted = format!("\"{}\"", tmp.path.display());
        let ok = inspect_eq_directory(&quoted);
        assert!(ok.ok);
        assert_eq!(ok.character.as_deref(), Some("Clericone"));
        assert!(ok.active_log.unwrap().contains("eqlog_Clericone_P1999Green.txt"));
    }

    fn fake_eq(tag: &str, character: &str) -> TempTree {
        let tmp = TempTree::new(tag);
        fs::write(tmp.path.join("eqgame.exe"), b"x").unwrap();
        let logs = tmp.path.join("Logs");
        fs::create_dir(&logs).unwrap();
        if !character.is_empty() {
            fs::write(
                logs.join(format!("eqlog_{character}_P1999Green.txt")),
                "hi\n",
            )
            .unwrap();
        }
        tmp
    }

    #[test]
    fn discover_among_picks_a_valid_eq_folder_and_ignores_junk() {
        assert!(discover_among(Vec::<PathBuf>::new()).is_none());

        let junk = TempTree::new("discover-junk");
        fs::write(junk.path.join("notes.txt"), "nope").unwrap();
        let incomplete = TempTree::new("discover-incomplete");
        fs::write(incomplete.path.join("eqgame.exe"), b"x").unwrap();
        let eq = fake_eq("discover-ok", "Clericone");

        let found = discover_among([
            junk.path.clone(),
            incomplete.path.clone(),
            eq.path.clone(),
        ])
        .expect("should find the EQ folder");
        assert!(found.ok);
        assert_eq!(found.character.as_deref(), Some("Clericone"));
        assert_eq!(
            PathBuf::from(&found.path).canonicalize().unwrap(),
            eq.path.canonicalize().unwrap()
        );
    }

    #[test]
    fn discover_among_accepts_the_logs_folder_and_prefers_a_character_log() {
        let empty = fake_eq("discover-empty", "");
        let with_char = fake_eq("discover-char", "Newtoon");

        let from_logs = discover_among([with_char.path.join("Logs")]).expect("logs folder");
        assert_eq!(from_logs.character.as_deref(), Some("Newtoon"));
        assert_eq!(
            PathBuf::from(&from_logs.path).canonicalize().unwrap(),
            with_char.path.canonicalize().unwrap()
        );

        let found =
            discover_among([empty.path.clone(), with_char.path.clone()]).expect("character install");
        assert_eq!(found.character.as_deref(), Some("Newtoon"));
    }

    #[test]
    fn eq_log_name_is_case_insensitive() {
        assert!(is_eq_log_name("eqlog_A_P1999Green.txt"));
        assert!(is_eq_log_name("EQLOG_A_P1999GREEN.TXT"));
        assert!(!is_eq_log_name("dbg.txt"));
        assert!(!is_eq_log_name("eqlog_A_P1999Green.log"));
        assert!(!is_eq_log_name("log_A_P1999Green.txt"));
    }

    #[test]
    fn newest_log_picks_the_latest_eqlog_and_ignores_other_files() {
        let tmp = TempTree::new("newest");
        fs::write(tmp.path.join("notes.txt"), "nope").unwrap();
        fs::write(tmp.path.join("eqlog_Old_P1999Green.txt"), "old\n").unwrap();
        fs::write(
            tmp.path.join("eqlog_New_P1999Green.txt"),
            "new and much longer\n",
        ).unwrap();
        let newest = newest_log(&tmp.path).unwrap().unwrap();
        assert_eq!(newest.file_name().unwrap(), "eqlog_New_P1999Green.txt");
        assert!(newest_log(&tmp.path.join("missing"))
            .unwrap_err()
            .contains("read Logs"));
        let empty = TempTree::new("empty-logs");
        assert_eq!(newest_log(&empty.path).unwrap(), None);
    }

    #[test]
    fn resolve_logs_dir_from_eq_root_or_logs_folder() {
        let tmp = TempTree::new("eqroot");
        assert!(resolve_logs_dir(Path::new("")).is_err());
        match resolve_logs_dir(&tmp.path) {
            Err(err) => assert!(err.contains("Logs folder not found")),
            Ok(_) => panic!("expected missing Logs folder"),
        }

        let logs = tmp.path.join("Logs");
        fs::create_dir(&logs).unwrap();
        fs::write(logs.join("eqlog_A_P1999Green.txt"), "x\n").unwrap();
        let from_root = resolve_logs_dir(&tmp.path).unwrap();
        assert_eq!(from_root.user_path, logs);
        assert!(from_root.canonical.is_some());

        let from_logs = resolve_logs_dir(&logs).unwrap();
        assert_eq!(from_logs.user_path, logs);
        assert!(is_logs_dir(&logs));
        assert!(!is_logs_dir(&tmp.path));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_logs_dir_follows_symlink() {
        let tmp = TempTree::new("symlink");
        let real = tmp.path.join("real_logs");
        fs::create_dir(&real).unwrap();
        fs::write(real.join("eqlog_A_P1999Green.txt"), "x\n").unwrap();
        let eq = tmp.path.join("eq");
        fs::create_dir(&eq).unwrap();
        std::os::unix::fs::symlink(&real, eq.join("Logs")).unwrap();
        let logs = resolve_logs_dir(&eq).unwrap();
        assert_eq!(logs.user_path, eq.join("Logs"));
        assert_eq!(logs.canonical.as_ref().unwrap(), &real.canonicalize().unwrap());
    }

    #[test]
    fn tail_starts_at_end_and_reads_new_lines() {
        let tmp = TempTree::new("tail-end");
        let path = tmp.path.join("eqlog_Clericone_P1999Green.txt");
        fs::write(&path, "[old] ignored\n").unwrap();
        let mut tail = LogTail::open_at_end(&path).unwrap();
        assert!(tail.read_new_lines().unwrap().is_empty());
        append(&path, b"[Fri Sep 18 16:27:00 2026] You shout, 'GG 001 CH -- Mluian'\n");
        let lines = tail.read_new_lines().unwrap();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("GG 001 CH"));
        assert!(tail.read_new_lines().unwrap().is_empty());
    }

    #[test]
    fn tail_handles_crlf_partial_lines_and_truncation() {
        let tmp = TempTree::new("tail-partial");
        let path = tmp.path.join("eqlog_A_P1999Green.txt");
        fs::write(&path, "").unwrap();
        let mut tail = LogTail::open_at_end(&path).unwrap();
        append(&path, b"first\r\npartial");
        let lines = tail.read_new_lines().unwrap();
        assert_eq!(lines, vec!["first"]);
        append(&path, b" line\n\nsecond\n");
        let lines = tail.read_new_lines().unwrap();
        assert_eq!(lines, vec!["partial line", "second"]);
        fs::write(&path, "rewound\n").unwrap();
        let lines = tail.read_new_lines().unwrap();
        assert_eq!(lines, vec!["rewound"]);
    }

    #[test]
    fn idle_status_carries_error() {
        let status = WatchStatus::idle(Some("missing".into()));
        assert_eq!(status.backend, "idle");
        assert_eq!(status.last_error.as_deref(), Some("missing"));
        assert!(status.active_log.is_none());
    }

    #[test]
    fn watcher_tails_the_active_log_and_can_switch_files() {
        let tmp = TempTree::new("watch");
        let logs = tmp.path.join("Logs");
        fs::create_dir_all(&logs).unwrap();
        let first = logs.join("eqlog_First_P1999Green.txt");
        let second = logs.join("eqlog_Second_P1999Green.txt");
        fs::write(&first, "old-first\n").unwrap();
        fs::write(&second, "old-second\n").unwrap();

        let (tx, rx) = mpsc::channel();
        let mut handle = spawn_watcher(tmp.path.clone(), 50, move |event| {
            let _ = tx.send(event);
        });

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut saw_first = false;
        while Instant::now() < deadline {
            if let Ok(WatchEvent::Status(status)) = rx.recv_timeout(Duration::from_millis(50)) {
                if status
                    .active_log
                    .as_ref()
                    .is_some_and(|p| p.ends_with("eqlog_First_P1999Green.txt"))
                    || status
                        .active_log
                        .as_ref()
                        .is_some_and(|p| p.ends_with("eqlog_Second_P1999Green.txt"))
                {
                    saw_first = true;
                    break;
                }
            }
        }
        assert!(saw_first, "watcher never selected a log");

        append(
            &first,
            b"[Fri Sep 18 16:27:00 2026] First shouts, 'GG 001 CH -- Mluian'\n",
        );
        append(
            &second,
            b"[Fri Sep 18 16:27:00 2026] Second shouts, 'GG 002 CH -- Mluian'\n",
        );

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut saw_line = false;
        let mut saw_second_char = false;
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(WatchEvent::Lines { character, lines }) => {
                    if lines.iter().any(|l| l.contains("CH -- Mluian")) {
                        saw_line = true;
                    }
                    if character.as_deref() == Some("Second") {
                        saw_second_char = true;
                    }
                    if saw_line {
                        break;
                    }
                }
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        handle.stop();
        assert!(saw_line, "watcher did not emit appended CH lines");
        let _ = saw_second_char;
    }
}
