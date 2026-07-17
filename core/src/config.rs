use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Error;
use crate::migrate;

/// Current `config.json` schema version. Bump when a change is not backward
/// compatible and add a matching step to [`CONFIG_MIGRATIONS`].
pub const CONFIG_SCHEMA_VERSION: u32 = 2;

fn config_schema_version() -> u32 {
    CONFIG_SCHEMA_VERSION
}

/// v1 -> v2: normalize each server's `base_url` by trimming trailing slashes, so
/// the stored value matches the form the client uses (it already trims at build
/// time). Idempotent.
fn m1_normalize_base_urls(value: &mut Value) -> Result<(), Error> {
    if let Some(servers) = value.get_mut("servers").and_then(Value::as_object_mut) {
        for profile in servers.values_mut() {
            if let Some(url) = profile.get("base_url").and_then(Value::as_str) {
                let trimmed = url.trim_end_matches('/').to_owned();
                profile["base_url"] = Value::String(trimmed);
            }
        }
    }
    Ok(())
}

/// Ordered forward migrations for `config.json`. `CONFIG_MIGRATIONS[i]` migrates
/// version `BASE_VERSION + i` to the next; the length must be
/// `CONFIG_SCHEMA_VERSION - migrate::BASE_VERSION`.
const CONFIG_MIGRATIONS: &[migrate::Step] = &[m1_normalize_base_urls];

/// Which tracker a server profile talks to.
///
/// Defaults to [`BackendKind::OpenProject`] so configs written before the field
/// existed keep working.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    #[default]
    OpenProject,
    Github,
}

impl BackendKind {
    /// The full capability set for this backend. Single source of truth: the
    /// `supports_*` / `default_*` accessors below read from it, so a new backend
    /// is described in one place instead of a dozen scattered `matches!` arms.
    pub fn capabilities(self) -> Capabilities {
        match self {
            BackendKind::OpenProject => Capabilities {
                notifications: true,
                notification_read: ReadToggle::TwoWay,
                status_filters: true,
                task_detail: DetailSupport::InApp,
                custom_fields: true,
                timelog: TimelogSupport::WithActivities,
                needs_local_history: true,
                default_open_target: OpenTarget::App,
                default_poll_secs: 120,
            },
            BackendKind::Github => Capabilities {
                notifications: true,
                notification_read: ReadToggle::OneWay,
                status_filters: false,
                task_detail: DetailSupport::None,
                custom_fields: false,
                timelog: TimelogSupport::None,
                needs_local_history: false,
                default_open_target: OpenTarget::Browser,
                default_poll_secs: 900,
            },
        }
    }

    /// Whether this backend tracks logged time (timelog aggregation applies).
    pub fn supports_timelog(self) -> bool {
        self.capabilities().timelog != TimelogSupport::None
    }

    /// Whether logging time supports selecting an activity type (OpenProject
    /// time-entry activities). Drives the activity picker in the log-time form.
    pub fn supports_time_activities(self) -> bool {
        self.capabilities().timelog == TimelogSupport::WithActivities
    }

    /// Whether laba keeps a local assignee history for this backend
    /// because the server does not reliably expose work packages the user was
    /// *previously* assigned to. OpenProject drops the assignee link on
    /// reassignment, so the "was mine" set is tracked locally and merged back in
    /// (`include_past`). GitHub search can still surface past issues, so it does
    /// not need a local history.
    pub fn needs_local_history(self) -> bool {
        self.capabilities().needs_local_history
    }

    /// Whether this backend exposes a notification inbox. Both current backends
    /// do; kept as a capability so a future backend without notifications hides
    /// the column instead of showing an empty one.
    pub fn supports_notifications(self) -> bool {
        self.capabilities().notifications
    }

    /// Whether a notification's read state can be written from the app. Both
    /// backends can: OpenProject toggles read/unread both ways, while GitHub can
    /// mark read (`PATCH`/`PUT`) — its list is unread-only, so a read item leaves
    /// the list and the reverse direction is never needed. Drives the read dot and
    /// the "mark all read" control.
    pub fn supports_notification_read_toggle(self) -> bool {
        self.capabilities().notification_read != ReadToggle::None
    }

    /// Whether tasks carry a rich workflow status worth filtering by. Drives the
    /// status-filter tabs in the GUI. OpenProject work packages do; GitHub issues
    /// only have open/closed.
    pub fn supports_status_filters(self) -> bool {
        self.capabilities().status_filters
    }

