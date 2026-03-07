use anyhow::{Context, Result};
use log::warn;
use tray_icon::{
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

#[derive(Debug, Clone, Copy)]
pub enum TrayAction {
    OpenWindow,
    OpenConfiguration,
    Exit,
}

pub struct TrayManager {
    _tray_icon: TrayIcon,
}

impl TrayManager {
    pub fn new<F>(icon_path: std::path::PathBuf, on_event: F) -> Result<Self>
    where
        F: Fn(TrayAction) + Send + Sync + 'static,
    {
        let tray_menu = Menu::new();

        let show_item = MenuItem::with_id("open-window", "Open Window", true, None);
        let settings_item = MenuItem::with_id("configuration", "Configuration", true, None);
        let quit_item = MenuItem::with_id("exit", "Exit", true, None);

        tray_menu.append_items(&[
            &show_item,
            &PredefinedMenuItem::separator(),
            &settings_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ])?;

        let icon = if icon_path.exists() {
            load_icon(&icon_path)?
        } else {
            warn!(
                "Tray icon file not found at {:?}, using built-in fallback icon",
                icon_path
            );
            fallback_icon()?
        };

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_menu_on_left_click(false)
            .with_tooltip("Clipboard Manager")
            .with_icon(icon)
            .build()?;

        Self::setup_event_handler(on_event);

        Ok(Self {
            _tray_icon: tray_icon,
        })
    }

    fn setup_event_handler<F>(on_event: F)
    where
        F: Fn(TrayAction) + Send + Sync + 'static,
    {
        let on_event = std::sync::Arc::new(on_event);

        let on_event_for_click = on_event.clone();
        std::thread::spawn(move || {
            let receiver = TrayIconEvent::receiver();
            loop {
                if let Ok(event) = receiver.recv()
                    && let TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } = event
                    && button == MouseButton::Left
                    && button_state == MouseButtonState::Up
                {
                    on_event_for_click(TrayAction::OpenWindow);
                }
            }
        });

        std::thread::spawn(move || {
            let receiver = MenuEvent::receiver();
            loop {
                if let Ok(event) = receiver.recv() {
                    let action = match event.id().as_ref() {
                        "open-window" => Some(TrayAction::OpenWindow),
                        "configuration" => Some(TrayAction::OpenConfiguration),
                        "exit" => Some(TrayAction::Exit),
                        _ => None,
                    };

                    if let Some(action) = action {
                        on_event(action);
                    }
                }
            }
        });
    }
}

fn load_icon(path: &std::path::Path) -> Result<tray_icon::Icon> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .context("Failed to open icon path")?
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .context("Failed to create tray icon from RGBA")
}

fn fallback_icon() -> Result<tray_icon::Icon> {
    const SIZE: u32 = 16;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let idx = ((y * SIZE + x) * 4) as usize;
            let is_border = x == 0 || y == 0 || x == SIZE - 1 || y == SIZE - 1;
            let is_center = (4..=11).contains(&x) && (4..=11).contains(&y);

            let (r, g, b, a) = if is_border {
                (180, 180, 180, 255)
            } else if is_center {
                (230, 230, 230, 255)
            } else {
                (40, 40, 40, 255)
            };

            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = a;
        }
    }

    tray_icon::Icon::from_rgba(rgba, SIZE, SIZE).context("Failed to create fallback tray icon")
}
