//! Thin Tauri commands wrapping `taskstream_core`. Business logic stays in core.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use taskstream_core::backend;
use taskstream_core::client::Client;
use taskstream_core::config::{default_config_path, Backend, Config, ServerProfile};
use taskstream_core::resources::{comment, notification, time};
use taskstream_core::secrets::Secrets;
use taskstream_core::settings::{default_settings_path, Settings, TimelogStart};
use taskstream_core::timelog::{self, DayCell, TimelogStatus};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ServerInfo {
    pub name: String,
    pub base_url: String,
    pub backend: String, // "openproject" | "github"
    pub is_default: bool,
    pub poll_secs: u64,
    /// Not temporarily disabled in the GUI.
    pub enabled: bool,
}

fn backend_str(b: Backend) -> &'static str {
    match b {
        Backend::OpenProject => "openproject",
        Backend::Github => "github",
    }
}

/// Build the server list for the UI switcher (pure, testable). The effective
/// poll interval reflects any per-server override in settings.
pub fn server_infos(cfg: &Config, settings: &Settings) -> Vec<ServerInfo> {
    cfg.servers
        .iter()
        .map(|(name, p)| ServerInfo {
            name: name.clone(),
            base_url: p.base_url.clone(),
            backend: backend_str(p.backend).into(),
            is_default: cfg.default_server.as_deref() == Some(name.as_str()),
            poll_secs: settings.effective_poll_secs(name, p.backend),
            enabled: settings.is_enabled(name),
        })
        .collect()
}

fn load_cfg() -> Result<Config, String> {
    Config::load(&default_config_path()).map_err(|e| e.to_string())
}

