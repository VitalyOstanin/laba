//! Thin Tauri commands wrapping `laba_core`. Business logic stays in core.

use laba_core::auth::login_and_store;
use laba_core::backend;
use laba_core::client::Client;
use laba_core::config::{
    default_config_path, BackendKind, Capabilities, Config, OpenTarget, ServerProfile, StatusColor,
    StatusFilter, TimelogStart,
};
use laba_core::resources::{comment, time, work_packages};
use laba_core::secrets::Secrets;
use laba_core::settings::{default_settings_path, Settings};
use laba_core::timelog::{self, DayCell, TimelogStatus};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ServerInfo {
    /// Short name / identifier (the profile's map key), shown in the switcher.
    pub name: String,
    /// Full display name (`display_name`, or the key when unset).
    pub display_name: String,
    pub base_url: String,
    pub backend: String, // "openproject" | "github"
    pub is_default: bool,
    /// Effective poll interval (override or backend default), for display.
    pub poll_secs: u64,
    /// Raw poll-interval override (`None` = use the backend default), for the
    /// settings input value.
    pub poll_override: Option<u64>,
    /// Not disabled in the GUI.
    pub enabled: bool,
    /// Timelog window start, for the settings input.
    pub timelog_start: Option<TimelogStart>,
    /// Per-status row tint tokens (status -> `danger`/`warn`/`success`/`dimmed`),
    /// for tinting task rows and editing in settings.
    pub status_colors: BTreeMap<String, String>,
    /// Static backend capabilities (notifications, read toggle, status filters,
    /// task detail, custom fields, timelog, ...). Nested so the frontend reads one
    /// object instead of a growing list of flat `supports_*` booleans.
    pub capabilities: Capabilities,
    /// Named status filters (label -> statuses) shown as task-list tabs.
    pub status_filters: Vec<StatusFilter>,
    /// Where a task opens on click: `app` (laba detail screen) or `browser`.
    /// Effective value (per-server override or backend default).
    pub open_content_in: String,
    /// Custom-field names shown as extra task-list columns (and sort keys).
    pub display_fields: Vec<String>,
    /// Per-server proxy override (URL, `"direct"`, or absent = inherit global/env).
    pub proxy: Option<String>,
    /// Whether an OpenProject token is stored for this server (drives the
    /// "sign in / update token" control). Always `false` for GitHub (uses `gh`).
    /// Filled by `list_servers`; the pure `server_infos` builder leaves it
    /// `false` to avoid keyring I/O in tests.
    pub has_token: bool,
}

fn backend_str(b: BackendKind) -> &'static str {
    match b {
        BackendKind::OpenProject => "openproject",
        BackendKind::Github => "github",
    }
}

