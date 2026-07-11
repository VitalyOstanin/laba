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
static HANDLE: OnceLock<Mutex<Option<Handle<LabaTray>>>> = OnceLock::new();

struct LabaTray {
    app: AppHandle,
    show_label: String,
    quit_label: String,
    /// Attention count shown as a red badge; 0 means the plain themed icon.
    count: u32,
}

impl LabaTray {
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

impl Tray for LabaTray {
    fn id(&self) -> String {
        "laba-gui".into()
    }

    fn title(&self) -> String {
        "laba".into()
    }

    /// Themed icon name (`laba-gui`), kept as a fallback for hosts that render
    /// `icon_name` in preference to the pixmap. The pixmap below carries the
    /// same wrench plus the attention badge.
    fn icon_name(&self) -> String {
        "laba-gui".into()
    }

    /// The app icon (wrench) with a small red count badge composited into the
    /// bottom-right corner when there is something to flag. Always a pixmap so
    /// the badge shows on hosts (GNOME) that ignore SNI overlay icons, while the
    /// icon itself keeps the app recognisable instead of a bare red number.
    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![tray_icon(self.count)]
    }

    fn tool_tip(&self) -> ToolTip {
        let title = if self.count > 0 {
            format!("laba — {} need attention", self.count)
        } else {
            "laba".into()
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
                .update(move |tray: &mut LabaTray| tray.count = count)
                .await;
        });
    }
}

/// Register the tray and keep it alive for the process lifetime.
pub fn spawn(app: AppHandle) {
    let (show_label, quit_label) = tray_labels();
    let tray = LabaTray {
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
/// The bundled 64×64 app icon decoded once into a straight-ARGB32 pixmap (SNI
/// byte order: A, R, G, B), cached for the process so repaints do not re-parse
/// the PNG.
fn base_icon() -> &'static Icon {
    static BASE: OnceLock<Icon> = OnceLock::new();
    BASE.get_or_init(|| {
        const PNG: &[u8] = include_bytes!("../icons/64x64.png");
        let mut reader = png::Decoder::new(PNG)
            .read_info()
            .expect("tray icon: PNG header");
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).expect("tray icon: PNG frame");
        let (w, h) = (info.width as i32, info.height as i32);
        let src_bpp = match info.color_type {
            png::ColorType::Rgba => 4,
            png::ColorType::Rgb => 3,
            other => panic!("tray icon: unexpected PNG color type {other:?}"),
        };
        let mut data = vec![0u8; (w * h * 4) as usize];
        for px in 0..(w * h) as usize {
            let o = px * src_bpp;
            let a = if src_bpp == 4 { buf[o + 3] } else { 0xff };
            data[px * 4..px * 4 + 4].copy_from_slice(&[a, buf[o], buf[o + 1], buf[o + 2]]);
        }
        Icon {
            width: w,
            height: h,
            data,
        }
    })
}

/// The app icon, with the attention count drawn as a red corner badge when
/// non-zero. Counts over 99 render as `99+`.
fn tray_icon(count: u32) -> Icon {
    let mut icon = base_icon().clone();
    if count > 0 {
        draw_badge(&mut icon, count);
    }
    icon
}

/// Composite a filled red circle with the count (white 3×5 bitmap digits, so no
/// font dependency) into the bottom-right corner of `icon`.
fn draw_badge(icon: &mut Icon, count: u32) {
    let (w, h) = (icon.width, icon.height);
    let diameter = (w * 9 / 16).max(16);
    let cx = w - diameter / 2 - 1;
    let cy = h - diameter / 2 - 1;
    let radius = diameter / 2;
    let red = [0xff, 0xd0, 0x39, 0x2b];
    let white = [0xff, 0xff, 0xff, 0xff];
    let mut put = |x: i32, y: i32, argb: [u8; 4]| {
        if x < 0 || y < 0 || x >= w || y >= h {
            return;
        }
        let i = ((y * w + x) * 4) as usize;
        icon.data[i..i + 4].copy_from_slice(&argb);
    };
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            let (dx, dy) = (x - cx, y - cy);
            if dx * dx + dy * dy <= radius * radius {
                put(x, y, red);
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
    // Largest scale whose glyphs still fit inside the circle, width and height.
    let glyphs = text.len() as i32;
    let inner = diameter - diameter / 4;
    let scale = ((inner / (glyphs * 4 - 1)).min(inner / 5)).max(1);
    let glyph_w = 3 * scale;
    let gap = scale;
    let total_w = glyphs * glyph_w + (glyphs - 1) * gap;
    let mut x0 = cx - total_w / 2;
    let y0 = cy - 5 * scale / 2;
    for &g in &text {
        let rows = glyph(g);
        for (ry, row) in rows.iter().enumerate() {
            for gx in 0..3 {
                if row & (1 << (2 - gx)) != 0 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            put(x0 + gx * scale + sx, y0 + ry as i32 * scale + sy, white);
                        }
                    }
                }
            }
        }
        x0 += glyph_w + gap;
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
