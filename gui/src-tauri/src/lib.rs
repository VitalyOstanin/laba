mod commands;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

/// Read the "hide to tray on close" preference, defaulting to true when
/// settings cannot be read.
fn minimize_to_tray() -> bool {
    use taskstream_core::settings::{default_settings_path, Settings};
    Settings::load(&default_settings_path())
        .map(|s| s.minimize_to_tray)
        .unwrap_or(true)
}

/// Localized `(show, quit)` labels for the tray menu, mirroring the frontend
/// `tray.show`/`tray.quit` keys. The native tray is built in Rust at startup,
/// before the webview locale exists, so the two strings are duplicated here.
/// `Lang::System` follows the `LANG`/`LC_ALL` environment, defaulting to English.
fn tray_labels() -> (&'static str, &'static str) {
    use taskstream_core::settings::{default_settings_path, Lang, Settings};
    let lang = Settings::load(&default_settings_path())
        .map(|s| s.language)
        .unwrap_or_default();
    let ru = match lang {
        Lang::Ru => true,
        Lang::En => false,
        Lang::System => std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LANG"))
            .map(|v| v.to_lowercase().starts_with("ru"))
            .unwrap_or(false),
    };
    if ru {
        ("Показать", "Выход")
    } else {
        ("Show", "Quit")
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .build(),
        )
        .setup(|app| {
            // Open the webview devtools automatically in debug builds; a
            // right-click "Inspect" is also available there.
            #[cfg(debug_assertions)]
            if let Some(w) = app.get_webview_window("main") {
                w.open_devtools();
            }
            let (show_label, quit_label) = tray_labels();
            let show = MenuItem::with_id(app, "show", show_label, true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", quit_label, true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        // Closing the window hides it to the tray unless the user turned that
        // off in settings, in which case it quits.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if minimize_to_tray() {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_servers,
            commands::list_tasks,
            commands::list_notifications,
            commands::get_settings,
            commands::save_settings,
            commands::get_timelog,
            commands::set_notification_read,
            commands::mark_all_read,
            commands::add_comment,
            commands::list_activities,
            commands::create_time_entry,
            commands::pick_candidates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