fn load_settings() -> Result<Settings, String> {
    Settings::load(&default_settings_path()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_servers() -> Result<Vec<ServerInfo>, String> {
    Ok(server_infos(&load_cfg()?, &load_settings()?))
}

#[tauri::command]
pub fn get_settings() -> Result<Settings, String> {
    load_settings()
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<(), String> {
    settings
        .save(&default_settings_path())
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelogResult {
    /// Whether at least one timelog-capable server has a valid start date. When
    /// false the rest is empty.
    pub configured: bool,
    pub status: TimelogStatus,
    pub timeline: Vec<DayCell>,
    /// Aggregate window start = earliest per-server start (`YYYY-MM-DD`, empty
    /// when unconfigured).
    pub start: String,
    /// True while any timelog-capable server's start is still the auto-seeded
    /// first-launch date, so the UI prompts the user to set real start dates.
    pub start_is_default: bool,
    /// Servers excluded from timelog because their backend has no time tracking
    /// (requirement 22), for a UI hint.
    pub excluded: Vec<String>,
}

/// Fetch one server's time entries in `[since, today]` as `(day, minutes)`
/// pairs. Timelog is best-effort across servers: an unauthenticated server, a
/// client that fails to build, or a failed request yields an empty vec (logged),
/// never an error. Only `token_for` failures propagate, aborting the command.
async fn server_time_minutes(
    name: &str,
    p: &ServerProfile,
    since: &str,
    today: &str,
) -> Result<Vec<(String, i64)>, String> {
    let token = match token_for(name, p.backend)? {
        Some(t) => t,
        None => return Ok(Vec::new()),
    };
    let client = match Client::new("", p, token, None) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("timelog skipped server '{name}': {e}");
            return Ok(Vec::new());
        }
    };
    let list = time::list_all(
        &client,
        Some("me"),
        None,
        None,
        Some(since),
        Some(today),
        false,
    )
    .await;
    let arr = match list {
        Ok(Value::Array(arr)) => arr,
        Ok(_) => return Ok(Vec::new()),
        Err(e) => {
            log::warn!("timelog fetch failed for server '{name}': {e}");
            return Ok(Vec::new());
        }
    };
    let mut out = Vec::new();
    for te in arr {
        let day = te.get("spentOn").and_then(|v| v.as_str()).unwrap_or("");
        let hours = te.get("hours").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if !day.is_empty() {
            out.push((day.to_string(), timelog::minutes_from_hours(hours)));
        }
    }
    Ok(out)
}

/// Aggregate my logged time across timelog-capable servers and compute the
/// work-log status and timeline. Each server contributes entries from its own
/// start date; the aggregate plan runs from the earliest start (requirement 21).
/// Servers whose backend has no time tracking (GitHub) are excluded
/// (requirement 22). A server first seen here is seeded with today's date as an
/// auto start so timelog has a sensible window until the user sets a real one.
/// Per-server fetch failures are skipped so one bad server does not blank the
/// whole indicator.
#[tauri::command]
pub async fn get_timelog() -> Result<TimelogResult, String> {
    let cfg = load_cfg()?;
    let mut settings = load_settings()?;
    let zone = taskstream_core::datetime::Zone::resolve(settings.timezone.as_deref());
    let today_date = zone.today();
    let today = timelog::fmt(today_date);

    let mut entries: Vec<(String, i64)> = Vec::new();
    let mut excluded: Vec<String> = Vec::new();
    let mut starts: Vec<String> = Vec::new();
    let mut any_auto = false;
    let mut dirty = false;

    for (name, p) in &cfg.servers {
        // Temporarily disabled servers are skipped entirely.
        if settings.disabled_servers.contains(name) {
            continue;
        }
        if !p.backend.supports_timelog() {
            excluded.push(name.clone());
            continue;
        }
        // Seed a start date the first time we see this server.
        let entry = settings
            .timelog_start
            .entry(name.clone())
            .or_insert_with(|| {
                dirty = true;
                TimelogStart {
                    date: today.clone(),
                    auto: true,
                }
            });
        let since = entry.date.clone();
        if entry.auto {
            any_auto = true;
        }
        if timelog::parse_date(&since).is_some() {
            starts.push(since.clone());
        }

        entries.extend(server_time_minutes(name, p, &since, &today).await?);
    }

    if dirty {
        // Best-effort: persist the seeded starts; ignore write failures.
        let _ = settings.save(&default_settings_path());
    }

    // ISO dates sort lexicographically, so the min string is the earliest start.
    let earliest = starts.iter().min().cloned();
    let first_day = settings.week_start.first_weekday();
    match earliest
        .and_then(|s| timelog::compute(&entries, &s, today_date, first_day).map(|r| (s, r)))
    {
        Some((start, (status, timeline))) => Ok(TimelogResult {
            configured: true,
            status,
            timeline,
            start,
            start_is_default: any_auto,
            excluded,
        }),
        None => Ok(TimelogResult {
            configured: false,
            status: timelog::empty_status(),
            timeline: Vec::new(),
            start: String::new(),
            start_is_default: any_auto,
            excluded,
        }),
    }
}

#[tauri::command]
pub async fn list_tasks(
    server: String,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<backend::Page, String> {
    let cfg = load_cfg()?;
    let profile = cfg
        .servers
        .get(&server)
        .ok_or_else(|| format!("unknown server '{server}'"))?
        .clone();
    let token = token_for(&server, profile.backend)?;
    backend::list_tasks_page(
        &profile,
        token.as_deref(),
        page.unwrap_or(1),
        page_size.unwrap_or(backend::PAGE_SIZE),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_notifications(
    server: String,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<backend::Page, String> {
    let cfg = load_cfg()?;
    let profile = cfg
        .servers
        .get(&server)
        .ok_or_else(|| format!("unknown server '{server}'"))?
        .clone();
    let token = token_for(&server, profile.backend)?;
    backend::list_notifications_page(
        &profile,
        token.as_deref(),
        page.unwrap_or(1),
        page_size.unwrap_or(backend::PAGE_SIZE),
    )
    .await
    .map_err(|e| e.to_string())
}

/// Build an OpenProject client for a write action. Errors clearly if the server
/// is unknown, not an OpenProject backend, or has no token.
fn op_client(server: &str) -> Result<Client, String> {
    let cfg = load_cfg()?;
    let p = cfg
        .servers
        .get(server)
        .ok_or_else(|| format!("unknown server '{server}'"))?
        .clone();
    if p.backend != Backend::OpenProject {
        return Err(format!(
            "server '{server}' backend does not support this action"
        ));
    }
    let token = token_for(server, p.backend)?.ok_or_else(|| format!("no token for '{server}'"))?;
    Client::new("", &p, token, None).map_err(|e| e.to_string())
}

/// Toggle a notification's read state (requirement 17).
#[tauri::command]
pub async fn set_notification_read(server: String, id: i64, read: bool) -> Result<(), String> {
    let client = op_client(&server)?;
    let r = if read {
        notification::read(&client, id).await
    } else {
        notification::unread(&client, id).await
    };
    r.map(|_| ()).map_err(|e| e.to_string())
}

/// Mark all notifications on a server as read (requirement 6). Returns the count.
#[tauri::command]
pub async fn mark_all_read(server: String) -> Result<u64, String> {
    let client = op_client(&server)?;
    let v = notification::read_all(&client)
        .await
        .map_err(|e| e.to_string())?;
    Ok(v.get("read").and_then(|x| x.as_u64()).unwrap_or(0))
}

/// Add a comment to a work package (requirement 4).
#[tauri::command]
pub async fn add_comment(server: String, work_package: i64, text: String) -> Result<(), String> {
    let client = op_client(&server)?;
    comment::create(&client, work_package, &text, false)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// List a server's time-entry activity types for the log-time form.
#[tauri::command]
pub async fn list_activities(server: String) -> Result<Value, String> {
    let client = op_client(&server)?;
    time::list_activities(&client)
        .await
        .map_err(|e| e.to_string())
}

/// Log time against a work package (requirement 20). `duration` accepts human
/// formats (`1h30m`, `90m`). `activity` is an optional activity type name.
#[tauri::command]
pub async fn create_time_entry(
    server: String,
    work_package: i64,
    duration: String,
    comment: Option<String>,
    activity: Option<String>,
) -> Result<(), String> {
    let client = op_client(&server)?;
    // Default spentOn to "today" in the configured zone, matching the timelog
    // day boundary (the UI does not offer a date field here).
    let zone = taskstream_core::datetime::Zone::resolve(load_settings()?.timezone.as_deref());
    let spent = timelog::fmt(zone.today());
    time::create(
        &client,
        work_package,
        None,
        Some(&duration),
        Some(&spent),
        comment.as_deref(),
        activity.as_deref(),
        false,
    )
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Rank my tasks across timelog-capable, enabled servers by least logged time in
/// each server's window, to surface under-logged tasks for time entry
/// (requirements 15/19).
#[tauri::command]
pub async fn pick_candidates() -> Result<Vec<timelog::Candidate>, String> {
    let cfg = load_cfg()?;
    let settings = load_settings()?;
    let today = timelog::fmt(timelog::today_local());
    let mut candidates: Vec<timelog::Candidate> = Vec::new();

    for (name, p) in &cfg.servers {
        if settings.disabled_servers.contains(name) || !p.backend.supports_timelog() {
            continue;
        }
        let since = settings
            .timelog_start
            .get(name)
            .map(|t| t.date.clone())
            .unwrap_or_else(|| today.clone());
        let token = match token_for(name, p.backend)? {
            Some(t) => t,
            None => continue,
        };
        let client = match Client::new("", p, token.clone(), None) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Logged minutes per work package in the window.
        let mut logged: HashMap<i64, i64> = HashMap::new();
        if let Ok(Value::Array(tes)) = time::list(
            &client,
            Some("me"),
            None,
            None,
            Some(&since),
            Some(&today),
            1,
            None,
            false,
        )
        .await
        {
            for te in &tes {
                if let (Some(wp), Some(h)) = (
                    te.get("workPackageId").and_then(|v| v.as_i64()),
                    te.get("hours").and_then(|v| v.as_f64()),
                ) {
                    *logged.entry(wp).or_insert(0) += timelog::minutes_from_hours(h);
                }
            }
        }

        let tasks = match backend::list_tasks(p, Some(&token)).await {
            Ok(t) => t,
            Err(_) => continue,
        };
        for t in &tasks {
            if let Some(wp) = t.get("id").and_then(|v| v.as_i64()) {
                candidates.push(timelog::Candidate {
                    server: name.clone(),
                    wp_id: wp,
                    subject: t
                        .get("subject")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    logged_min: *logged.get(&wp).unwrap_or(&0),
                });
            }
        }
    }

    Ok(timelog::rank_candidates(candidates))
}

/// OpenProject servers need a token from the keyring/file secret store; GitHub
/// uses `gh` and needs none.
fn token_for(server: &str, backend: Backend) -> Result<Option<String>, String> {
    match backend {
        Backend::Github => Ok(None),
        Backend::OpenProject => {
            let secrets = Secrets::new(Secrets::default_fallback_path());
            secrets.get(server).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use taskstream_core::config::ServerProfile;

    fn cfg() -> Config {
        let mut servers = BTreeMap::new();
        servers.insert(
            "work".into(),
            ServerProfile {
                base_url: "https://op.example".into(),
                backend: Backend::OpenProject,
                timeout: 30,
                verify_ssl: true,
                proxy: None,
            },
        );
        servers.insert(
            "gh".into(),
            ServerProfile {
                base_url: "github.com".into(),
                backend: Backend::Github,
                timeout: 30,
                verify_ssl: true,
                proxy: None,
            },
        );
        Config {
            default_server: Some("work".into()),
            servers,
        }
    }

    #[test]
    fn server_infos_lists_all_with_backend_and_default() {
        let infos = server_infos(&cfg(), &Settings::default());
        assert_eq!(infos.len(), 2);
        let work = infos.iter().find(|i| i.name == "work").unwrap();
        assert_eq!(work.backend, "openproject");
        assert!(work.is_default);
        assert_eq!(work.poll_secs, 120);
        assert!(work.enabled);
        let gh = infos.iter().find(|i| i.name == "gh").unwrap();
        assert_eq!(gh.backend, "github");
        assert!(!gh.is_default);
        assert_eq!(gh.poll_secs, 900);
    }

    #[test]
    fn server_infos_apply_poll_override() {
        let mut settings = Settings::default();
        settings.poll_override.insert("work".into(), 300);
        let infos = server_infos(&cfg(), &settings);
        let work = infos.iter().find(|i| i.name == "work").unwrap();
        assert_eq!(work.poll_secs, 300);
        // Non-overridden server keeps its backend default.
        let gh = infos.iter().find(|i| i.name == "gh").unwrap();
        assert_eq!(gh.poll_secs, 900);
    }

    #[test]
    fn server_infos_mark_disabled() {
        let mut settings = Settings::default();
        settings.disabled_servers.insert("gh".into());
        let infos = server_infos(&cfg(), &settings);
        assert!(infos.iter().find(|i| i.name == "work").unwrap().enabled);
        assert!(!infos.iter().find(|i| i.name == "gh").unwrap().enabled);
    }
}
