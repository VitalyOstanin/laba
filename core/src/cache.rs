//! Two-tier per-server cache for stable OpenProject entities.
//!
//! The cache is scoped to a single server profile and layers an in-memory map
//! (lazily loaded, lives for the process) on top of a JSON file (survives
//! restarts). It stores three categories of rarely-changing lookups: resolved
//! `name -> id` of enumerations, `user id -> name`, and form `schema -> custom
//! field names`. Entries expire after [`CACHE_TTL_SECS`].
//!
//! The cache is a best-effort optimization: file IO failures during writes are
//! logged to stderr and swallowed, and a corrupt file is treated as empty. Only
//! the explicit `clear_*` operations surface IO errors.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Entries older than one week are treated as cache misses.
pub const CACHE_TTL_SECS: u64 = 7 * 24 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserEntry {
    name: Option<String>,
    ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResolveEntry {
    id: String,
    ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchemaEntry {
    fields: HashMap<String, String>,
    ts: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CacheData {
    #[serde(default)]
    users: HashMap<String, UserEntry>,
    #[serde(default)]
    resolve: HashMap<String, ResolveEntry>,
    #[serde(default)]
    schemas: HashMap<String, SchemaEntry>,
}

/// Per-server two-tier cache of stable entities.
#[derive(Debug)]
pub struct Cache {
    server: String,
    path: PathBuf,
    /// `None` until the first access lazily loads the file.
    mem: Mutex<Option<CacheData>>,
}

impl Cache {
    /// Build a cache handle for `server`. The backing file is not read yet.
    pub fn for_server(server: &str) -> Cache {
        let path = server_cache_path(server);
        Cache {
            server: server.to_string(),
            path,
            mem: Mutex::new(None),
        }
    }

    pub fn get_user(&self, id: i64) -> Option<Option<String>> {
        let mut guard = self.mem.lock().unwrap();
        let data = self.ensure_loaded(&mut guard);
        let e = data.users.get(&id.to_string())?;
        if is_fresh(e.ts) {
            Some(e.name.clone())
        } else {
            None
        }
    }

    pub fn put_user(&self, id: i64, name: Option<String>) {
        let mut guard = self.mem.lock().unwrap();
        let data = self.ensure_loaded(&mut guard);
        data.users
            .insert(id.to_string(), UserEntry { name, ts: now() });
        self.persist(data);
    }

    pub fn get_resolve(&self, kind: &str, name: &str) -> Option<String> {
        let mut guard = self.mem.lock().unwrap();
        let data = self.ensure_loaded(&mut guard);
        let e = data.resolve.get(&resolve_key(kind, name))?;
        if is_fresh(e.ts) {
            Some(e.id.clone())
        } else {
            None
        }
    }

    pub fn put_resolve(&self, kind: &str, name: &str, id: &str) {
        let mut guard = self.mem.lock().unwrap();
        let data = self.ensure_loaded(&mut guard);
        data.resolve.insert(
            resolve_key(kind, name),
            ResolveEntry {
                id: id.to_string(),
                ts: now(),
            },
        );
        self.persist(data);
    }

    pub fn get_schema(&self, href: &str) -> Option<HashMap<String, String>> {
        let mut guard = self.mem.lock().unwrap();
        let data = self.ensure_loaded(&mut guard);
        let e = data.schemas.get(href)?;
        if is_fresh(e.ts) {
            Some(e.fields.clone())
        } else {
            None
        }
    }

    pub fn put_schema(&self, href: &str, fields: HashMap<String, String>) {
        let mut guard = self.mem.lock().unwrap();
        let data = self.ensure_loaded(&mut guard);
        data.schemas
            .insert(href.to_string(), SchemaEntry { fields, ts: now() });
        self.persist(data);
    }

    /// Delete the cache file of a single server. Missing file is `Ok`.
    pub fn clear_server(server: &str) -> Result<(), Error> {
        let dir = server_cache_path(server);
        let dir = dir.parent().unwrap_or(Path::new("."));
        match std::fs::remove_dir_all(dir) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::Io(format!("clear cache for {server}: {e}"))),
        }
    }

    /// Delete the whole cache base directory. Missing directory is `Ok`.
    pub fn clear_all() -> Result<(), Error> {
        let base = cache_base_dir();
        match std::fs::remove_dir_all(&base) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::Io(format!("clear cache: {e}"))),
        }
    }

    /// Lazily load the file into `mem` on first access. A corrupt or unreadable
    /// file yields an empty cache (with a stderr warning), never a panic.
    fn ensure_loaded<'g>(&self, guard: &'g mut Option<CacheData>) -> &'g mut CacheData {
        if guard.is_none() {
            *guard = Some(self.load_file());
        }
        guard.as_mut().unwrap()
    }

    fn load_file(&self) -> CacheData {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("warning: ignoring corrupt cache for {}: {e}", self.server);
                    CacheData::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => CacheData::default(),
            Err(e) => {
                eprintln!("warning: cannot read cache for {}: {e}", self.server);
                CacheData::default()
            }
        }
    }

    /// Atomically rewrite the file. Best-effort: errors are logged, not returned.
    fn persist(&self, data: &CacheData) {
        if let Err(e) = self.persist_inner(data) {
            eprintln!("warning: cannot write cache for {}: {e}", self.server);
        }
    }

    fn persist_inner(&self, data: &CacheData) -> std::io::Result<()> {
        let dir = self
            .path
            .parent()
            .ok_or_else(|| std::io::Error::other("cache path has no parent"))?;
        std::fs::create_dir_all(dir)?;
        set_dir_mode_0700(dir);
        let text = serde_json::to_string(data)?;
        let tmp = dir.join(format!(".cache.json.tmp.{}", std::process::id()));
        std::fs::write(&tmp, text)?;
        set_file_mode_0600(&tmp);
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

fn resolve_key(kind: &str, name: &str) -> String {
    format!("{kind}:{}", name.to_lowercase())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn is_fresh(ts: u64) -> bool {
    now().saturating_sub(ts) <= CACHE_TTL_SECS
}

/// Cache base directory: `$OPENPROJECT_CACHE`, else
/// `$XDG_CACHE_HOME/laboro`, else `~/.cache/laboro`.
fn cache_base_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("OPENPROJECT_CACHE") {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from(".cache"));
    base.join("laboro")
}

fn server_cache_path(server: &str) -> PathBuf {
    cache_base_dir().join(server).join("cache.json")
}

#[cfg(unix)]
fn set_dir_mode_0700(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn set_dir_mode_0700(_path: &Path) {}

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

    /// Point the cache base at a fresh temp dir for this test process.
    /// nextest runs each test in its own process, so the global env var is safe.
    fn isolate() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("OPENPROJECT_CACHE", dir.path());
        dir
    }

    #[test]
    fn user_roundtrip_including_none_name() {
        let _t = isolate();
        let c = Cache::for_server("srv-user");
        c.put_user(42, Some("Ivan".into()));
        c.put_user(7, None);
        assert_eq!(c.get_user(42), Some(Some("Ivan".to_string())));
        assert_eq!(c.get_user(7), Some(None));
        assert_eq!(c.get_user(999), None);
    }

    #[test]
    fn resolve_is_case_insensitive_on_name() {
        let _t = isolate();
        let c = Cache::for_server("srv-resolve");
        c.put_resolve("status", "New", "1");
        assert_eq!(c.get_resolve("status", "new").as_deref(), Some("1"));
        assert_eq!(c.get_resolve("status", "NEW").as_deref(), Some("1"));
        assert_eq!(c.get_resolve("status", "closed"), None);
        // kind is not normalized.
        assert_eq!(c.get_resolve("Status", "new"), None);
    }

    #[test]
    fn schema_roundtrip() {
        let _t = isolate();
        let c = Cache::for_server("srv-schema");
        let mut fields = HashMap::new();
        fields.insert("customField1".to_string(), "Priority".to_string());
        fields.insert("customField2".to_string(), "Sprint".to_string());
        c.put_schema("/api/v3/work_packages/schemas/1-1", fields.clone());
        assert_eq!(
            c.get_schema("/api/v3/work_packages/schemas/1-1"),
            Some(fields)
        );
        assert_eq!(c.get_schema("/other"), None);
    }

    #[test]
    fn stale_entry_is_a_miss() {
        let _t = isolate();
        let server = "srv-ttl";
        // Write a file by hand with a timestamp far in the past.
        let path = server_cache_path(server);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let old = now() - CACHE_TTL_SECS - 100;
        let json = format!(
            r#"{{"users":{{"42":{{"name":"Ivan","ts":{old}}}}},"resolve":{{"status:new":{{"id":"1","ts":{old}}}}},"schemas":{{"h":{{"fields":{{"customField1":"P"}},"ts":{old}}}}}}}"#
        );
        std::fs::write(&path, json).unwrap();

        let c = Cache::for_server(server);
        assert_eq!(c.get_user(42), None);
        assert_eq!(c.get_resolve("status", "new"), None);
        assert_eq!(c.get_schema("h"), None);
    }

    #[test]
    fn corrupt_file_yields_empty_cache() {
        let _t = isolate();
        let server = "srv-corrupt";
        let path = server_cache_path(server);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "garbage").unwrap();

        let c = Cache::for_server(server);
        assert_eq!(c.get_user(1), None);
        // Still usable after corruption.
        c.put_user(1, Some("X".into()));
        assert_eq!(c.get_user(1), Some(Some("X".to_string())));
    }

    #[test]
    fn clear_server_removes_file() {
        let _t = isolate();
        let server = "srv-clear";
        let c = Cache::for_server(server);
        c.put_user(1, Some("X".into()));
        let path = server_cache_path(server);
        assert!(path.exists());

        Cache::clear_server(server).unwrap();
        assert!(!path.exists());
        // Missing is Ok.
        Cache::clear_server(server).unwrap();

        let c2 = Cache::for_server(server);
        assert_eq!(c2.get_user(1), None);
    }

    #[test]
    fn distinct_servers_use_distinct_files() {
        let _t = isolate();
        let a = Cache::for_server("srv-a");
        let b = Cache::for_server("srv-b");
        a.put_user(1, Some("Alice".into()));
        b.put_user(1, Some("Bob".into()));
        assert_eq!(a.get_user(1), Some(Some("Alice".to_string())));
        assert_eq!(b.get_user(1), Some(Some("Bob".to_string())));
        assert_ne!(server_cache_path("srv-a"), server_cache_path("srv-b"));
    }
}
