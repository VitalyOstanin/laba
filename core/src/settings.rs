//! GUI preferences persisted separately from server profiles.
//!
//! Server connection data lives in `config.json` ([`crate::config`]); this holds
//! app-level UI choices (theme, language, tray behavior, poll interval
//! overrides) that only the GUI cares about. Kept in `core` so it is testable
//! and could be reused by other frontends.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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
/// `System` follows the machine's locale (best-effort; see
/// [`system_first_weekday`]). `Monday`/`Sunday` force a choice.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WeekStart {
    #[default]
    System,
    Monday,
    Sunday,
}

impl WeekStart {
    /// The corresponding `chrono` weekday, for timelog week grouping.
    pub fn first_weekday(self) -> chrono::Weekday {
        match self {
            WeekStart::System => system_first_weekday(),
            WeekStart::Monday => chrono::Weekday::Mon,
            WeekStart::Sunday => chrono::Weekday::Sun,
        }
    }
}

/// First day of the week for the machine's locale, best-effort. The country is
/// read from `LC_ALL`/`LC_TIME`/`LANG` (e.g. `ru_RU.UTF-8` -> `RU`); a small set
/// of Sunday-first countries maps to Sunday, everything else to Monday. True
/// locale week data needs CLDR/ICU and is out of scope.
fn system_first_weekday() -> chrono::Weekday {
    // Common Sunday-first countries (North America, much of Latin America, East
    // Asia, and parts of the Middle East). Not exhaustive; the rest fall to Monday.
    const SUNDAY_FIRST: &[&str] = &[
        "US", "CA", "JP", "CN", "KR", "TW", "HK", "IN", "IL", "BR", "MX", "PH", "ZA", "CO", "AR",
        "PE", "VE", "SA", "AE", "EG", "TH", "ID",
    ];
    let country = ["LC_ALL", "LC_TIME", "LANG"].iter().find_map(|var| {
        let v = std::env::var(var).ok()?;
        // "ru_RU.UTF-8" -> take before '.', then the part after '_'.
        let base = v.split('.').next().unwrap_or(&v);
        let (_, c) = base.split_once('_')?;
        (!c.is_empty()).then(|| c.to_ascii_uppercase())
    });
    match country {
        Some(c) if SUNDAY_FIRST.contains(&c.as_str()) => chrono::Weekday::Sun,
        _ => chrono::Weekday::Mon,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// datetime display. `"system"` (or any unresolvable value) means the
    /// machine's local zone. See [`crate::datetime::Zone`].
    ///
    /// Deserialized leniently: a `null` or empty value (older configs stored the
    /// system zone as `null`) maps to the `"system"` sentinel.
    #[serde(default = "default_timezone", deserialize_with = "de_timezone")]
    pub timezone: String,
    /// Interface scale as a factor (`1.0` = no scaling). Applied by the GUI to the
    /// root font size. Clamp with [`clamp_ui_scale`] before use.
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f64,
}

fn default_true() -> bool {
    true
}

/// Default timezone sentinel: the machine's local zone.
pub const DEFAULT_TIMEZONE: &str = "system";

fn default_timezone() -> String {
    DEFAULT_TIMEZONE.to_owned()
}

/// Deserialize `timezone`, accepting `null`/empty (older configs) as the
/// `"system"` sentinel so a legacy `gui-settings.json` still loads.
fn de_timezone<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(d)?;
    Ok(opt
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(default_timezone))
}

/// Default interface scale factor (no scaling).
pub const DEFAULT_UI_SCALE: f64 = 1.0;
/// Smallest / largest interface scale factor accepted.
pub const MIN_UI_SCALE: f64 = 0.5;
pub const MAX_UI_SCALE: f64 = 2.0;

fn default_ui_scale() -> f64 {
    DEFAULT_UI_SCALE
}

