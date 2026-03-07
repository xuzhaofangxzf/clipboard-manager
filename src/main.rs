use anyhow::Result;
use clipboard_manager::clipboard::ClipboardMonitor;
use clipboard_manager::clipboard::ClipboardMonitorHandler;
use clipboard_manager::db::ClipboardDatabase;
use clipboard_manager::db::ClipboardEntry;
use clipboard_manager::settings::Settings;
use clipboard_manager::shortcuts::ShortcutManager;
use clipboard_manager::tray::{TrayAction, TrayManager};
use clipboard_manager::ui::{MainWindow, MainWindowCommand, SettingsWindow};
use global_hotkey::HotKeyState;
use gpui::*;
use gpui_component::Root;
use gpui_component::input::InputState;
use gpui_component::scroll::ScrollbarShow;
use gpui_component::theme::{Theme as UiTheme, ThemeMode};
use log::{error, info};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

#[cfg(target_os = "macos")]
use cocoa::appkit::{NSApp, NSApplication, NSImage};
#[cfg(target_os = "macos")]
use cocoa::base::nil;
#[cfg(target_os = "macos")]
use cocoa::foundation::NSString;

#[derive(Debug, Clone, Copy)]
enum AppCommand {
    HideWindow,
    ShowWindow,
    ShowWindowFromTray,
    OpenConfiguration,
    Exit,
}

const MAIN_WINDOW_WIDTH_PX: f32 = 380.0;
const MAIN_WINDOW_HEIGHT_PX: f32 = 500.0;

#[cfg(target_os = "macos")]
fn set_app_icon(icon_path: &std::path::Path) {
    if !icon_path.exists() {
        error!("App icon file not found: {}", icon_path.display());
        return;
    }

    let path_str = icon_path.to_string_lossy();
    unsafe {
        let app = NSApp();
        if app == nil {
            error!("Failed to access NSApp for setting app icon");
            return;
        }
        let ns_path = NSString::alloc(nil).init_str(&path_str);
        let image = NSImage::alloc(nil).initByReferencingFile_(ns_path);
        if image == nil {
            error!("Failed to load app icon image: {}", icon_path.display());
            return;
        }
        app.setApplicationIconImage_(image);
    }
}

#[cfg(not(target_os = "macos"))]
fn set_app_icon(_icon_path: &std::path::Path) {}

fn apply_theme(
    theme: clipboard_manager::settings::Theme,
    window: Option<&mut Window>,
    cx: &mut App,
) {
    match theme {
        clipboard_manager::settings::Theme::Light => UiTheme::change(ThemeMode::Light, window, cx),
        clipboard_manager::settings::Theme::Dark => UiTheme::change(ThemeMode::Dark, window, cx),
        clipboard_manager::settings::Theme::System => UiTheme::sync_system_appearance(window, cx),
    }
}

fn open_main_window(
    cx: &mut App,
    db: Arc<ClipboardDatabase>,
    monitor: Arc<ClipboardMonitor>,
    app_settings: Settings,
    max_history: usize,
    ui_refresh_rx: mpsc::Receiver<ClipboardEntry>,
    ui_cmd_rx: mpsc::Receiver<MainWindowCommand>,
    app_cmd_tx: mpsc::Sender<AppCommand>,
) -> Option<(WindowHandle<Root>, Arc<AtomicBool>)> {
    let window_alive = Arc::new(AtomicBool::new(true));
    let window_alive_for_view = window_alive.clone();
    match cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: Size {
                    width: px(MAIN_WINDOW_WIDTH_PX),
                    height: px(MAIN_WINDOW_HEIGHT_PX),
                },
            })),
            is_resizable: false,
            window_min_size: Some(size(px(MAIN_WINDOW_WIDTH_PX), px(MAIN_WINDOW_HEIGHT_PX))),
            titlebar: Some(TitlebarOptions {
                title: Some("Clipboard Manager".into()),
                appears_transparent: true,
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
            let hide_cmd_tx = app_cmd_tx.clone();
            let config_cmd_tx = app_cmd_tx.clone();

            let main_window = MainWindow::new(
                db,
                monitor,
                max_history,
                search_input,
                window_alive_for_view,
                Arc::new(move || {
                    let _ = hide_cmd_tx.send(AppCommand::HideWindow);
                }),
                Arc::new({
                    move || {
                        let _ = config_cmd_tx.send(AppCommand::OpenConfiguration);
                    }
                }),
                ui_refresh_rx,
                ui_cmd_rx,
                app_settings.clone(),
                cx,
            );
            cx.new(|cx| Root::new(main_window, window, cx))
        },
    ) {
        Ok(handle) => Some((handle, window_alive)),
        Err(err) => {
            error!("Failed to open window: {err}");
            None
        }
    }
}

