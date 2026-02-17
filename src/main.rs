use anyhow::Result;
use clipboard_manager::clipboard::ClipboardMonitor;
use clipboard_manager::clipboard::ClipboardMonitorHandler;
use clipboard_manager::db::ClipboardDatabase;
use clipboard_manager::db::ClipboardEntry;
use clipboard_manager::settings::Settings;
use clipboard_manager::shortcuts::ShortcutManager;
use clipboard_manager::tray::TrayManager;
use clipboard_manager::ui::MainWindow;
use gpui::*;
use gpui_component::Root;
use gpui_component::input::InputState;
use log::{error, info};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::mpsc;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*.svg"]
struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(Assets::get(path).map(|file| Cow::Owned(file.data.into_owned())))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let prefix = path.trim_end_matches('/');
        Ok(Assets::iter()
            .filter(|p| prefix.is_empty() || p.starts_with(prefix))
            .map(|p| p.to_string().into())
            .collect())
    }
}

fn main() -> Result<()> {
    env_logger::init();
    info!("Starting Clipboard Manager");

    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get data directory"))?
        .join("clipboard-manager");
    std::fs::create_dir_all(&data_dir)?;

    let settings_path = data_dir.join("settings.json");
    let settings = Settings::load(settings_path.clone())?;

    let db_path = data_dir.join("clipboard_history.redb");
    let db = Arc::new(ClipboardDatabase::open(db_path)?);
    let (ui_refresh_tx, ui_refresh_rx) = mpsc::channel::<ClipboardEntry>();

    let clipboard_monitor = Arc::new(ClipboardMonitor::new(Arc::clone(&db)));
    let clipboard_handler = ClipboardMonitorHandler::new(clipboard_monitor.clone(), ui_refresh_tx);
    clipboard_monitor.start(clipboard_handler)?;

    let icon_path = std::env::current_dir()?.join("assets").join("icon.png");

    Application::new()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            // Theme::sync_system_appearance(None, cx);
            // theme::init(cx);
            gpui_component::init(cx);
            let db_clone = Arc::clone(&db);
            let monitor_clone = Arc::clone(&clipboard_monitor);
            let max_history = settings.max_history_count;

            let window_handle = match cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: Size {
                            width: px(420.0),
                            height: px(620.0),
                        },
                    })),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Clipboard Manager".into()),
                        appears_transparent: false,
                        traffic_light_position: None,
                    }),
                    focus: true,
                    show: true,
                    ..Default::default()
                },
                move |window, cx| {
                    let search_input = cx.new(|cx| {
                        InputState::new(window, cx)
                            .placeholder("Search clipboard history...")
                            .default_value("")
                    });

                    let main_window = MainWindow::new(
                        db_clone,
                        monitor_clone,
                        max_history,
                        search_input,
                        ui_refresh_rx,
                        cx,
                    );
                    cx.new(|cx| Root::new(main_window, window, cx))
                },
            ) {
                Ok(handle) => handle,
                Err(err) => {
                    error!("Failed to open window: {err}");
                    return;
                }
            };

            // Setup Shortcut Manager
            if let Ok(mut shortcut_manager) = ShortcutManager::new() {
                if let Err(err) = shortcut_manager.register(&settings.global_shortcut) {
                    error!(
                        "Failed to register shortcut '{}': {err}",
                        settings.global_shortcut
                    );
                }

                let _window_to_toggle = window_handle.clone();
                ShortcutManager::setup_event_handler(move |_event| {
                    // Toggle window visibility
                    // This is a bit tricky in GPUI without a direct "hide" API that works easily from outside
                    // But we can update the window state
                    println!("Hotkey pressed!");
                });
            } else {
                error!("Failed to init ShortcutManager");
            }

            // Setup Tray Manager
            let tray_on_event = move |_event| {
                println!("Tray event received");
            };
            let _tray_manager = TrayManager::new(icon_path, tray_on_event).ok();

            info!("Application initialized");
        });

    Ok(())
}