/// Build the server list for the UI switcher (pure, testable). All server-level
/// state (display name, enabled, effective poll interval) comes from the profile.
pub fn server_infos(cfg: &Config) -> Vec<ServerInfo> {
    cfg.servers
        .iter()
        .map(|(name, p)| ServerInfo {
            name: name.clone(),
            display_name: p.display(name).to_owned(),
            base_url: p.base_url.clone(),
            backend: backend_str(p.backend).into(),
            is_default: cfg.default_server.as_deref() == Some(name.as_str()),
            poll_secs: p.effective_poll_secs(),
            poll_override: p.poll_secs,
            enabled: p.enabled,
            timelog_start: p.timelog_start.clone(),
            status_colors: p
                .status_colors
                .iter()
                .map(|(status, color)| (status.clone(), color.token().to_owned()))
                .collect(),
            capabilities: p.backend.capabilities(),
            status_filters: p.status_filters.clone(),
            open_content_in: p.effective_open_target().token().to_owned(),
            display_fields: p.display_fields.clone(),
            proxy: p.proxy.clone(),
            // Filled by list_servers (keyring lookup); kept out of the pure
            // builder so tests do not touch the real token store.
            has_token: false,
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
    let mut infos = server_infos(&load_cfg()?);
    // Fill has_token from the secret store (OpenProject only; GitHub uses gh).
    let secrets = Secrets::resolve();
    for info in &mut infos {
        if info.backend == "openproject" {
            info.has_token = secrets
                .get(&info.name)
                .map(|t| t.is_some())
                .unwrap_or(false);
        }
    }
    Ok(infos)
}

/// Quit the whole application (Ctrl+Q). Exits regardless of the minimize-to-tray
/// setting, unlike closing the window.
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Close the current window (Ctrl+W). This fires the normal close flow, so with
/// minimize-to-tray on it hides to the tray; otherwise the app exits.
#[tauri::command]
pub fn close_window(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
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

/// Whether any enabled server can track time. When false, the timelog indicator
/// does not apply and [`get_timelog`] returns `None` so the dashboard hides it
/// entirely. A GitHub-only (or all-disabled) config yields false. Pure and
/// unit-testable, split out from the async command's config/keyring I/O.
fn any_timelog_capable(cfg: &Config) -> bool {
    cfg.servers
        .values()
        .any(|p| p.enabled && p.backend.supports_timelog())
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
    global_proxy: Option<&str>,
) -> Result<Vec<(String, i64)>, String> {
    let token = match token_for(name, p.backend)? {
        Some(t) => t,
        None => return Ok(Vec::new()),
    };
    let client = match Client::new_with_global("", p, token, None, global_proxy) {
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
/// whole indicator. Returns `None` when no enabled server supports time tracking
/// at all (e.g. only GitHub servers), so the dashboard hides the indicator
/// entirely instead of showing an empty "not configured" state.
#[tauri::command]
pub async fn get_timelog() -> Result<Option<TimelogResult>, String> {
    let mut cfg = load_cfg()?;
    let settings = load_settings()?;
    let zone = laba_core::datetime::Zone::resolve(Some(&settings.timezone));
    let today_date = zone.today();
    let today = timelog::fmt(today_date);

    let mut entries: Vec<(String, i64)> = Vec::new();
    let mut excluded: Vec<String> = Vec::new();
    let mut starts: Vec<String> = Vec::new();
    let mut any_auto = false;
    let mut dirty = false;
    // Whether any enabled server can track time; if none, the indicator does not
    // apply and we return None below.
    let timelog_capable = any_timelog_capable(&cfg);
    // Snapshot the global proxy before borrowing `cfg.servers` mutably below.
    let global_proxy = cfg.proxy.clone();

    for (name, p) in cfg.servers.iter_mut() {
        // Disabled servers are skipped entirely.
        if !p.enabled {
            continue;
        }
        if !p.backend.supports_timelog() {
            excluded.push(name.clone());
            continue;
        }
        // Seed a start date the first time we see this server.
        let entry = p.timelog_start.get_or_insert_with(|| {
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

        entries
            .extend(server_time_minutes(name, p, &since, &today, global_proxy.as_deref()).await?);
    }

    if dirty {
        // Best-effort: persist the seeded starts into config.json; ignore failures.
        let _ = cfg.save(&default_config_path());
    }

    // No enabled server tracks time: the indicator does not apply, so hide it
    // rather than show an empty "not configured" bar.
    if !timelog_capable {
        return Ok(None);
    }

    // ISO dates sort lexicographically, so the min string is the earliest start.
    let earliest = starts.iter().min().cloned();
    let first_day = settings.week_start.first_weekday();
    match earliest
        .and_then(|s| timelog::compute(&entries, &s, today_date, first_day).map(|r| (s, r)))
    {
        Some((start, (status, timeline))) => Ok(Some(TimelogResult {
            configured: true,
            status,
            timeline,
            start,
            start_is_default: any_auto,
            excluded,
        })),
        None => Ok(Some(TimelogResult {
            configured: false,
            status: timelog::empty_status(),
            timeline: Vec::new(),
            start: String::new(),
            start_is_default: any_auto,
            excluded,
        })),
    }
}

#[tauri::command]
pub async fn list_tasks(
    server: String,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<backend::Page<laba_core::entities::Task>, String> {
    let cfg = load_cfg()?;
    let profile = cfg
        .servers
        .get(&server)
        .ok_or_else(|| format!("unknown server '{server}'"))?
        .clone();
    let token = token_for(&server, profile.backend)?;
    if profile.backend == BackendKind::OpenProject && token.is_none() {
        // Stable sentinel the GUI maps to a friendly "not signed in" message
        // with a link to Settings, instead of surfacing a raw 401.
        return Err("not-signed-in".into());
    }
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
) -> Result<backend::Page<laba_core::entities::Notification>, String> {
    let cfg = load_cfg()?;
    let profile = cfg
        .servers
        .get(&server)
        .ok_or_else(|| format!("unknown server '{server}'"))?
        .clone();
    let token = token_for(&server, profile.backend)?;
    if profile.backend == BackendKind::OpenProject && token.is_none() {
        // Stable sentinel the GUI maps to a friendly "not signed in" message
        // with a link to Settings, instead of surfacing a raw 401.
        return Err("not-signed-in".into());
    }
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
/// Load a server's profile and its stored token (if any), for backend calls that
/// dispatch across OpenProject and GitHub. GitHub authenticates through `gh`, so
/// its token is normally `None`.
fn profile_and_token(server: &str) -> Result<(ServerProfile, Option<String>), String> {
    let cfg = load_cfg()?;
    let p = cfg
        .servers
        .get(server)
        .ok_or_else(|| format!("unknown server '{server}'"))?
        .clone();
    let token = token_for(server, p.backend)?;
    Ok((p, token))
}

fn op_client(server: &str) -> Result<Client, String> {
    let cfg = load_cfg()?;
    let p = cfg
        .servers
        .get(server)
        .ok_or_else(|| format!("unknown server '{server}'"))?
        .clone();
    if p.backend != BackendKind::OpenProject {
        return Err(format!(
            "server '{server}' backend does not support this action"
        ));
    }
    let token = token_for(server, p.backend)?.ok_or_else(|| format!("no token for '{server}'"))?;
    Client::new_with_global("", &p, token, None, cfg.proxy.as_deref()).map_err(|e| e.to_string())
}

/// Toggle a notification's read state (requirement 17). OpenProject toggles both
/// ways; GitHub can only mark read (its list is unread-only).
#[tauri::command]
pub async fn set_notification_read(server: String, id: i64, read: bool) -> Result<(), String> {
    let (profile, token) = profile_and_token(&server)?;
    backend::set_notification_read(&profile, token.as_deref(), id, read)
        .await
        .map_err(|e| e.to_string())
}

/// Mark all notifications on a server as read (requirement 6). Returns the count.
#[tauri::command]
pub async fn mark_all_read(server: String) -> Result<u64, String> {
    let (profile, token) = profile_and_token(&server)?;
    backend::mark_all_read(&profile, token.as_deref())
        .await
        .map_err(|e| e.to_string())
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

/// Fetch one work package with its description and expanded custom fields, for
/// the task-detail screen.
#[tauri::command]
pub async fn get_task_detail(server: String, id: i64) -> Result<Value, String> {
    let client = op_client(&server)?;
    work_packages::get(&client, id, false)
        .await
        .map_err(|e| e.to_string())
}

/// List a work package's comments (activities that carry a comment body) for the
/// task-detail screen, oldest first.
#[tauri::command]
pub async fn list_task_comments(server: String, id: i64) -> Result<Value, String> {
    let client = op_client(&server)?;
    comment::list(&client, id, true, 1, None, false)
        .await
        .map_err(|e| e.to_string())
}

/// Push the aggregate attention count (unread notifications + tasks in red
/// status filters) to the system tray. A count > 0 paints a red badge with the
/// number; 0 restores the plain icon. Linux-only for now (native SNI tray);
/// a no-op elsewhere until a Windows/macOS overlay is wired up.
#[tauri::command]
pub fn set_tray_status(count: u32) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    crate::linux_tray::update_badge(count);
    #[cfg(not(target_os = "linux"))]
    let _ = count;
    Ok(())
}

/// Cumulative changelog for versions newer than the running one, for the update
/// banner's "what's new". Display only; the updater plugin performs the actual
/// update. The running version is this bundle's `CARGO_PKG_VERSION`.
#[tauri::command]
pub async fn get_changelog() -> Result<Vec<laba_core::update::ReleaseNote>, String> {
    laba_core::update::changelog_since(env!("CARGO_PKG_VERSION"))
        .await
        .map_err(|e| e.to_string())
}

/// Whether the `gh` CLI dependency is satisfied. Only relevant when a GitHub
/// server is configured — the update checker never uses `gh`. Lets the GUI show
/// a friendly install/login hint at startup instead of failing mid-request.
#[derive(Debug, Clone, Serialize)]
pub struct GhDependency {
    /// A GitHub-backend server is configured (so `gh` is actually needed).
    pub used: bool,
    /// `gh` availability: "ready" | "missing" | "unauthenticated".
    pub status: laba_core::github::GhStatus,
}

#[tauri::command]
pub async fn gh_dependency() -> Result<GhDependency, String> {
    use laba_core::github::{gh_status_for_host, GhStatus};
    let cfg = load_cfg()?;
    let uses_github = cfg
        .servers
        .values()
        .any(|p| matches!(p.backend, BackendKind::Github));
    if !uses_github {
        return Ok(GhDependency {
            used: false,
            status: GhStatus::Ready,
        });
    }
    // Probing spawns `gh`; keep it off the async runtime thread.
    let status = tauri::async_runtime::spawn_blocking(|| gh_status_for_host(""))
        .await
        .map_err(|e| format!("gh probe failed: {e}"))?;
    Ok(GhDependency { used: true, status })
}

/// Probe `gh` availability for a host (empty = default), regardless of whether a
/// GitHub server is configured yet. Used by the setup wizard's GitHub step to
/// offer installing/authenticating `gh` before the profile exists.
#[tauri::command]
pub async fn gh_probe(host: String) -> Result<laba_core::github::GhStatus, String> {
    use laba_core::github::gh_status_for_host;
    tauri::async_runtime::spawn_blocking(move || gh_status_for_host(&host))
        .await
        .map_err(|e| format!("gh probe failed: {e}"))
}

/// Read the authenticated `gh` account (login + host) for a host (empty =
/// default). The setup wizard shows it so the user confirms *who* and *where*
/// `gh` is signed in before creating the profile.
#[tauri::command]
pub async fn gh_account(host: String) -> Result<laba_core::github::GhAccount, String> {
    use laba_core::github::gh_account_for_host;
    tauri::async_runtime::spawn_blocking(move || gh_account_for_host(&host))
        .await
        .map_err(|e| format!("gh account failed: {e}"))?
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
    let settings = load_settings()?;
    let zone = laba_core::datetime::Zone::resolve(Some(&settings.timezone));
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
    let today = timelog::fmt(timelog::today_local());
    let mut candidates: Vec<timelog::Candidate> = Vec::new();

    for (name, p) in &cfg.servers {
        if !p.enabled || !p.backend.supports_timelog() {
            continue;
        }
        let since = p
            .timelog_start
            .as_ref()
            .map(|t| t.date.clone())
            .unwrap_or_else(|| today.clone());
        let token = match token_for(name, p.backend)? {
            Some(t) => t,
            None => continue,
        };
        let client = match Client::new_with_global("", p, token.clone(), None, cfg.proxy.as_deref())
        {
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
            // OpenProject task ids are the numeric work-package id (id.raw).
            if let Ok(wp) = t.id.raw.parse::<i64>() {
                candidates.push(timelog::Candidate {
                    server: name.clone(),
                    wp_id: wp,
                    subject: t.title.clone(),
                    logged_min: *logged.get(&wp).unwrap_or(&0),
                });
            }
        }
    }

    Ok(timelog::rank_candidates(candidates))
}

/// Load the config, apply `f` to it, and save it back. Used by the per-server
/// profile editors so each one is a load/mutate/save unit.
fn with_config<F>(f: F) -> Result<(), String>
where
    F: FnOnce(&mut Config) -> Result<(), String>,
{
    let path = default_config_path();
    let mut cfg = Config::load(&path).map_err(|e| e.to_string())?;
    f(&mut cfg)?;
    cfg.save(&path).map_err(|e| e.to_string())
}

fn profile_mut<'a>(cfg: &'a mut Config, name: &str) -> Result<&'a mut ServerProfile, String> {
    cfg.servers
        .get_mut(name)
        .ok_or_else(|| format!("unknown server '{name}'"))
}

/// Set a server's full display name. An empty/blank value clears it, so the
/// short name (the key) is shown instead.
#[tauri::command]
pub fn set_server_display_name(name: String, display_name: Option<String>) -> Result<(), String> {
    with_config(|cfg| {
        profile_mut(cfg, &name)?.display_name = display_name.filter(|s| !s.trim().is_empty());
        Ok(())
    })
}

/// Enable or disable a server in the GUI.
#[tauri::command]
pub fn set_server_enabled(name: String, enabled: bool) -> Result<(), String> {
    with_config(|cfg| {
        profile_mut(cfg, &name)?.enabled = enabled;
        Ok(())
    })
}

/// Set a server's poll interval (seconds). A `None`/0 value clears the override
/// so the backend default applies.
#[tauri::command]
pub fn set_server_poll_secs(name: String, poll_secs: Option<u64>) -> Result<(), String> {
    with_config(|cfg| {
        profile_mut(cfg, &name)?.poll_secs = poll_secs.filter(|&s| s > 0);
        Ok(())
    })
}

/// Set a server's timelog start date (`YYYY-MM-DD`). An empty value clears it.
/// Setting a date marks it non-auto (an explicit user choice).
#[tauri::command]
pub fn set_server_timelog_start(name: String, date: Option<String>) -> Result<(), String> {
    with_config(|cfg| {
        profile_mut(cfg, &name)?.timelog_start = date
            .filter(|d| !d.trim().is_empty())
            .map(|date| TimelogStart { date, auto: false });
        Ok(())
    })
}

/// Store (and validate) an OpenProject token for a server, entered from the GUI
/// instead of the CLI. Delegates to the shared core login: the token is checked
/// against `users/me` and a duplicate account (same base URL + user) is rejected
/// unless `force`. GitHub servers authenticate via `gh` and are rejected here.
#[tauri::command]
pub async fn login_server(name: String, token: String, force: bool) -> Result<(), String> {
    let cfg = load_cfg()?;
    let secrets = Secrets::resolve();
    login_and_store(&cfg, &secrets, &name, &token, force)
        .await
        .map_err(|e| e.to_string())
}

/// Add a server profile from the GUI. `backend` is `openproject` or `github`;
/// GitHub servers authenticate through `gh` and need no token. Rejects a
/// duplicate short name. The first server added becomes the default.
#[tauri::command]
pub fn add_server(
    name: String,
    url: String,
    backend: String,
    display_name: Option<String>,
) -> Result<(), String> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err("server name is required".into());
    }
    let backend = match backend.as_str() {
        "openproject" => BackendKind::OpenProject,
        "github" => BackendKind::Github,
        other => return Err(format!("unknown backend '{other}'")),
    };
    with_config(|cfg| {
        if cfg.servers.contains_key(&name) {
            return Err(format!("server '{name}' already exists"));
        }
        cfg.servers.insert(
            name.clone(),
            ServerProfile {
                display_name: display_name.filter(|s| !s.trim().is_empty()),
                base_url: url.trim().to_owned(),
                backend,
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
        if cfg.default_server.is_none() {
            cfg.default_server = Some(name);
        }
        Ok(())
    })
}

/// Set or clear a server's row tint for a workflow status. A `None`/blank color
/// removes the mapping (status renders neutral). An unknown color token is an
/// error so a typo does not silently drop the tint.
#[tauri::command]
pub fn set_server_status_color(
    name: String,
    status: String,
    color: Option<String>,
) -> Result<(), String> {
    with_config(|cfg| {
        let profile = profile_mut(cfg, &name)?;
        match color.filter(|c| !c.trim().is_empty()) {
            Some(token) => {
                let parsed = StatusColor::from_token(&token)
                    .ok_or_else(|| format!("unknown color '{token}'"))?;
                profile.status_colors.insert(status, parsed);
            }
            None => {
                profile.status_colors.remove(&status);
            }
        }
        Ok(())
    })
}

/// Replace a server's named status filters (the task-list tabs). The GUI builds
/// the whole ordered list and sends it; an empty list clears them (the GUI then
/// auto-derives one tab per status present).
#[tauri::command]
pub fn set_server_status_filters(name: String, filters: Vec<StatusFilter>) -> Result<(), String> {
    with_config(|cfg| {
        // Drop blank-labelled entries so an unfinished row never persists.
        profile_mut(cfg, &name)?.status_filters = filters
            .into_iter()
            .filter(|f| !f.label.trim().is_empty())
            .collect();
        Ok(())
    })
}

/// Replace a server's display fields (extra task-list columns / sort keys). The
/// GUI sends the whole ordered list of custom-field names; blank entries are
/// dropped. An empty list shows no extra columns.
#[tauri::command]
pub fn set_server_display_fields(name: String, fields: Vec<String>) -> Result<(), String> {
    with_config(|cfg| {
        profile_mut(cfg, &name)?.display_fields = fields
            .into_iter()
            .map(|f| f.trim().to_owned())
            .filter(|f| !f.is_empty())
            .collect();
        Ok(())
    })
}

/// Normalize a proxy input from the settings UI: trim; an empty value clears the
/// override (inherit the next level); a URL or the `"direct"`/`"none"` sentinel
/// is kept verbatim (interpreted by [`laba_core::client::resolve_proxy`]).
fn normalize_proxy(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

/// Set a server's proxy override. Empty clears it (inherit global/env); a URL
/// routes through that proxy; `"direct"` forces a direct connection.
#[tauri::command]
pub fn set_server_proxy(name: String, proxy: Option<String>) -> Result<(), String> {
    with_config(|cfg| {
        profile_mut(cfg, &name)?.proxy = normalize_proxy(proxy);
        Ok(())
    })
}

/// Set where this server's tasks open on click: `"app"`, `"browser"`, or `None`
/// to clear the override and defer to the backend default.
#[tauri::command]
pub fn set_server_open_content_in(name: String, target: Option<String>) -> Result<(), String> {
    let parsed = match target.as_deref() {
        None | Some("") => None,
        Some("app") => Some(OpenTarget::App),
        Some("browser") => Some(OpenTarget::Browser),
        Some(other) => return Err(format!("unknown open target '{other}'")),
    };
    with_config(|cfg| {
        profile_mut(cfg, &name)?.open_content_in = parsed;
        Ok(())
    })
}

/// The global default proxy (applies to servers without their own override).
#[tauri::command]
pub fn get_global_proxy() -> Result<Option<String>, String> {
    Ok(load_cfg()?.proxy)
}

/// Set the global default proxy. Empty clears it (each server falls back to
/// env, then direct); a URL or `"direct"` sets the default.
#[tauri::command]
pub fn set_global_proxy(proxy: Option<String>) -> Result<(), String> {
    with_config(|cfg| {
        cfg.proxy = normalize_proxy(proxy);
        Ok(())
    })
}

/// Rename a server: re-key its profile, update the default, and move its stored
/// token to the new key. The short name is the identifier, so renaming it is a
/// key move rather than a field edit.
#[tauri::command]
pub fn rename_server(old: String, new: String) -> Result<(), String> {
    if old == new {
        return Ok(());
    }
    let path = default_config_path();
    let mut cfg = Config::load(&path).map_err(|e| e.to_string())?;
    if cfg.servers.contains_key(&new) {
        return Err(format!("server '{new}' already exists"));
    }
    let profile = cfg
        .servers
        .remove(&old)
        .ok_or_else(|| format!("unknown server '{old}'"))?;
    cfg.servers.insert(new.clone(), profile);
    if cfg.default_server.as_deref() == Some(old.as_str()) {
        cfg.default_server = Some(new.clone());
    }
    cfg.save(&path).map_err(|e| e.to_string())?;
    // Move the stored token from the old key to the new one (best-effort: a
    // missing token just means nothing to move).
    let secrets = Secrets::resolve();
    if let Some(token) = secrets.get(&old).map_err(|e| e.to_string())? {
        secrets.set(&new, &token).map_err(|e| e.to_string())?;
        secrets.delete(&old).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Drop a server profile from the config in memory. If it was the default, the
/// default falls back to the first remaining profile (or `None` when the list is
/// now empty). Pure config mutation, split out from [`remove_server`] so the
/// default-fallback branch is unit-testable without touching the on-disk config
/// or the secret store.
fn remove_from_config(cfg: &mut Config, name: &str) -> Result<(), String> {
    if cfg.servers.remove(name).is_none() {
        return Err(format!("unknown server '{name}'"));
    }
    if cfg.default_server.as_deref() == Some(name) {
        cfg.default_server = cfg.servers.keys().next().cloned();
    }
    Ok(())
}

/// Remove a server profile and its stored token. If it was the default, the
/// default falls back to the first remaining profile (or none when the list is
/// now empty). Mirrors the CLI `server remove`.
#[tauri::command]
pub fn remove_server(name: String) -> Result<(), String> {
    with_config(|cfg| remove_from_config(cfg, &name))?;
    // Best-effort token cleanup; a missing token just means nothing to delete.
    Secrets::resolve().delete(&name).map_err(|e| e.to_string())
}

/// Set the default server in the config in memory, rejecting an unknown name.
/// Split out from [`set_default_server`] so the validation branch is unit-testable
/// without touching the on-disk config.
fn set_default_in_config(cfg: &mut Config, name: &str) -> Result<(), String> {
    if !cfg.servers.contains_key(name) {
        return Err(format!("unknown server '{name}'"));
    }
    cfg.default_server = Some(name.to_owned());
    Ok(())
}

/// Mark a server as the default. Mirrors the CLI `server set-default`.
#[tauri::command]
pub fn set_default_server(name: String) -> Result<(), String> {
    with_config(|cfg| set_default_in_config(cfg, &name))
}

/// Sign out of an OpenProject server: delete its stored token, keeping the
/// profile. GitHub servers hold no token here (they authenticate through `gh`),
/// so deleting is a no-op for them.
#[tauri::command]
pub fn logout_server(name: String) -> Result<(), String> {
    Secrets::resolve().delete(&name).map_err(|e| e.to_string())
}

/// OpenProject servers need a token from the keyring/file secret store; GitHub
/// uses `gh` and needs none.
fn token_for(server: &str, backend: BackendKind) -> Result<Option<String>, String> {
    match backend {
        BackendKind::Github => Ok(None),
        BackendKind::OpenProject => {
            let secrets = Secrets::resolve();
            secrets.get(server).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use laba_core::config::ServerProfile;

    fn cfg() -> Config {
        let mut servers = BTreeMap::new();
        servers.insert(
            "work".into(),
            ServerProfile {
                base_url: "https://op.example".into(),
                backend: BackendKind::OpenProject,
                timeout: 30,
                verify_ssl: true,
                proxy: None,
                display_name: None,
                enabled: true,
                poll_secs: None,
                timelog_start: None,
                status_colors: Default::default(),
                status_filters: Vec::new(),
                display_fields: Vec::new(),
                open_content_in: None,
            },
        );
        servers.insert(
            "gh".into(),
            ServerProfile {
                base_url: "github.com".into(),
                backend: BackendKind::Github,
                timeout: 30,
                verify_ssl: true,
                proxy: None,
                display_name: None,
                enabled: true,
                poll_secs: None,
                timelog_start: None,
                status_colors: Default::default(),
                status_filters: Vec::new(),
                display_fields: Vec::new(),
                open_content_in: None,
            },
        );
        Config {
            schema_version: laba_core::config::CONFIG_SCHEMA_VERSION,
            default_server: Some("work".into()),
            proxy: None,
            servers,
        }
    }

    #[test]
    fn server_infos_lists_all_with_backend_and_default() {
        let infos = server_infos(&cfg());
        assert_eq!(infos.len(), 2);
        let work = infos.iter().find(|i| i.name == "work").unwrap();
        assert_eq!(work.backend, "openproject");
        assert!(work.is_default);
        assert_eq!(work.poll_secs, 120);
        assert!(work.enabled);
        // No display_name set -> falls back to the key.
        assert_eq!(work.display_name, "work");
        let gh = infos.iter().find(|i| i.name == "gh").unwrap();
        assert_eq!(gh.backend, "github");
        assert!(!gh.is_default);
        assert_eq!(gh.poll_secs, 900);
    }

    #[test]
    fn server_infos_reflect_profile_display_and_poll() {
        let mut c = cfg();
        let work = c.servers.get_mut("work").unwrap();
        work.display_name = Some("Metaprime".into());
        work.poll_secs = Some(300);
        let infos = server_infos(&c);
        let work = infos.iter().find(|i| i.name == "work").unwrap();
        assert_eq!(work.display_name, "Metaprime");
        assert_eq!(work.poll_secs, 300);
        // Server with no override keeps its backend default.
        let gh = infos.iter().find(|i| i.name == "gh").unwrap();
        assert_eq!(gh.poll_secs, 900);
    }

    #[test]
    fn server_infos_mark_disabled() {
        let mut c = cfg();
        c.servers.get_mut("gh").unwrap().enabled = false;
        let infos = server_infos(&c);
        assert!(infos.iter().find(|i| i.name == "work").unwrap().enabled);
        assert!(!infos.iter().find(|i| i.name == "gh").unwrap().enabled);
    }

    #[test]
    fn remove_default_reassigns_default_to_remaining() {
        let mut c = cfg();
        // "work" is the default; removing it hands the default to the next key.
        remove_from_config(&mut c, "work").unwrap();
        assert!(!c.servers.contains_key("work"));
        assert_eq!(c.default_server.as_deref(), Some("gh"));
    }

    #[test]
    fn remove_non_default_keeps_default() {
        let mut c = cfg();
        remove_from_config(&mut c, "gh").unwrap();
        assert!(!c.servers.contains_key("gh"));
        assert_eq!(c.default_server.as_deref(), Some("work"));
    }

    #[test]
    fn remove_last_server_clears_default() {
        let mut c = cfg();
        remove_from_config(&mut c, "gh").unwrap();
        remove_from_config(&mut c, "work").unwrap();
        assert!(c.servers.is_empty());
        assert_eq!(c.default_server, None);
    }

    #[test]
    fn remove_unknown_server_errors() {
        let mut c = cfg();
        assert!(remove_from_config(&mut c, "nope").is_err());
        // Config is otherwise unchanged.
        assert_eq!(c.servers.len(), 2);
        assert_eq!(c.default_server.as_deref(), Some("work"));
    }

    #[test]
    fn any_timelog_capable_true_when_enabled_openproject_present() {
        // Default fixture: "work" is an enabled OpenProject server.
        assert!(any_timelog_capable(&cfg()));
    }

    #[test]
    fn any_timelog_capable_false_for_github_only() {
        // Drop the OpenProject server, leaving only GitHub (no time tracking).
        let mut c = cfg();
        c.servers.remove("work");
        assert!(!any_timelog_capable(&c));
    }

    #[test]
    fn any_timelog_capable_false_when_only_capable_server_disabled() {
        // The only timelog-capable server is disabled, so the indicator hides.
        let mut c = cfg();
        c.servers.get_mut("work").unwrap().enabled = false;
        assert!(!any_timelog_capable(&c));
    }

    #[test]
    fn any_timelog_capable_false_for_empty_config() {
        let mut c = cfg();
        c.servers.clear();
        assert!(!any_timelog_capable(&c));
    }

    #[test]
    fn set_default_switches_to_existing_server() {
        let mut c = cfg();
        set_default_in_config(&mut c, "gh").unwrap();
        assert_eq!(c.default_server.as_deref(), Some("gh"));
    }

    #[test]
    fn set_default_unknown_server_errors_and_keeps_current() {
        let mut c = cfg();
        assert!(set_default_in_config(&mut c, "nope").is_err());
        assert_eq!(c.default_server.as_deref(), Some("work"));
    }
}