fn open_settings_window(
    cx: &mut App,
    settings: Settings,
    settings_path: PathBuf,
    ui_cmd_tx_shared: Arc<Mutex<mpsc::Sender<MainWindowCommand>>>,
    runtime_settings: Arc<Mutex<Settings>>,
    shortcut_manager: Option<Arc<Mutex<ShortcutManager>>>,
) -> Option<WindowHandle<Root>> {
    match cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: Size {
                    width: px(560.0),
                    height: px(440.0),
                },
            })),
            is_resizable: false,
            window_min_size: Some(size(px(560.0), px(440.0))),
            titlebar: Some(TitlebarOptions {
                title: Some("Settings".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            focus: true,
            show: true,
            ..Default::default()
        },
        move |window, cx| {
            let settings_window = SettingsWindow::new(settings.clone())
                .on_theme_change({
                    let ui_cmd_tx_shared = ui_cmd_tx_shared.clone();
                    let runtime_settings = runtime_settings.clone();
                    let settings_path = settings_path.clone();
                    move |theme, _window, _cx| {
                        let mut to_save = None;
                        if let Ok(mut runtime) = runtime_settings.lock() {
                            runtime.theme = theme;
                            to_save = Some(runtime.clone());
                        }
                        if let Some(settings_for_save) = to_save
                            && let Err(err) = settings_for_save.save(settings_path.clone())
                        {
                            error!("Failed to save theme setting immediately: {err}");
                        }
                        if let Ok(tx) = ui_cmd_tx_shared.lock() {
                            let _ = tx.send(MainWindowCommand::PreviewTheme(theme));
                        }
                    }
                })
                .on_save({
                    let settings_path = settings_path.clone();
                    let ui_cmd_tx_shared = ui_cmd_tx_shared.clone();
                    let runtime_settings = runtime_settings.clone();
                    let shortcut_manager = shortcut_manager.clone();
                    move |new_settings, window, cx| {
                        if let Err(e) = new_settings.validate() {
                            error!("Invalid settings: {e}");
                            return;
                        }
                        if let Err(e) = new_settings.save(settings_path.clone()) {
                            error!("Failed to save settings: {e}");
                            return;
                        }
                        if let Ok(tx) = ui_cmd_tx_shared.lock() {
                            let _ = tx.send(MainWindowCommand::ApplySettings(new_settings.clone()));
                        }
                        if let Ok(mut runtime) = runtime_settings.lock() {
                            *runtime = new_settings.clone();
                        }
                        if let Some(manager) = shortcut_manager.as_ref()
                            && let Ok(mut manager) = manager.lock()
                            && let Err(err) = manager.register(&new_settings.global_shortcut)
                        {
                            error!(
                                "Failed to register shortcut '{}': {err}",
                                new_settings.global_shortcut
                            );
                        }
                        apply_theme(new_settings.theme, Some(window), cx);
                        window.remove_window();
                    }
                });
            let settings_view = cx.new(|_| settings_window);
            cx.new(|cx| Root::new(settings_view, window, cx))
        },
    ) {
        Ok(handle) => Some(handle),
        Err(err) => {
            error!("Failed to open settings window: {err}");
            None
        }
    }
}

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
    let ui_refresh_tx_shared = Arc::new(Mutex::new(Some(ui_refresh_tx)));

    let clipboard_monitor = Arc::new(ClipboardMonitor::new(Arc::clone(&db)));
    let clipboard_handler =
        ClipboardMonitorHandler::new(clipboard_monitor.clone(), ui_refresh_tx_shared.clone());
    clipboard_monitor.start(clipboard_handler)?;

    let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("icons")
        .join("app.png");

    Application::new()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            set_app_icon(&icon_path);
            // Theme::sync_system_appearance(None, cx);
            // theme::init(cx);
            gpui_component::init(cx);
            gpui_component::theme::Theme::global_mut(cx).scrollbar_show = ScrollbarShow::Scrolling;
            let db_clone = Arc::clone(&db);
            let monitor_clone = Arc::clone(&clipboard_monitor);
            let app_settings = settings.clone();
            let runtime_settings = Arc::new(Mutex::new(app_settings.clone()));
            let (app_cmd_tx, app_cmd_rx) = mpsc::channel::<AppCommand>();
            let (ui_cmd_tx, ui_cmd_rx) = mpsc::channel::<MainWindowCommand>();
            let ui_cmd_tx_shared = Arc::new(Mutex::new(ui_cmd_tx));
            let ui_refresh_tx_shared_for_commands = ui_refresh_tx_shared.clone();
            let mut ui_refresh_rx_opt = Some(ui_refresh_rx);
            let mut ui_cmd_rx_opt = Some(ui_cmd_rx);

            let initial_settings = runtime_settings
                .lock()
                .map(|v| v.clone())
                .unwrap_or_else(|_| app_settings.clone());

            let Some((window_handle, window_alive)) = open_main_window(
                cx,
                db_clone.clone(),
                monitor_clone.clone(),
                initial_settings.clone(),
                initial_settings.max_history_count,
                ui_refresh_rx_opt
                    .take()
                    .unwrap_or_else(|| mpsc::channel::<ClipboardEntry>().1),
                ui_cmd_rx_opt
                    .take()
                    .unwrap_or_else(|| mpsc::channel::<MainWindowCommand>().1),
                app_cmd_tx.clone(),
            ) else {
                return;
            };

            let _ = window_handle.update(cx, |_, window, _| {
                window.resize(size(px(MAIN_WINDOW_WIDTH_PX), px(MAIN_WINDOW_HEIGHT_PX)));
            });
            let _ = window_handle.update(cx, |_, window, cx| {
                apply_theme(settings.theme, Some(window), cx);
            });

            // Setup Shortcut Manager
            let shortcut_manager = match ShortcutManager::new() {
                Ok(mut manager) => {
                    if let Err(err) = manager.register(&settings.global_shortcut) {
                        error!(
                            "Failed to register shortcut '{}': {err}",
                            settings.global_shortcut
                        );
                    }
                    Some(Arc::new(Mutex::new(manager)))
                }
                Err(_) => {
                    error!("Failed to init ShortcutManager");
                    None
                }
            };

            let app_cmd_tx_clone = app_cmd_tx.clone();
            ShortcutManager::setup_event_handler(move |event| {
                if event.state == HotKeyState::Pressed {
                    let _ = app_cmd_tx_clone.send(AppCommand::ShowWindow);
                }
            });

            // Setup Tray Manager
            let app_cmd_tx_for_tray = app_cmd_tx.clone();
            let tray_on_event = move |event| {
                let command = match event {
                    TrayAction::OpenWindow => AppCommand::ShowWindowFromTray,
                    TrayAction::OpenConfiguration => AppCommand::OpenConfiguration,
                    TrayAction::Exit => AppCommand::Exit,
                };
                let _ = app_cmd_tx_for_tray.send(command);
            };
            match TrayManager::new(icon_path.clone(), tray_on_event) {
                Ok(manager) => {
                    let _tray_manager = Box::leak(Box::new(manager));
                }
                Err(err) => {
                    error!("Failed to initialize tray icon/menu: {err}");
                }
            }

            let window_handle_for_commands = window_handle.clone();
            let ui_cmd_tx_shared_for_commands = ui_cmd_tx_shared.clone();
            let window_alive_for_commands = window_alive.clone();
            let runtime_settings_for_commands = runtime_settings.clone();
            cx.spawn(async move |cx| {
                let mut current_window = Some(window_handle_for_commands);
                let mut current_window_alive = window_alive_for_commands;
                let mut settings_window: Option<WindowHandle<Root>> = None;
                loop {
                    smol::Timer::after(Duration::from_millis(120)).await;
                    loop {
                        match app_cmd_rx.try_recv() {
                            Ok(AppCommand::HideWindow) => {
                                if !current_window_alive.swap(false, Ordering::Relaxed) {
                                    current_window = None;
                                    continue;
                                }
                                if let Some(ref handle) = current_window {
                                    let _ = handle.update(cx, |_, window, _| {
                                        window.remove_window();
                                    });
                                }
                                if let Ok(mut tx) = ui_refresh_tx_shared_for_commands.lock() {
                                    *tx = None;
                                }
                                current_window = None;
                            }
                            Ok(AppCommand::ShowWindow) => {
                                let mut shown = false;
                                if !current_window_alive.load(Ordering::Relaxed) {
                                    current_window = None;
                                } else if let Some(ref handle) = current_window {
                                    shown = handle
                                        .update(cx, |_, window, cx| {
                                            cx.activate(true);
                                            window.activate_window();
                                        })
                                        .is_ok();
                                    if !shown {
                                        current_window = None;
                                    }
                                }
                                if !shown {
                                    let open_settings = runtime_settings_for_commands
                                        .lock()
                                        .map(|v| v.clone())
                                        .unwrap_or_else(|_| app_settings.clone());
                                    let (new_ui_refresh_tx, new_ui_refresh_rx) =
                                        mpsc::channel::<ClipboardEntry>();
                                    if let Ok(mut tx) = ui_refresh_tx_shared_for_commands.lock() {
                                        *tx = Some(new_ui_refresh_tx);
                                    }
                                    let (new_ui_cmd_tx, new_ui_cmd_rx) =
                                        mpsc::channel::<MainWindowCommand>();
                                    if let Ok(mut tx) = ui_cmd_tx_shared_for_commands.lock() {
                                        *tx = new_ui_cmd_tx;
                                    }
                                    let mut refresh_rx_opt = Some(new_ui_refresh_rx);
                                    let mut cmd_rx_opt = Some(new_ui_cmd_rx);
                                    if let Some((new_handle, new_alive)) = cx.update(|cx| {
                                        open_main_window(
                                            cx,
                                            db_clone.clone(),
                                            monitor_clone.clone(),
                                            open_settings.clone(),
                                            open_settings.max_history_count,
                                            refresh_rx_opt.take().unwrap_or_else(|| {
                                                mpsc::channel::<ClipboardEntry>().1
                                            }),
                                            cmd_rx_opt.take().unwrap_or_else(|| {
                                                mpsc::channel::<MainWindowCommand>().1
                                            }),
                                            app_cmd_tx.clone(),
                                        )
                                    }) {
                                        let _ = new_handle.update(cx, |_, window, cx| {
                                            cx.activate(true);
                                            window.activate_window();
                                        });
                                        current_window = Some(new_handle);
                                        current_window_alive = new_alive;
                                    } else {
                                        current_window = None;
                                    }
                                }
                            }
                            Ok(AppCommand::ShowWindowFromTray) => {
                                let mut shown = false;
                                if !current_window_alive.load(Ordering::Relaxed) {
                                    current_window = None;
                                } else if let Some(ref handle) = current_window {
                                    shown = handle
                                        .update(cx, |_, window, cx| {
                                            cx.activate(true);
                                            window.activate_window();
                                        })
                                        .is_ok();
                                    if !shown {
                                        current_window = None;
                                    }
                                }
                                if !shown {
                                    let open_settings = runtime_settings_for_commands
                                        .lock()
                                        .map(|v| v.clone())
                                        .unwrap_or_else(|_| app_settings.clone());
                                    let (new_ui_refresh_tx, new_ui_refresh_rx) =
                                        mpsc::channel::<ClipboardEntry>();
                                    if let Ok(mut tx) = ui_refresh_tx_shared_for_commands.lock() {
                                        *tx = Some(new_ui_refresh_tx);
                                    }
                                    let (new_ui_cmd_tx, new_ui_cmd_rx) =
                                        mpsc::channel::<MainWindowCommand>();
                                    if let Ok(mut tx) = ui_cmd_tx_shared_for_commands.lock() {
                                        *tx = new_ui_cmd_tx;
                                    }
                                    let mut refresh_rx_opt = Some(new_ui_refresh_rx);
                                    let mut cmd_rx_opt = Some(new_ui_cmd_rx);
                                    if let Some((new_handle, new_alive)) = cx.update(|cx| {
                                        open_main_window(
                                            cx,
                                            db_clone.clone(),
                                            monitor_clone.clone(),
                                            open_settings.clone(),
                                            open_settings.max_history_count,
                                            refresh_rx_opt.take().unwrap_or_else(|| {
                                                mpsc::channel::<ClipboardEntry>().1
                                            }),
                                            cmd_rx_opt.take().unwrap_or_else(|| {
                                                mpsc::channel::<MainWindowCommand>().1
                                            }),
                                            app_cmd_tx.clone(),
                                        )
                                    }) {
                                        let _ = new_handle.update(cx, |_, window, cx| {
                                            cx.activate(true);
                                            window.activate_window();
                                        });
                                        current_window = Some(new_handle);
                                        current_window_alive = new_alive;
                                    } else {
                                        current_window = None;
                                    }
                                }
                            }
                            Ok(AppCommand::OpenConfiguration) => {
                                let mut shown = false;
                                if let Some(ref handle) = settings_window {
                                    shown = handle
                                        .update(cx, |_, window, cx| {
                                            cx.activate(true);
                                            window.activate_window();
                                        })
                                        .is_ok();
                                    if !shown {
                                        settings_window = None;
                                    }
                                }
                                if !shown {
                                    let settings_for_window = runtime_settings_for_commands
                                        .lock()
                                        .map(|v| v.clone())
                                        .unwrap_or_else(|_| app_settings.clone());
                                    settings_window = cx.update(|cx| {
                                        open_settings_window(
                                            cx,
                                            settings_for_window,
                                            settings_path.clone(),
                                            ui_cmd_tx_shared_for_commands.clone(),
                                            runtime_settings_for_commands.clone(),
                                            shortcut_manager.clone(),
                                        )
                                    });
                                    if let Some(ref handle) = settings_window {
                                        let _ = handle.update(cx, |_, window, cx| {
                                            cx.activate(true);
                                            window.activate_window();
                                        });
                                    }
                                }
                            }
                            Ok(AppCommand::Exit) => std::process::exit(0),
                            Err(mpsc::TryRecvError::Empty) => break,
                            Err(mpsc::TryRecvError::Disconnected) => return,
                        }
                    }
                }
            })
            .detach();

            info!("Application initialized");
        });

    Ok(())
}