    /// Whether a single task can be opened for its full description and comment
    /// thread (the task-detail screen). OpenProject exposes a work package with
    /// its description plus an activities/comments endpoint; the GitHub backend
    /// here does not fetch issue bodies or comments, so its rows only link out.
    pub fn supports_task_detail(self) -> bool {
        self.capabilities().task_detail != DetailSupport::None
    }

    /// Whether tasks carry custom fields the user can choose to show as extra
    /// list columns (`display_fields`). OpenProject work packages do; GitHub
    /// issues do not.
    pub fn supports_custom_fields(self) -> bool {
        self.capabilities().custom_fields
    }

    /// Default polling interval in seconds. GitHub is polled less often because
    /// `gh` shares the account's stricter API rate limit.
    pub fn default_poll_secs(self) -> u64 {
        self.capabilities().default_poll_secs
    }

    /// Where a task opens by default when the user has not overridden it. This
    /// encodes the quality of each backend's own web UI: OpenProject's web view
    /// is heavy, so its tasks open inside laba's detail screen; GitHub's web UI
    /// is good, so its tasks open in the browser. A per-server
    /// [`ServerProfile::open_content_in`] overrides this.
    pub fn default_open_target(self) -> OpenTarget {
        self.capabilities().default_open_target
    }
}

/// The capabilities of a backend, as data rather than a bag of boolean accessors.
/// Nuances are modeled with enums (a one-way vs two-way read toggle, timelog with
/// or without activities) instead of a boolean plus a comment. Built per backend
/// by [`BackendKind::capabilities`]; adding a backend fills one of these in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// The backend exposes a notification inbox.
    pub notifications: bool,
    /// How a notification's read state can be written (see [`ReadToggle`]).
    pub notification_read: ReadToggle,
    /// Tasks carry a workflow status worth filtering by (drives the status tabs).
    pub status_filters: bool,
    /// A task can be opened for its description and comments (see [`DetailSupport`]).
    pub task_detail: DetailSupport,
    /// Tasks carry custom fields shown as extra list columns.
    pub custom_fields: bool,
    /// Time logging support (see [`TimelogSupport`]).
    pub timelog: TimelogSupport,
    /// laba keeps a local assignee history because the server forgets past
    /// assignees.
    pub needs_local_history: bool,
    /// Where a task opens by default (per-server override notwithstanding).
    pub default_open_target: OpenTarget,
    /// Default polling interval in seconds.
    pub default_poll_secs: u64,
}

/// How a notification's read state can be written from the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReadToggle {
    /// Read state cannot be changed from the app.
    None,
    /// Only "mark read" is possible (e.g. GitHub: the list is unread-only, so a
    /// read item leaves the list and "mark unread" is never needed).
    OneWay,
    /// Read and unread can both be set (e.g. OpenProject).
    TwoWay,
}

/// Whether a task can be opened for its full description and comment thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetailSupport {
    /// No in-app detail; rows only link out to the web page.
    None,
    /// An in-app detail screen (description plus comments) is available.
    InApp,
}

/// Time-logging support for a backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimelogSupport {
    /// No time logging.
    None,
    /// Time logging without an activity type.
    Basic,
    /// Time logging with a selectable activity type (OpenProject activities).
    WithActivities,
}

/// Where a task opens on click: inside laba's detail screen (`App`) or in the
/// system browser (`Browser`). Chosen per server, defaulting by backend (see
/// [`BackendKind::default_open_target`]).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OpenTarget {
    App,
    Browser,
}

impl OpenTarget {
    /// The lowercase token passed to the GUI (`app` / `browser`).
    pub fn token(self) -> &'static str {
        match self {
            OpenTarget::App => "app",
            OpenTarget::Browser => "browser",
        }
    }
}

/// Semantic tint for a task row, keyed by workflow status per server.
///
/// A token (not a raw color) so it renders correctly in both the light and dark
/// GUI themes: the frontend maps each variant to a theme-aware CSS token
/// (`Danger` -> `--danger`, `Warn` -> `--warn`, `Success` -> `--ok`,
/// `Progress` -> `--info`, `Dimmed` -> `--text-dim`). A status with no mapping
/// stays the neutral default.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StatusColor {
    Danger,
    Warn,
    Success,
    Progress,
    Dimmed,
}