/// Clamp an interface scale factor to `[MIN_UI_SCALE, MAX_UI_SCALE]`, mapping a
/// non-finite or 0 value (an absent/blank input) back to the default so the UI
/// can never shrink to nothing.
pub fn clamp_ui_scale(scale: f64) -> f64 {
    if !scale.is_finite() || scale == 0.0 {
        DEFAULT_UI_SCALE
    } else {
        scale.clamp(MIN_UI_SCALE, MAX_UI_SCALE)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::default(),
            language: Lang::default(),
            minimize_to_tray: true,
            week_start: WeekStart::default(),
            timezone: default_timezone(),
            ui_scale: DEFAULT_UI_SCALE,
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
        assert_eq!(s.week_start, WeekStart::System);
        assert_eq!(s.timezone, DEFAULT_TIMEZONE);
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gui-settings.json");
        let s = Settings {
            theme: Theme::Dark,
            language: Lang::Ru,
            minimize_to_tray: false,
            week_start: WeekStart::Sunday,
            timezone: "Europe/Moscow".into(),
            ui_scale: 1.25,
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), s);
    }

    #[test]
    fn week_start_defaults_to_system_and_maps_to_weekday() {
        assert_eq!(WeekStart::default(), WeekStart::System);
        assert_eq!(WeekStart::Monday.first_weekday(), chrono::Weekday::Mon);
        assert_eq!(WeekStart::Sunday.first_weekday(), chrono::Weekday::Sun);
        assert_eq!(Settings::default().week_start, WeekStart::System);
        // Absent in older configs -> serde defaults (system week, system timezone).
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.week_start, WeekStart::System);
        assert_eq!(s.timezone, DEFAULT_TIMEZONE);
        assert_eq!(
            serde_json::to_string(&WeekStart::System).unwrap(),
            "\"system\""
        );
        assert_eq!(
            serde_json::to_string(&WeekStart::Sunday).unwrap(),
            "\"sunday\""
        );
    }

    #[test]
    fn system_first_weekday_reads_country() {
        // Force a known locale via LC_ALL. This mutates process env, so keep it
        // in one test and restore afterward.
        let prev = std::env::var("LC_ALL").ok();
        std::env::set_var("LC_ALL", "en_US.UTF-8");
        assert_eq!(system_first_weekday(), chrono::Weekday::Sun);
        std::env::set_var("LC_ALL", "ru_RU.UTF-8");
        assert_eq!(system_first_weekday(), chrono::Weekday::Mon);
        match prev {
            Some(v) => std::env::set_var("LC_ALL", v),
            None => std::env::remove_var("LC_ALL"),
        }
    }

    #[test]
    fn timezone_accepts_legacy_null_and_empty() {
        // Older gui-settings.json stored the system zone as null.
        let s: Settings = serde_json::from_str(r#"{"timezone": null}"#).unwrap();
        assert_eq!(s.timezone, DEFAULT_TIMEZONE);
        let s: Settings = serde_json::from_str(r#"{"timezone": ""}"#).unwrap();
        assert_eq!(s.timezone, DEFAULT_TIMEZONE);
        let s: Settings = serde_json::from_str(r#"{"timezone": "Europe/Moscow"}"#).unwrap();
        assert_eq!(s.timezone, "Europe/Moscow");
    }

    #[test]
    fn ui_scale_defaults_and_clamps() {
        assert_eq!(Settings::default().ui_scale, DEFAULT_UI_SCALE);
        // Absent in older configs -> serde default.
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.ui_scale, DEFAULT_UI_SCALE);
        assert_eq!(clamp_ui_scale(0.0), DEFAULT_UI_SCALE);
        assert_eq!(clamp_ui_scale(f64::NAN), DEFAULT_UI_SCALE);
        assert_eq!(clamp_ui_scale(0.1), MIN_UI_SCALE);
        assert_eq!(clamp_ui_scale(10.0), MAX_UI_SCALE);
        assert_eq!(clamp_ui_scale(1.25), 1.25);
    }

    #[test]
    fn theme_and_lang_serialize_lowercase() {
        let json = serde_json::to_string(&Theme::Dark).unwrap();
        assert_eq!(json, "\"dark\"");
        let json = serde_json::to_string(&Lang::Ru).unwrap();
        assert_eq!(json, "\"ru\"");
    }
}
