//! Desktop notifications for newly-arrived unread items.
//!
//! The frontend poller detects which unread notifications are new since the last
//! poll and calls [`notify_items`] with a ready-to-show list. Each item carries
//! an opaque `target` payload that is echoed back to the frontend on click, so
//! all routing (open the task detail, open an external URL, focus a server) stays
//! in the frontend and this module needs no knowledge of it.
//!
//! Click-through is Linux-only: Tauri's notification plugin exposes click actions
//! only on mobile, so Linux talks to the freedesktop notification service
//! directly (notify-rust over zbus) with a `default` action and reveals the
//! window plus emits `open-notification` when it is invoked. Windows and macOS
//! fall back to the Tauri plugin (a basic banner without click-through).

use serde::Deserialize;
use tauri::AppHandle;

/// One notification to show. `target` is opaque here: it is emitted back to the
/// frontend verbatim on click so the frontend can route it.
#[derive(Debug, Clone, Deserialize)]
pub struct NotifyItem {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub target: serde_json::Value,
}

/// Show a desktop notification for each item. Called from the frontend when new
/// unread items arrive; a no-op list is fine.
#[tauri::command]
pub fn notify_items(app: AppHandle, items: Vec<NotifyItem>) {
    for item in items {
        show_notification(&app, item);
    }
}

/// Linux: freedesktop notification with a `default` click action. Shown on a
/// detached thread because `wait_for_action` blocks until the banner is acted on
/// or dismissed. On click, reveal the window and emit the item's target so the
/// frontend navigates to it.
#[cfg(target_os = "linux")]
fn show_notification(app: &AppHandle, item: NotifyItem) {
    use tauri::Emitter;
    let app = app.clone();
    std::thread::spawn(move || {
        let shown = notify_rust::Notification::new()
            .summary(&item.title)
            .body(&item.body)
            .appname("laba")
            .action("default", "Open")
            .show();
        match shown {
            Ok(handle) => handle.wait_for_action(|action| {
                if action == "default" {
                    let a = app.clone();
                    let _ = app.run_on_main_thread(move || crate::show_main_window(&a));
                    let _ = app.emit("open-notification", item.target);
                }
            }),
            Err(e) => log::warn!("desktop notification failed: {e}"),
        }
    });
}

/// Windows/macOS: a basic banner through the Tauri notification plugin. No click
/// action (the plugin's Actions API is mobile-only).
#[cfg(not(target_os = "linux"))]
fn show_notification(app: &AppHandle, item: NotifyItem) {
    use tauri_plugin_notification::NotificationExt;
    if let Err(e) = app
        .notification()
        .builder()
        .title(item.title)
        .body(item.body)
        .show()
    {
        log::warn!("desktop notification failed: {e}");
    }
}
