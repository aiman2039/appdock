//! Append-only attachment lifecycle log. Failures never affect docking.
use crate::model::{ClosedInfo, TabId};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_BYTES: u64 = 1_048_576;
const RESUME_IN_PROGRESS: &str = "Resuming docking";
static WRITE: Mutex<()> = Mutex::new(());

pub fn directory() -> PathBuf {
    if let Some(dir) = std::env::var_os("APPDOCK_DATA_DIR") {
        return PathBuf::from(dir);
    }
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join("Library/Logs/AppDock"),
        None => PathBuf::from("appdock-logs"),
    }
}

pub fn path() -> PathBuf {
    directory().join("events.jsonl")
}

pub fn emit(event: &str, fields: Value) {
    if cfg!(test) {
        return;
    }
    let _ = write_event(&path(), event, fields);
}

pub fn pause_transition(previous: Option<&str>, next: Option<&str>) {
    match next {
        None if previous.is_some() => emit("resumed", json!({})),
        Some(reason) if reason != RESUME_IN_PROGRESS && previous != Some(reason) => {
            emit("paused", json!({"reason": reason}));
        }
        _ => {}
    }
}

pub fn window_closed(info: &ClosedInfo, tab: Option<TabId>, tab_bundle: Option<&str>) {
    emit(
        "window_closed",
        json!({
            "tab": tab,
            "tab_bundle": tab_bundle,
            "window": info.window,
            "pid": info.pid,
            "bundle": info.bundle,
            "process_exited": info.process_exited,
            "ax_window_count": info.ax_window_count,
            "exact": info.exact,
            "identifier": info.identifier,
            "owners": info.owners,
            "reason": info.reason,
        }),
    );
}

pub(crate) fn write_event(path: &Path, event: &str, fields: Value) -> std::io::Result<()> {
    write_event_limited(path, event, fields, MAX_BYTES)
}

fn write_event_limited(
    path: &Path,
    event: &str,
    fields: Value,
    max_bytes: u64,
) -> std::io::Result<()> {
    let mut record = match fields {
        Value::Object(map) => map,
        other => {
            let mut map = serde_json::Map::new();
            map.insert("detail".into(), other);
            map
        }
    };
    record.insert("ts".into(), json!(timestamp()));
    record.insert("pid".into(), json!(std::process::id()));
    record.insert("event".into(), json!(event));
    let mut line = serde_json::to_string(&Value::Object(record))?;
    line.push('\n');
    let _guard = WRITE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    rotate_if_needed(path, max_bytes);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())
}

fn rotate_if_needed(path: &Path, max_bytes: u64) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if meta.len() < max_bytes {
        return;
    }
    let rotated = path.with_extension("jsonl.1");
    let _ = fs::rename(path, rotated);
}

fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    datetime_utc(secs)
}

fn datetime_utc(secs: u64) -> String {
    let days = secs / 86_400;
    let clock = secs % 86_400;
    let (year, month, day) = civil_from_unix_days(days);
    let hour = clock / 3_600;
    let minute = (clock % 3_600) / 60;
    let second = clock % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_unix_days(unix_days: u64) -> (i32, u32, u32) {
    let z = i64::try_from(unix_days)
        .unwrap_or(i64::MAX)
        .saturating_add(719_468);
    let era = if z >= 0 { z } else { z.saturating_sub(146_096) }.div_euclid(146_097);
    let doe = (z.saturating_sub(era.saturating_mul(146_097))) as u32;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year =
        i32::try_from(i64::from(yoe).saturating_add(era.saturating_mul(400))).unwrap_or(i32::MAX);
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year = year.saturating_add(1);
    }
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_log(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "appdock-event-log-{name}-{}.jsonl",
            std::process::id()
        ))
    }

    fn cleanup(path: &Path) {
        let rotated = path.with_extension("jsonl.1");
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(rotated);
    }

    #[test]
    fn datetime_utc_formats_known_unix_times() {
        assert_eq!(datetime_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(datetime_utc(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn write_event_appends_json_lines() {
        let path = temp_log("append");
        cleanup(&path);
        write_event(
            &path,
            "attached",
            json!({"tab": 1, "bundle": "dev.appdock.test"}),
        )
        .unwrap();
        write_event(&path, "released", json!({"tab": 1})).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        cleanup(&path);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        let first: Value = serde_json::from_str(lines[0]).unwrap();
        let second: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first["event"], "attached");
        assert_eq!(first["tab"], 1);
        assert_eq!(first["bundle"], "dev.appdock.test");
        assert!(first["ts"].as_str().unwrap().ends_with('Z'));
        assert_eq!(second["event"], "released");
        assert_eq!(first["pid"], second["pid"]);
    }

    #[test]
    fn write_event_rotates_oversized_file() {
        let path = temp_log("rotate");
        cleanup(&path);
        write_event_limited(&path, "attached", json!({"pad": "x".repeat(120)}), 80).unwrap();
        write_event_limited(&path, "attached", json!({"n": 2}), 80).unwrap();
        let rotated = path.with_extension("jsonl.1");
        assert!(rotated.exists(), "oversized log should rotate");
        let current = fs::read_to_string(&path).unwrap();
        let archived = fs::read_to_string(&rotated).unwrap();
        cleanup(&path);
        assert!(current.contains("\"n\":2"));
        assert!(archived.contains("\"pad\""));
        assert!(!current.contains("\"pad\""));
    }

    #[test]
    fn pause_transition_skips_resume_in_progress() {
        pause_transition(None, Some("Desktop changed"));
        pause_transition(Some("Desktop changed"), Some(RESUME_IN_PROGRESS));
        pause_transition(Some(RESUME_IN_PROGRESS), None);
    }
}
