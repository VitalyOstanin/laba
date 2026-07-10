//! Linux system tray over the native StatusNotifierItem protocol (ksni).
//!
//! Tauri's built-in Linux tray (libayatana-appindicator) does not deliver click
//! events, so a GNOME double-click never reaches the app. Serving the tray over
//! ksni exposes the SNI `Activate` method, which the GNOME AppIndicator
//! extension invokes on a double-click — we use it to reveal the window.

use ksni::{menu::StandardItem, Tray, TrayMethods};
use tauri::AppHandle;

use crate::{show_main_window, toggle_main_window, tray_labels};

struct TaskstreamTray {
    app: AppHandle,
    show_label: String,
    quit_label: String,
}

impl TaskstreamTray {
    /// Reveal the window on the Tauri main thread. The ksni callbacks run on the
    /// service task, and GTK window calls must happen on the main thread.
    fn reveal(&self) {
        let app = self.app.clone();
        let _ = self.app.run_on_main_thread(move || show_main_window(&app));
    }

    /// Toggle the window: hide it if already active, otherwise reveal it. Used by
    /// the tray double-click so a second double-click closes the window.
    fn toggle(&self) {
        let app = self.app.clone();
        let _ = self
            .app
            .run_on_main_thread(move || toggle_main_window(&app));
    }
}

impl Tray for TaskstreamTray {
    fn id(&self) -> String {
        "taskstream-gui".into()
    }

    fn title(&self) -> String {
        "taskstream".into()
    }

    /// Themed icon installed under the name `taskstream-gui`; resolved by the
    /// tray host through the icon theme. Not a `-symbolic` name, so the host
    /// shows it as-is rather than recolouring it.
    fn icon_name(&self) -> String {
        "taskstream-gui".into()
    }

    /// GNOME's AppIndicator extension maps a double-click to `Activate`. Toggle
    /// so a double-click on the already-active window hides it again.
    fn activate(&mut self, _x: i32, _y: i32) {
        self.toggle();
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem {
                label: self.show_label.clone(),
                activate: Box::new(|this: &mut Self| this.reveal()),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: self.quit_label.clone(),
                activate: Box::new(|this: &mut Self| this.app.exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Register the tray and keep it alive for the process lifetime.
pub fn spawn(app: AppHandle) {
    let (show_label, quit_label) = tray_labels();
    let tray = TaskstreamTray {
        app,
        show_label: show_label.to_owned(),
        quit_label: quit_label.to_owned(),
    };
    tauri::async_runtime::spawn(async move {
        match tray.spawn().await {
            // Hold the handle for the whole run so the item is not torn down.
            Ok(_handle) => std::future::pending::<()>().await,
            Err(e) => log::error!("linux tray: could not register status notifier item: {e}"),
        }
    });
}
