//! GUI preferences persisted separately from server profiles.
//!
//! Server connection data lives in `config.json` ([`crate::config`]); this holds
//! app-level UI choices (theme, language, tray behavior, poll interval
//! overrides) that only the GUI cares about. Kept in `core` so it is testable
//! and could be reused by other frontends.

use std::collections::BTreeMap;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub language: Lang,
    /// Hide to tray on window close instead of quitting.
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    /// Per-server poll interval overrides (seconds). Absent entries fall back to
    /// the server backend's default (see [`Backend::default_poll_secs`]).
    #[serde(default)]
    pub poll_override: BTreeMap<String, u64>,
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
            poll_override: BTreeMap::new(),
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
        let mut s = Settings::default();
        s.theme = Theme::Dark;
        s.language = Lang::Ru;
        s.minimize_to_tray = false;
        s.poll_override.insert("work".into(), 300);
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
    fn theme_and_lang_serialize_lowercase() {
        let json = serde_json::to_string(&Theme::Dark).unwrap();
        assert_eq!(json, "\"dark\"");
        let json = serde_json::to_string(&Lang::Ru).unwrap();
        assert_eq!(json, "\"ru\"");
    }
}
