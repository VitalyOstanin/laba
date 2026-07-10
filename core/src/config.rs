use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Which tracker a server profile talks to.
///
/// Defaults to [`Backend::OpenProject`] so configs written before the field
/// existed keep working.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    #[default]
    OpenProject,
    Github,
}

impl Backend {
    /// Whether this backend tracks logged time (timelog aggregation applies).
    pub fn supports_timelog(self) -> bool {
        matches!(self, Backend::OpenProject)
    }

    /// Whether logging time supports selecting an activity type (OpenProject
    /// time-entry activities). Drives the activity picker in the log-time form.
    pub fn supports_time_activities(self) -> bool {
        matches!(self, Backend::OpenProject)
    }

    /// Whether taskstream keeps a local assignee history for this backend
    /// because the server does not reliably expose work packages the user was
    /// *previously* assigned to. OpenProject drops the assignee link on
    /// reassignment, so the "was mine" set is tracked locally and merged back in
    /// (`include_past`). GitHub search can still surface past issues, so it does
    /// not need a local history.
    pub fn needs_local_history(self) -> bool {
        matches!(self, Backend::OpenProject)
    }

    /// Default polling interval in seconds. GitHub is polled less often because
    /// `gh` shares the account's stricter API rate limit.
    pub fn default_poll_secs(self) -> u64 {
        match self {
            Backend::OpenProject => 120,
            Backend::Github => 900,
        }
    }
}

/// Semantic tint for a task row, keyed by workflow status per server.
///
/// A token (not a raw color) so it renders correctly in both the light and dark
/// GUI themes: the frontend maps each variant to a theme-aware CSS token
/// (`Danger` -> `--danger`, `Warn` -> `--warn`, `Success` -> `--ok`, `Dimmed` ->
/// `--text-dim`). A status with no mapping stays the neutral default.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StatusColor {
    Danger,
    Warn,
    Success,
    Dimmed,
}

impl StatusColor {
    /// Parse a lowercase token (`danger`/`warn`/`success`/`dimmed`), returning
    /// `None` for anything else. Used by the CLI and GUI to accept a color name.
    pub fn from_token(s: &str) -> Option<StatusColor> {
        match s {
            "danger" => Some(StatusColor::Danger),
            "warn" => Some(StatusColor::Warn),
            "success" => Some(StatusColor::Success),
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
            StatusColor::Dimmed => "dimmed",
        }
    }
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
    pub backend: Backend,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    /// Whether the server is active in the GUI. A disabled server is not polled,
    /// hidden from the dashboard, and excluded from timelog; its profile is kept
    /// so it can be re-enabled. CLI behavior is unaffected.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Poll interval in seconds. Absent (or 0) falls back to the backend default
    /// (see [`Backend::default_poll_secs`]).
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
}

impl ServerProfile {
    /// The configured tint for a task with the given status, if any.
    pub fn status_color(&self, status: &str) -> Option<StatusColor> {
        self.status_colors.get(status).copied()
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_server: Option<String>,
    #[serde(default)]
    pub servers: BTreeMap<String, ServerProfile>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, Error> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| Error::Config(format!("parse {}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(Error::Io(format!("read {}: {e}", path.display()))),
        }
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

/// Default config path: `$XDG_CONFIG_HOME/taskstream/config.json`.
pub fn default_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("taskstream").join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_returns_default() {
        let p = std::path::Path::new("/nonexistent/taskstream/config.json");
        assert_eq!(Config::load(p).unwrap(), Config::default());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut cfg = Config::default();
        cfg.default_server = Some("primary".into());
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
            },
        );
        cfg.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), cfg);
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
        assert_eq!(p.backend, Backend::OpenProject);
    }

    #[test]
    fn backend_parses_github() {
        let p: ServerProfile =
            serde_json::from_str(r#"{"base_url":"github.com","backend":"github"}"#).unwrap();
        assert_eq!(p.backend, Backend::Github);
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
        assert!(Backend::OpenProject.supports_timelog());
        assert!(!Backend::Github.supports_timelog());
        assert!(Backend::OpenProject.supports_time_activities());
        assert!(!Backend::Github.supports_time_activities());
        assert!(Backend::OpenProject.needs_local_history());
        assert!(!Backend::Github.needs_local_history());
        assert_eq!(Backend::OpenProject.default_poll_secs(), 120);
        assert_eq!(Backend::Github.default_poll_secs(), 900);
    }
}
