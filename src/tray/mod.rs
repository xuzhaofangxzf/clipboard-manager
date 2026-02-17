use anyhow::{Context, Result};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

pub struct TrayManager {
    _tray_icon: TrayIcon,
}

impl TrayManager {
    pub fn new<F>(icon_path: std::path::PathBuf, on_event: F) -> Result<Self>
    where
        F: Fn(MenuEvent) + Send + 'static,
    {
        let tray_menu = Menu::new();

        let show_item = MenuItem::new("Show Clipboard", true, None);
        let settings_item = MenuItem::new("Settings...", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

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
            return Err(anyhow::anyhow!("Tray icon not found at {:?}", icon_path));
        };

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
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
        F: Fn(MenuEvent) + Send + 'static,
    {
        std::thread::spawn(move || {
            let receiver = MenuEvent::receiver();
            loop {
                if let Ok(event) = receiver.recv() {
                    on_event(event);
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
