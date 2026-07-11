//! Linux system tray over the native StatusNotifierItem protocol (ksni).
//!
//! Tauri's built-in Linux tray (libayatana-appindicator) does not deliver click
//! events, so a GNOME double-click never reaches the app. Serving the tray over
//! ksni exposes the SNI `Activate` method, which the GNOME AppIndicator
//! extension invokes on a double-click — we use it to reveal the window.

use std::sync::{Mutex, OnceLock};

use ksni::{menu::StandardItem, Handle, Icon, ToolTip, Tray, TrayMethods};
use tauri::AppHandle;

use crate::{show_main_window, toggle_main_window, tray_labels};

/// Live handle to the running tray, so the frontend can push an attention count
/// (unread notifications + tasks in red status filters) that repaints the icon.
static HANDLE: OnceLock<Mutex<Option<Handle<TaskstreamTray>>>> = OnceLock::new();

struct TaskstreamTray {
    app: AppHandle,
    show_label: String,
    quit_label: String,
    /// Attention count shown as a red badge; 0 means the plain themed icon.
    count: u32,
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
    /// tray host through the icon theme. Cleared when a badge is shown so the
    /// host prefers `icon_pixmap` (the red count badge) instead.
    fn icon_name(&self) -> String {
        if self.count > 0 {
            String::new()
        } else {
            "taskstream-gui".into()
        }
    }

    /// A red badge with the attention count, drawn as an ARGB pixmap so it does
    /// not depend on a themed asset. Empty when there is nothing to flag, so the
    /// host falls back to the plain themed `icon_name`.
    fn icon_pixmap(&self) -> Vec<Icon> {
        if self.count == 0 {
            vec![]
        } else {
            vec![badge_icon(self.count)]
        }
    }

    fn tool_tip(&self) -> ToolTip {
        let title = if self.count > 0 {
            format!("taskstream — {} need attention", self.count)
        } else {
            "taskstream".into()
        };
        ToolTip {
            title,
            ..Default::default()
        }
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

/// Push a new attention count to the tray (0 clears the badge). Called from the
/// `set_tray_status` command whenever the frontend's unread / red-tab total
/// changes. No-op until the tray has registered.
pub fn update_badge(count: u32) {
    let Some(lock) = HANDLE.get() else { return };
    let handle = lock.lock().ok().and_then(|g| g.clone());
    if let Some(handle) = handle {
        tauri::async_runtime::spawn(async move {
            handle
                .update(move |tray: &mut TaskstreamTray| tray.count = count)
                .await;
        });
    }
}

/// Register the tray and keep it alive for the process lifetime.
pub fn spawn(app: AppHandle) {
    let (show_label, quit_label) = tray_labels();
    let tray = TaskstreamTray {
        app,
        show_label: show_label.to_owned(),
        quit_label: quit_label.to_owned(),
        count: 0,
    };
    tauri::async_runtime::spawn(async move {
        match tray.spawn().await {
            Ok(handle) => {
                // Publish the handle so `update_badge` can repaint the icon,
                // then hold it for the whole run so the item is not torn down.
                let _ = HANDLE
                    .get_or_init(|| Mutex::new(None))
                    .lock()
                    .map(|mut g| *g = Some(handle));
                std::future::pending::<()>().await
            }
            Err(e) => log::error!("linux tray: could not register status notifier item: {e}"),
        }
    });
}

// --- Badge rendering -------------------------------------------------------

/// A red rounded square (22×22) with the count drawn in white, as an SNI
/// ARGB32 icon. Self-contained: digits come from a hand-coded 3×5 bitmap font,
/// so no image/font dependency is pulled in. Counts over 99 render as `99+`
/// truncated to fit.
fn badge_icon(count: u32) -> Icon {
    const W: i32 = 22;
    const H: i32 = 22;
    const RADIUS: i32 = 5;
    // straight ARGB (network byte order per the SNI spec): bytes are A, R, G, B.
    let mut data = vec![0u8; (W * H * 4) as usize];
    let put = |data: &mut [u8], x: i32, y: i32, argb: [u8; 4]| {
        if x < 0 || y < 0 || x >= W || y >= H {
            return;
        }
        let i = ((y * W + x) * 4) as usize;
        data[i..i + 4].copy_from_slice(&argb);
    };
    // Rounded-rect red fill (skip the corner pixels outside the radius).
    let red = [0xff, 0xd0, 0x39, 0x2b];
    for y in 0..H {
        for x in 0..W {
            let dx = if x < RADIUS {
                RADIUS - x
            } else if x >= W - RADIUS {
                x - (W - RADIUS - 1)
            } else {
                0
            };
            let dy = if y < RADIUS {
                RADIUS - y
            } else if y >= H - RADIUS {
                y - (H - RADIUS - 1)
            } else {
                0
            };
            if dx * dx + dy * dy <= RADIUS * RADIUS {
                put(&mut data, x, y, red);
            }
        }
    }
    // Digits (clamp to two chars; a trailing '+' marks an overflow).
    let text: Vec<u8> = if count > 99 {
        vec![9, 9, GLYPH_PLUS]
    } else if count >= 10 {
        vec![(count / 10) as u8, (count % 10) as u8]
    } else {
        vec![count as u8]
    };
    let scale = if text.len() >= 3 { 2 } else { 3 };
    let glyph_w = 3 * scale;
    let glyph_h = 5 * scale;
    let gap = scale;
    let total_w = text.len() as i32 * glyph_w + (text.len() as i32 - 1) * gap;
    let mut x0 = (W - total_w) / 2;
    let y0 = (H - glyph_h) / 2;
    let white = [0xff, 0xff, 0xff, 0xff];
    for &g in &text {
        let rows = glyph(g);
        for (ry, row) in rows.iter().enumerate() {
            for cx in 0..3 {
                if row & (1 << (2 - cx)) != 0 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            put(
                                &mut data,
                                x0 + cx * scale + sx,
                                y0 + ry as i32 * scale + sy,
                                white,
                            );
                        }
                    }
                }
            }
        }
        x0 += glyph_w + gap;
    }
    Icon {
        width: W,
        height: H,
        data,
    }
}

/// Sentinel index for the `+` glyph (past the ten digits).
const GLYPH_PLUS: u8 = 10;

/// 3×5 bitmap rows (top to bottom), 3 low bits per row, MSB is the left column.
fn glyph(g: u8) -> [u8; 5] {
    match g {
        0 => [0b111, 0b101, 0b101, 0b101, 0b111],
        1 => [0b010, 0b110, 0b010, 0b010, 0b111],
        2 => [0b111, 0b001, 0b111, 0b100, 0b111],
        3 => [0b111, 0b001, 0b111, 0b001, 0b111],
        4 => [0b101, 0b101, 0b111, 0b001, 0b001],
        5 => [0b111, 0b100, 0b111, 0b001, 0b111],
        6 => [0b111, 0b100, 0b111, 0b101, 0b111],
        7 => [0b111, 0b001, 0b001, 0b001, 0b001],
        8 => [0b111, 0b101, 0b111, 0b101, 0b111],
        9 => [0b111, 0b101, 0b111, 0b001, 0b111],
        _ => [0b000, 0b010, 0b111, 0b010, 0b000], // '+'
    }
}
