//! GUI preferences persisted separately from server profiles.
//!
//! Server connection data lives in `config.json` ([`crate::config`]); this holds
//! app-level UI choices (theme, language, tray behavior, poll interval
//! overrides) that only the GUI cares about. Kept in `core` so it is testable
//! and could be reused by other frontends.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::Backend;
use crate::error::Error;

/// Color theme choice. `System` follows the OS preference.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

/// UI language choice. `System` follows the browser/OS locale.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    System,
    En,
    Ru,
}

/// First day of the week, for week-based grouping (the timelog week boundary).
///
/// Deriving this from the system locale is deferred (needs CLDR/ICU week data);
/// for now it is an explicit choice defaulting to Monday. See `TODO.md`.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WeekStart {
    #[default]
    Monday,
    Sunday,
}

impl WeekStart {
    /// The corresponding `chrono` weekday, for timelog week grouping.
    pub fn first_weekday(self) -> chrono::Weekday {
        match self {
            WeekStart::Monday => chrono::Weekday::Mon,
            WeekStart::Sunday => chrono::Weekday::Sun,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub language: Lang,
    /// Hide to tray on window close instead of quitting.
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    /// First day of the week for week-based grouping.
    #[serde(default)]
    pub week_start: WeekStart,
    /// IANA timezone name (e.g. `Europe/Moscow`) for the timelog day boundary and
    /// datetime display. Absent/empty means the machine's local zone. See
    /// [`crate::datetime::Zone`].
    #[serde(default)]
    pub timezone: Option<String>,
    /// Per-server poll interval overrides (seconds). Absent entries fall back to
    /// the server backend's default (see [`Backend::default_poll_secs`]).
    #[serde(default)]
    pub poll_override: BTreeMap<String, u64>,
    /// Per-server timelog window start (server name -> start). Each timelog-capable
    /// server has its own start date; the aggregate plan runs from the earliest.
    /// Seeded with the first-launch date (`auto: true`) when a server is first
    /// seen, so the UI can prompt the user to set a real start date.
    #[serde(default)]
    pub timelog_start: BTreeMap<String, TimelogStart>,
    /// Servers temporarily disabled in the GUI: not polled, hidden from the
    /// dashboard, and excluded from timelog. The server profile in `config.json`
    /// is kept, so the server can be re-enabled. CLI behavior is unaffected.
    #[serde(default)]
    pub disabled_servers: BTreeSet<String>,
}

impl Settings {
    /// Whether a server is enabled (not in `disabled_servers`).
    pub fn is_enabled(&self, server: &str) -> bool {
        !self.disabled_servers.contains(server)
    }
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::default(),
            language: Lang::default(),
            minimize_to_tray: true,
            week_start: WeekStart::default(),
            timezone: None,
            poll_override: BTreeMap::new(),
            timelog_start: BTreeMap::new(),
            disabled_servers: BTreeSet::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Result<Settings, Error> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| Error::Config(format!("parse {}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(e) => Err(Error::Io(format!("read {}: {e}", path.display()))),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| Error::Io(format!("mkdir {}: {e}", dir.display())))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| Error::Internal(format!("serialize settings: {e}")))?;
        std::fs::write(path, text).map_err(|e| Error::Io(format!("write {}: {e}", path.display())))
    }

    /// Effective poll interval for a server: an explicit override, else the
    /// backend default. An override of 0 is ignored (treated as unset) so a
    /// blank field in the UI cannot disable polling.
    pub fn effective_poll_secs(&self, server: &str, backend: Backend) -> u64 {
        match self.poll_override.get(server) {
            Some(&secs) if secs > 0 => secs,
            _ => backend.default_poll_secs(),
        }
    }
}

/// Default settings path: `$XDG_CONFIG_HOME/taskstream/gui-settings.json`.
pub fn default_settings_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("taskstream").join("gui-settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_returns_default() {
        let p = Path::new("/nonexistent/taskstream/gui-settings.json");
        let s = Settings::load(p).unwrap();
        assert_eq!(s, Settings::default());
        assert_eq!(s.theme, Theme::System);
        assert_eq!(s.language, Lang::System);
        assert!(s.minimize_to_tray);
        assert!(s.poll_override.is_empty());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gui-settings.json");
        let mut s = Settings {
            theme: Theme::Dark,
            language: Lang::Ru,
            minimize_to_tray: false,
            ..Settings::default()
        };
        s.poll_override.insert("work".into(), 300);
        s.timelog_start.insert(
            "work".into(),
            TimelogStart {
                date: "2026-07-01".into(),
                auto: false,
            },
        );
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), s);
    }

    #[test]
    fn effective_poll_prefers_override_then_backend_default() {
        let mut s = Settings::default();
        s.poll_override.insert("work".into(), 300);
        assert_eq!(s.effective_poll_secs("work", Backend::OpenProject), 300);
        // No override: backend defaults apply.
        assert_eq!(s.effective_poll_secs("gh", Backend::Github), 900);
        assert_eq!(s.effective_poll_secs("op", Backend::OpenProject), 120);
    }

    #[test]
    fn zero_override_is_ignored() {
        let mut s = Settings::default();
        s.poll_override.insert("work".into(), 0);
        assert_eq!(s.effective_poll_secs("work", Backend::OpenProject), 120);
    }

    #[test]
    fn week_start_defaults_to_monday_and_maps_to_weekday() {
        assert_eq!(WeekStart::default(), WeekStart::Monday);
        assert_eq!(WeekStart::Monday.first_weekday(), chrono::Weekday::Mon);
        assert_eq!(WeekStart::Sunday.first_weekday(), chrono::Weekday::Sun);
        assert_eq!(Settings::default().week_start, WeekStart::Monday);
        // Absent in older configs -> serde defaults (Monday, no timezone).
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.week_start, WeekStart::Monday);
        assert_eq!(s.timezone, None);
        assert_eq!(
            serde_json::to_string(&WeekStart::Sunday).unwrap(),
            "\"sunday\""
        );
    }

    #[test]
    fn theme_and_lang_serialize_lowercase() {
        let json = serde_json::to_string(&Theme::Dark).unwrap();
        assert_eq!(json, "\"dark\"");
        let json = serde_json::to_string(&Lang::Ru).unwrap();
        assert_eq!(json, "\"ru\"");
    }
}
