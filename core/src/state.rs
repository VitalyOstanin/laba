//! Per-(server, user) history of work packages the user was ever assigned to.
//!
//! Used by `wp list --include-past` to show work packages from which the user
//! has since been unassigned. Ported from the Python `state.py`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Resolve the state file path, honoring (in priority order):
/// `OPENPROJECT_STATE`, `XDG_STATE_HOME/taskstream/...`, then
/// `~/.local/state/taskstream/assignee-history.json`.
fn state_path() -> PathBuf {
    if let Some(p) = std::env::var_os("OPENPROJECT_STATE") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default();
            home.join(".local").join("state")
        });
    base.join("taskstream").join("assignee-history.json")
}

fn key(base_url: &str, uid: &str) -> String {
    format!("{base_url}#{uid}")
}

fn read_map(path: &Path) -> BTreeMap<String, Vec<i64>> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => BTreeMap::new(),
    }
}

fn dedup_sorted(mut ids: Vec<i64>) -> Vec<i64> {
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Read the sorted, unique list of work-package ids the user was ever assigned
/// to on the given server. Returns an empty list when the file, or the key, is
/// absent. A corrupt file yields an empty list rather than panicking.
pub fn load(base_url: &str, uid: &str) -> Vec<i64> {
    let map = read_map(&state_path());
    map.get(&key(base_url, uid))
        .map(|ids| dedup_sorted(ids.clone()))
        .unwrap_or_default()
}

/// Best-effort merge of `ids` into the stored history for `(base_url, uid)`.
/// Reads the existing file, unions in the new ids, and writes the whole object
/// back with 2-space indent and sorted keys. Any IO error is swallowed (at most
/// a stderr warning); the signature intentionally has no `Result`.
pub fn save(base_url: &str, uid: &str, ids: &[i64]) {
    let path = state_path();
    let mut map = read_map(&path);
    let entry = map.entry(key(base_url, uid)).or_default();
    entry.extend_from_slice(ids);
    let merged = dedup_sorted(std::mem::take(entry));
    *entry = merged;

    if let Err(e) = write_map(&path, &map) {
        eprintln!("warning: could not save assignee history: {e}");
    }
}

fn write_map(path: &Path, map: &BTreeMap<String, Vec<i64>>) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
        set_dir_mode_0700(dir);
    }
    // BTreeMap serializes with sorted keys; two-space indent per the file format.
    let text = serde_json::to_string_pretty(map).map_err(std::io::Error::other)?;
    // Write to a per-process tmp file then rename, so a crash mid-write cannot
    // leave a truncated history file behind.
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(".assignee-history.json.tmp.{}", std::process::id()));
    std::fs::write(&tmp, text)?;
    set_file_mode_0600(&tmp);
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(unix)]
fn set_dir_mode_0700(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn set_dir_mode_0700(_dir: &Path) {}

#[cfg(unix)]
fn set_file_mode_0600(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_file_mode_0600(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guard that points `OPENPROJECT_STATE` at a unique temp file for the test.
    ///
    /// Env vars are process-global; the mutex serializes the state tests so they
    /// do not observe each other's `OPENPROJECT_STATE`.
    struct EnvGuard {
        _dir: tempfile::TempDir,
    }

    impl EnvGuard {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            std::env::set_var("OPENPROJECT_STATE", dir.path().join("history.json"));
            Self { _dir: dir }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var("OPENPROJECT_STATE");
        }
    }

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn save_then_load_unions() {
        let _l = lock();
        let _g = EnvGuard::new();
        save("https://op.example", "42", &[3, 1]);
        save("https://op.example", "42", &[2, 1]);
        assert_eq!(load("https://op.example", "42"), vec![1, 2, 3]);
    }

    #[test]
    fn load_missing_is_empty() {
        let _l = lock();
        let _g = EnvGuard::new();
        assert_eq!(load("https://op.example", "42"), Vec::<i64>::new());
        save("https://op.example", "42", &[5]);
        assert_eq!(load("https://op.example", "99"), Vec::<i64>::new());
    }

    #[test]
    fn corrupt_file_is_empty() {
        let _l = lock();
        let _g = EnvGuard::new();
        let path = state_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "garbage").unwrap();
        assert_eq!(load("https://op.example", "42"), Vec::<i64>::new());
    }

    #[test]
    fn distinct_keys_do_not_mix() {
        let _l = lock();
        let _g = EnvGuard::new();
        save("https://a.example", "1", &[10]);
        save("https://b.example", "1", &[20]);
        save("https://a.example", "2", &[30]);
        assert_eq!(load("https://a.example", "1"), vec![10]);
        assert_eq!(load("https://b.example", "1"), vec![20]);
        assert_eq!(load("https://a.example", "2"), vec![30]);
    }
}