impl StatusColor {
    /// Parse a lowercase token (`danger`/`warn`/`success`/`progress`/`dimmed`),
    /// returning `None` for anything else. Used by the CLI and GUI to accept a
    /// color name.
    pub fn from_token(s: &str) -> Option<StatusColor> {
        match s {
            "danger" => Some(StatusColor::Danger),
            "warn" => Some(StatusColor::Warn),
            "success" => Some(StatusColor::Success),
            "progress" => Some(StatusColor::Progress),
            "dimmed" => Some(StatusColor::Dimmed),
            _ => None,
        }
    }

    /// The lowercase token for this color (inverse of [`from_token`]).
    pub fn token(self) -> &'static str {
        match self {
            StatusColor::Danger => "danger",
            StatusColor::Warn => "warn",
            StatusColor::Success => "success",
            StatusColor::Progress => "progress",
            StatusColor::Dimmed => "dimmed",
        }
    }
}

/// A named status filter, shown in the GUI as a task-list tab with a count.
/// `statuses` is the set of workflow statuses the filter groups (one status, or
/// a combination). Status strings are instance-specific, so this is user data
/// (kept out of code/tests, which use fictional statuses).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusFilter {
    pub label: String,
    #[serde(default)]
    pub statuses: Vec<String>,
}

/// Per-server timelog window start.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelogStart {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// True while `date` is the auto-seeded first-launch date and has not been
    /// set explicitly by the user (drives a "reconfigure me" hint).
    #[serde(default)]
    pub auto: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerProfile {
    /// Full human name shown in tooltips and the settings heading. The map key
    /// under which this profile is stored is the short name / identifier; when
    /// `display_name` is absent the key is used for display too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub base_url: String,
    #[serde(default)]
    pub backend: BackendKind,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
    /// Per-server proxy override. A proxy URL routes this server's HTTP through it;
    /// `"direct"` (also `"none"`/empty) forces a direct connection, ignoring the
    /// global default and env. Absent defers to the global [`Config::proxy`], then
    /// the ambient env, then direct. See [`crate::client::resolve_proxy`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    /// Whether the server is active in the GUI. A disabled server is not polled,
    /// hidden from the dashboard, and excluded from timelog; its profile is kept
    /// so it can be re-enabled. CLI behavior is unaffected.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Poll interval in seconds. Absent (or 0) falls back to the backend default
    /// (see [`BackendKind::default_poll_secs`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_secs: Option<u64>,
    /// Timelog window start for this server. Seeded with the first-launch date
    /// (`auto: true`) when the server is first seen, so the UI can prompt the
    /// user to set a real start date. Only meaningful for timelog-capable
    /// backends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timelog_start: Option<TimelogStart>,
    /// Per-status row tint, keyed by the exact workflow status string as it
    /// appears on this server. The status names are instance-specific, so the
    /// map is user data (kept out of code/tests, which use fictional statuses).
    /// An unlisted status renders neutral.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub status_colors: BTreeMap<String, StatusColor>,
    /// Named status filters shown as task-list tabs (label -> set of statuses).
    /// Ordered. Empty means the GUI auto-derives one tab per status present.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_filters: Vec<StatusFilter>,
    /// Custom-field names to show as extra columns in the task list (and to sort
    /// by), matched against each task's expanded `customFields[].name`. Ordered.
    /// The name is used both to look up the value and as the column label (e.g.
    /// `Rank`). Instance-specific, so this is user data. Only meaningful for
    /// backends with custom fields (see [`BackendKind::supports_custom_fields`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub display_fields: Vec<String>,
    /// Preferred place to open a task on click: `app` (laba's detail screen) or
    /// `browser`. Absent defers to the backend default (see
    /// [`BackendKind::default_open_target`]): OpenProject → app, GitHub → browser.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_content_in: Option<OpenTarget>,
}

impl ServerProfile {
    /// The configured tint for a task with the given status, if any.
    pub fn status_color(&self, status: &str) -> Option<StatusColor> {
        self.status_colors.get(status).copied()
    }

    /// Effective open target for tasks: the per-server `open_content_in` override,
    /// else the backend default. `App` is only honored when the backend can
    /// actually render a detail screen ([`BackendKind::supports_task_detail`]);
    /// otherwise it falls back to `Browser` so the title is never left inert.
    pub fn effective_open_target(&self) -> OpenTarget {
        let want = self
            .open_content_in
            .unwrap_or_else(|| self.backend.default_open_target());
        if want == OpenTarget::App && !self.backend.supports_task_detail() {
            OpenTarget::Browser
        } else {
            want
        }
    }

    /// Full display name: `display_name` if set, else the profile's map key
    /// (its short name / identifier).
    pub fn display<'a>(&'a self, key: &'a str) -> &'a str {
        self.display_name.as_deref().unwrap_or(key)
    }

    /// Effective poll interval: an explicit `poll_secs` (when > 0), else the
    /// backend default. A stored 0 is treated as unset so it can never disable
    /// polling.
    pub fn effective_poll_secs(&self) -> u64 {
        match self.poll_secs {
            Some(secs) if secs > 0 => secs,
            _ => self.backend.default_poll_secs(),
        }
    }
}

fn default_timeout() -> u64 {
    30
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Schema version, for forward migrations on load (see [`crate::migrate`]).
    #[serde(default = "config_schema_version")]
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_server: Option<String>,
    /// Global default proxy applied to every server that does not set its own.
    /// A proxy URL (`socks5://…`, `http://…`) routes all backend HTTP through it;
    /// `"direct"` (also `"none"`/empty) forces a direct connection. Absent means
    /// no global default — each server falls back to the ambient
    /// `HTTP(S)_PROXY`/`NO_PROXY` env, then direct. A per-server `proxy` and the
    /// CLI `--proxy` override still win over this. See [`crate::client::resolve_proxy`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    #[serde(default)]
    pub servers: BTreeMap<String, ServerProfile>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            schema_version: CONFIG_SCHEMA_VERSION,
            default_server: None,
            proxy: None,
            servers: BTreeMap::new(),
        }
    }
}

/// Warn (once, at load) about server profiles whose transport can expose the
/// API token: TLS verification disabled, or a non-HTTPS `base_url` on a
/// token-carrying backend. These are deliberate user choices (the safe defaults
/// stand), so they only log a warning rather than being rejected. Loopback hosts
/// and the GitHub backend (which authenticates through the `gh` CLI, not an HTTP
/// token) are exempt from the scheme check.
fn warn_insecure_servers(cfg: &Config) {
    for (name, p) in &cfg.servers {
        if !p.verify_ssl {
            log::warn!(
                "server '{name}': TLS certificate verification is disabled (verify_ssl=false); the API token can be intercepted by a man-in-the-middle"
            );
        }
        if matches!(p.backend, BackendKind::OpenProject) {
            if let Ok(url) = reqwest::Url::parse(&p.base_url) {
                let loopback = url
                    .host_str()
                    .is_some_and(|h| h == "localhost" || h.starts_with("127.") || h == "::1");
                if url.scheme() != "https" && !loopback {
                    log::warn!(
                        "server '{name}': base_url uses {}:// (not https); the API token will be sent unencrypted",
                        url.scheme()
                    );
                }
            }
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, Error> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(e) => return Err(Error::Io(format!("read {}: {e}", path.display()))),
        };
        let mut value: Value = serde_json::from_str(&text)
            .map_err(|e| Error::Config(format!("parse {}: {e}", path.display())))?;
        let from = migrate::version_of(&value);
        let migrated = migrate::run(&mut value, from, CONFIG_SCHEMA_VERSION, CONFIG_MIGRATIONS)?;
        // Deserializing the migrated JSON verifies the new shape loads before we
        // commit it to disk.
        let mut cfg: Config = serde_json::from_value(value)
            .map_err(|e| Error::Config(format!("parse {}: {e}", path.display())))?;
        cfg.schema_version = if migrated {
            CONFIG_SCHEMA_VERSION
        } else {
            from
        };
        if migrated {
            migrate::backup(path, &text, from)?;
            cfg.save(path)?;
        }
        warn_insecure_servers(&cfg);
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| Error::Io(format!("mkdir {}: {e}", dir.display())))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| Error::Internal(format!("serialize config: {e}")))?;
        std::fs::write(path, text).map_err(|e| Error::Io(format!("write {}: {e}", path.display())))
    }

    /// Resolve the active server name: explicit `--server` wins, then
    /// `OPENPROJECT_SERVER`, then `default_server`.
    pub fn resolve_server_name(&self, flag: Option<&str>) -> Result<String, Error> {
        let from_env = std::env::var("OPENPROJECT_SERVER").ok();
        let name = flag
            .map(str::to_owned)
            .or(from_env)
            .or_else(|| self.default_server.clone())
            .ok_or_else(|| {
                Error::Usage(
                    "no server selected: pass --server or set a default (server set-default)"
                        .into(),
                )
            })?;
        if !self.servers.contains_key(&name) {
            return Err(Error::Usage(format!("unknown server '{name}'")));
        }
        Ok(name)
    }
}

/// Default config path: `$XDG_CONFIG_HOME/laba/config.json`.
pub fn default_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("laba").join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_returns_default() {
        let p = std::path::Path::new("/nonexistent/laba/config.json");
        assert_eq!(Config::load(p).unwrap(), Config::default());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut cfg = Config::default();
        cfg.default_server = Some("primary".into());
        cfg.proxy = Some("http://proxy.example:8080".into());
        cfg.servers.insert(
            "primary".into(),
            ServerProfile {
                display_name: Some("Primary".into()),
                backend: Default::default(),
                base_url: "https://host.example/openproject".into(),
                timeout: 30,
                verify_ssl: true,
                proxy: Some("socks5://127.0.0.1:10808".into()),
                enabled: true,
                poll_secs: Some(300),
                timelog_start: Some(TimelogStart {
                    date: "2026-07-01".into(),
                    auto: false,
                }),
                status_colors: BTreeMap::from([
                    ("In progress".into(), StatusColor::Warn),
                    ("Under review".into(), StatusColor::Success),
                ]),
                status_filters: vec![StatusFilter {
                    label: "Active".into(),
                    statuses: vec!["In progress".into(), "Under review".into()],
                }],
                display_fields: vec!["Rank".into()],
                open_content_in: Some(OpenTarget::Browser),
            },
        );
        cfg.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), cfg);
    }

    #[test]
    fn legacy_config_migrates_base_url_and_backs_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        // A pre-versioning file (no schema_version) with a trailing slash.
        std::fs::write(
            &path,
            r#"{"servers":{"a":{"base_url":"https://h.example/op/"}}}"#,
        )
        .unwrap();

        let cfg = Config::load(&path).unwrap();
        assert_eq!(cfg.schema_version, CONFIG_SCHEMA_VERSION);
        assert_eq!(cfg.servers["a"].base_url, "https://h.example/op");

        // The original (v1) file is preserved as a backup and the migrated file
        // is rewritten stamped with the current version.
        assert!(dir.path().join("config.json.bak-v1").exists());
        let rewritten = std::fs::read_to_string(&path).unwrap();
        assert!(rewritten.contains(&format!("\"schema_version\": {CONFIG_SCHEMA_VERSION}")));

        // A second load does not migrate again.
        let reread = Config::load(&path).unwrap();
        assert_eq!(reread.schema_version, CONFIG_SCHEMA_VERSION);
        assert_eq!(reread.servers["a"].base_url, "https://h.example/op");
    }

    #[test]
    fn newer_config_is_not_downgraded() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"schema_version":99,"servers":{}}"#).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        let cfg = Config::load(&path).unwrap();
        assert_eq!(cfg.schema_version, 99);
        // The file is left untouched (never rewritten to a lower version).
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    }

    #[allow(clippy::field_reassign_with_default)]
    fn cfg_with(names: &[&str], default: Option<&str>) -> Config {
        let mut c = Config::default();
        c.default_server = default.map(str::to_owned);
        for n in names {
            c.servers.insert(
                (*n).into(),
                ServerProfile {
                    display_name: None,
                    backend: Default::default(),
                    base_url: "u".into(),
                    timeout: 30,
                    verify_ssl: true,
                    proxy: None,
                    enabled: true,
                    poll_secs: None,
                    timelog_start: None,
                    status_colors: Default::default(),
                    status_filters: Vec::new(),
                    display_fields: Vec::new(),
                    open_content_in: None,
                },
            );
        }
        c
    }

    #[test]
    fn flag_beats_default() {
        let c = cfg_with(&["a", "b"], Some("a"));
        assert_eq!(c.resolve_server_name(Some("b")).unwrap(), "b");
    }

    #[test]
    fn falls_back_to_default() {
        let c = cfg_with(&["a"], Some("a"));
        assert_eq!(c.resolve_server_name(None).unwrap(), "a");
    }

    #[test]
    fn unknown_server_is_usage_error() {
        let c = cfg_with(&["a"], None);
        assert_eq!(c.resolve_server_name(Some("x")).unwrap_err().exit_code(), 2);
    }

    #[test]
    fn backend_defaults_to_openproject_when_absent() {
        let p: ServerProfile = serde_json::from_str(r#"{"base_url":"u"}"#).unwrap();
        assert_eq!(p.backend, BackendKind::OpenProject);
    }

    #[test]
    fn backend_parses_github() {
        let p: ServerProfile =
            serde_json::from_str(r#"{"base_url":"github.com","backend":"github"}"#).unwrap();
        assert_eq!(p.backend, BackendKind::Github);
    }

    #[test]
    fn display_falls_back_to_key() {
        let p: ServerProfile = serde_json::from_str(r#"{"base_url":"u"}"#).unwrap();
        assert_eq!(p.display("MP"), "MP");
        let p: ServerProfile =
            serde_json::from_str(r#"{"base_url":"u","display_name":"Metaprime"}"#).unwrap();
        assert_eq!(p.display("MP"), "Metaprime");
    }

    #[test]
    fn server_level_fields_default_when_absent() {
        let p: ServerProfile = serde_json::from_str(r#"{"base_url":"u"}"#).unwrap();
        assert!(p.enabled);
        assert_eq!(p.poll_secs, None);
        assert_eq!(p.timelog_start, None);
        assert!(p.status_colors.is_empty());
        assert!(p.status_filters.is_empty());
        // No explicit poll_secs -> backend default.
        assert_eq!(p.effective_poll_secs(), 120);
    }

    #[test]
    fn status_colors_parse_and_look_up() {
        let p: ServerProfile = serde_json::from_str(
            r#"{"base_url":"u","status_colors":{"Blocked":"danger","Done":"dimmed"}}"#,
        )
        .unwrap();
        assert_eq!(p.status_color("Blocked"), Some(StatusColor::Danger));
        assert_eq!(p.status_color("Done"), Some(StatusColor::Dimmed));
        // Unlisted status -> neutral (no tint).
        assert_eq!(p.status_color("In progress"), None);
        // Empty map is omitted from the serialized form.
        let bare: ServerProfile = serde_json::from_str(r#"{"base_url":"u"}"#).unwrap();
        let json = serde_json::to_string(&bare).unwrap();
        assert!(
            !json.contains("status_colors"),
            "empty map must not serialize"
        );
    }

    #[test]
    fn status_filters_parse_and_omit_when_empty() {
        let p: ServerProfile = serde_json::from_str(
            r#"{"base_url":"u","status_filters":[{"label":"Active","statuses":["A","B"]}]}"#,
        )
        .unwrap();
        assert_eq!(p.status_filters.len(), 1);
        assert_eq!(p.status_filters[0].label, "Active");
        assert_eq!(p.status_filters[0].statuses, vec!["A", "B"]);
        // Empty list is omitted from the serialized form.
        let bare: ServerProfile = serde_json::from_str(r#"{"base_url":"u"}"#).unwrap();
        let json = serde_json::to_string(&bare).unwrap();
        assert!(!json.contains("status_filters"));
    }

    #[test]
    fn status_color_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&StatusColor::Success).unwrap(),
            "\"success\""
        );
    }

    #[test]
    fn status_color_token_roundtrips() {
        for c in [
            StatusColor::Danger,
            StatusColor::Warn,
            StatusColor::Success,
            StatusColor::Progress,
            StatusColor::Dimmed,
        ] {
            assert_eq!(StatusColor::from_token(c.token()), Some(c));
        }
        assert_eq!(StatusColor::from_token("bogus"), None);
    }

    #[test]
    fn effective_poll_prefers_explicit_then_backend_default() {
        let p: ServerProfile = serde_json::from_str(r#"{"base_url":"u","poll_secs":300}"#).unwrap();
        assert_eq!(p.effective_poll_secs(), 300);
        // A stored 0 is treated as unset.
        let p: ServerProfile = serde_json::from_str(r#"{"base_url":"u","poll_secs":0}"#).unwrap();
        assert_eq!(p.effective_poll_secs(), 120);
        let p: ServerProfile =
            serde_json::from_str(r#"{"base_url":"u","backend":"github"}"#).unwrap();
        assert_eq!(p.effective_poll_secs(), 900);
    }

    #[test]
    fn backend_capabilities() {
        assert!(BackendKind::OpenProject.supports_timelog());
        assert!(!BackendKind::Github.supports_timelog());
        assert!(BackendKind::OpenProject.supports_time_activities());
        assert!(!BackendKind::Github.supports_time_activities());
        assert!(BackendKind::OpenProject.needs_local_history());
        assert!(!BackendKind::Github.needs_local_history());
        assert!(BackendKind::OpenProject.supports_notifications());
        assert!(BackendKind::Github.supports_notifications());
        assert!(BackendKind::OpenProject.supports_notification_read_toggle());
        assert!(BackendKind::Github.supports_notification_read_toggle());
        assert!(BackendKind::OpenProject.supports_status_filters());
        assert!(!BackendKind::Github.supports_status_filters());
        assert_eq!(BackendKind::OpenProject.default_poll_secs(), 120);
        assert_eq!(BackendKind::Github.default_poll_secs(), 900);
    }

    #[test]
    fn default_open_target_reflects_web_ui_quality() {
        // OpenProject's heavy web UI opens in laba; GitHub's good web UI opens
        // in the browser.
        assert_eq!(BackendKind::OpenProject.default_open_target(), OpenTarget::App);
        assert_eq!(BackendKind::Github.default_open_target(), OpenTarget::Browser);
        assert_eq!(OpenTarget::App.token(), "app");
        assert_eq!(OpenTarget::Browser.token(), "browser");
    }

    #[test]
    fn capabilities_are_the_single_source_for_accessors() {
        // OpenProject: full-featured — two-way read, in-app detail, timelog with
        // activities, local history, status filters, custom fields.
        let op = BackendKind::OpenProject.capabilities();
        assert_eq!(op.notification_read, ReadToggle::TwoWay);
        assert_eq!(op.task_detail, DetailSupport::InApp);
        assert_eq!(op.timelog, TimelogSupport::WithActivities);
        assert!(op.status_filters && op.custom_fields && op.needs_local_history);

        // GitHub: one-way read, no in-app detail, no timelog, no local history.
        let gh = BackendKind::Github.capabilities();
        assert_eq!(gh.notification_read, ReadToggle::OneWay);
        assert_eq!(gh.task_detail, DetailSupport::None);
        assert_eq!(gh.timelog, TimelogSupport::None);
        assert!(!gh.status_filters && !gh.custom_fields && !gh.needs_local_history);

        // The boolean accessors must agree with the capability set they read from.
        for b in [BackendKind::OpenProject, BackendKind::Github] {
            let c = b.capabilities();
            assert_eq!(b.supports_timelog(), c.timelog != TimelogSupport::None);
            assert_eq!(
                b.supports_time_activities(),
                c.timelog == TimelogSupport::WithActivities
            );
            assert_eq!(
                b.supports_notification_read_toggle(),
                c.notification_read != ReadToggle::None
            );
            assert_eq!(
                b.supports_task_detail(),
                c.task_detail != DetailSupport::None
            );
            assert_eq!(b.default_poll_secs(), c.default_poll_secs);
            assert_eq!(b.default_open_target(), c.default_open_target);
        }
    }

    #[test]
    fn effective_open_target_falls_back_to_browser_without_detail() {
        let op = ServerProfile {
            backend: BackendKind::OpenProject,
            ..profile("https://op.example")
        };
        assert_eq!(op.effective_open_target(), OpenTarget::App);
        let gh = ServerProfile {
            backend: BackendKind::Github,
            ..profile("https://github.com")
        };
        // GitHub cannot render a detail screen, so App would degrade to Browser;
        // its default is already Browser.
        assert_eq!(gh.effective_open_target(), OpenTarget::Browser);
    }

    fn profile(base: &str) -> ServerProfile {
        ServerProfile {
            display_name: None,
            backend: BackendKind::OpenProject,
            base_url: base.into(),
            timeout: 30,
            verify_ssl: true,
            proxy: None,
            enabled: true,
            poll_secs: None,
            timelog_start: None,
            status_colors: BTreeMap::new(),
            status_filters: Vec::new(),
            display_fields: Vec::new(),
            open_content_in: None,
        }
    }
}
