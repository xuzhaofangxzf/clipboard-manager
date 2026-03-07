use gpui::prelude::FluentBuilder as _;
use gpui::prelude::InteractiveElement as _;
use gpui::prelude::StatefulInteractiveElement as _;
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::theme::{Theme as UiTheme, ThemeMode};
use gpui_component::{VirtualListScrollHandle, v_virtual_list};
use smol::Timer;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};
#[cfg(target_os = "macos")]
use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};

use crate::clipboard::ClipboardMonitor;
use crate::db::{ClipboardDatabase, ClipboardEntry};
use crate::settings::{Settings, Theme as SettingsTheme};

const LIST_ITEM_HEIGHT_PX: f32 = 86.0;
const TITLEBAR_SAFE_INSET_PX: f32 = if cfg!(target_os = "macos") { 30.0 } else { 0.0 };

pub enum MainWindowCommand {
    PreviewTheme(SettingsTheme),
    ApplySettings(Settings),
}

pub struct MainWindow {
    db: Arc<ClipboardDatabase>,
    clipboard_monitor: Arc<ClipboardMonitor>,
    entries: Vec<ClipboardEntry>,
    search_query: String,
    search_input: Entity<InputState>,
    window_alive: Arc<AtomicBool>,
    on_request_hide: Arc<dyn Fn() + Send + Sync + 'static>,
    settings: Settings,
    is_pinned: bool,
    max_count: usize,
    list_scroll_handle: VirtualListScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl MainWindow {
    pub fn new(
        db: Arc<ClipboardDatabase>,
        clipboard_monitor: Arc<ClipboardMonitor>,
        max_count: usize,
        search_input: Entity<InputState>,
        window_alive: Arc<AtomicBool>,
        on_request_hide: Arc<dyn Fn() + Send + Sync + 'static>,
        ui_refresh_rx: Receiver<ClipboardEntry>,
        command_rx: Receiver<MainWindowCommand>,
        settings: Settings,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let mut window = Self {
                db,
                clipboard_monitor,
                entries: Vec::new(),
                search_query: String::new(),
                search_input: search_input.clone(),
                window_alive: window_alive.clone(),
                on_request_hide,
                settings,
                is_pinned: false,
                max_count,
                list_scroll_handle: VirtualListScrollHandle::new(),
                _subscriptions: Vec::new(),
            };

            let subscription = cx.subscribe(
                &search_input,
                |this: &mut MainWindow, state, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        let value = state.read(cx).value();
                        this.handle_search(value.to_string(), cx);
                    }
                },
            );
            window._subscriptions.push(subscription);

            let refresh_window_alive = window_alive.clone();
            cx.spawn(async move |this, cx| {
                loop {
                    Timer::after(Duration::from_millis(120)).await;
                    if !refresh_window_alive.load(Ordering::Relaxed) {
                        return;
                    }

                    loop {
                        if !refresh_window_alive.load(Ordering::Relaxed) {
                            return;
                        }
                        match ui_refresh_rx.try_recv() {
                            Ok(entry) => {
                                let result = this.update(cx, |this, cx| {
                                    this.handle_new_entry(entry, cx);
                                });
                                if result.is_err() {
                                    return;
                                }
                            }
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => return,
                        }
                    }
                }
            })
            .detach();

            let command_window_alive = window_alive.clone();
            cx.spawn(async move |this, cx| {
                loop {
                    Timer::after(Duration::from_millis(120)).await;
                    if !command_window_alive.load(Ordering::Relaxed) {
                        return;
                    }

                    loop {
                        if !command_window_alive.load(Ordering::Relaxed) {
                            return;
                        }
                        match command_rx.try_recv() {
                            Ok(MainWindowCommand::ApplySettings(new_settings)) => {
                                let result = this.update(cx, |this, cx| {
                                    this.apply_settings_update(new_settings, cx);
                                });
                                if result.is_err() {
                                    return;
                                }
                            }
                            Ok(MainWindowCommand::PreviewTheme(theme)) => {
                                let result = this.update(cx, |this, cx| {
                                    this.apply_theme_preview(theme, cx);
                                });
                                if result.is_err() {
                                    return;
                                }
                            }
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => return,
                        }
                    }
                }
            })
            .detach();

            window.load_entries(cx);
            window
        })
    }

    fn load_entries(&mut self, cx: &mut Context<Self>) {
        if self.search_query.is_empty() {
            match self.db.get_entries(0, self.max_count) {
                Ok(mut entries) => {
                    // Keep in-memory order as oldest -> newest for O(1) push on new clipboard items.
                    entries.reverse();
                    self.entries = entries;
                    cx.notify();
                }
                Err(e) => {
                    log::error!("Failed to load entries: {}", e);
                }
            }
        } else {
            match self.db.search_entries(&self.search_query, self.max_count) {
                Ok(mut entries) => {
                    // Keep in-memory order as oldest -> newest for O(1) push on new clipboard items.
                    entries.reverse();
                    self.entries = entries;
                    cx.notify();
                }
                Err(e) => {
                    log::error!("Failed to search entries: {}", e);
                }
            }
        }
    }

    fn handle_search(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_query = query;
        self.load_entries(cx);
    }

    fn handle_new_entry(&mut self, entry: ClipboardEntry, cx: &mut Context<Self>) {
        if self.search_query.is_empty() {
            self.entries.push(entry);
            if self.entries.len() > self.max_count {
                self.entries.remove(0);
            }
            cx.notify();
            return;
        }

        let query_lower = self.search_query.to_lowercase();
        if entry.preview.to_lowercase().contains(&query_lower) {
            self.entries.push(entry);
            if self.entries.len() > self.max_count {
                self.entries.remove(0);
            }
            cx.notify();
        }
    }

    fn apply_theme(&self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        match self.settings.theme {
            SettingsTheme::Light => UiTheme::change(ThemeMode::Light, window, cx),
            SettingsTheme::Dark => UiTheme::change(ThemeMode::Dark, window, cx),
            SettingsTheme::System => UiTheme::sync_system_appearance(window, cx),
        }
    }

    fn handle_item_click(&mut self, entry_id: u64, _window: &mut Window, cx: &mut Context<Self>) {
        // Get the entry and copy to clipboard
        if let Ok(Some(entry)) = self.db.get_entry_by_id(entry_id) {
            if let Err(e) = self.clipboard_monitor.copy_to_clipboard(&entry.data) {
                log::error!("Failed to copy to clipboard: {}", e);
            } else {
                log::info!("Copied entry {} to clipboard", entry_id);
                match self.db.promote_entry_to_top(entry_id) {
                    Ok(Some(promoted_entry)) => {
                        self.entries.retain(|item| item.id != entry_id);

                        if self.search_query.is_empty() {
                            self.entries.push(promoted_entry);
                        } else {
                            let query_lower = self.search_query.to_lowercase();
                            if promoted_entry.preview.to_lowercase().contains(&query_lower) {
                                self.entries.push(promoted_entry);
                            }
                        }

                        if self.entries.len() > self.max_count {
                            self.entries.remove(0);
                        }
                        cx.notify();
                        if !self.is_pinned {
                            // Let the app command loop own the close-state transition.
                            if self.window_alive.load(Ordering::Relaxed) {
                                (self.on_request_hide)();
                            }
                        }
                    }
                    Ok(None) => {
                        log::warn!("Entry {} not found while promoting to top", entry_id);
                    }
                    Err(e) => {
                        log::error!("Failed to promote entry {} to top: {}", entry_id, e);
                    }
                }
            }
        }
    }

    fn handle_item_delete(&mut self, entry_id: u64, cx: &mut Context<Self>) {
        if let Err(e) = self.db.delete_entry(entry_id) {
            log::error!("Failed to delete entry {}: {}", entry_id, e);
            return;
        }

        self.entries.retain(|entry| entry.id != entry_id);
        cx.notify();
    }

    fn handle_clear_all_click(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = window.prompt(
            PromptLevel::Warning,
            "Clear all clipboard history?",
            Some("This action cannot be undone."),
            &["Cancel", "Clear"],
            cx,
        );

        cx.spawn(async move |this, cx| match prompt.await {
            Ok(1) => {
                let _ = this.update(cx, |this, cx| {
                    if let Err(err) = this.db.clear_all_entries() {
                        log::error!("Failed to clear all entries: {}", err);
                        return;
                    }
                    this.entries.clear();
                    cx.notify();
                });
            }
            Ok(_) => {}
            Err(err) => {
                log::error!("Failed to resolve clear-all prompt: {}", err);
            }
        })
        .detach();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.load_entries(cx);
    }

    fn apply_settings_update(&mut self, new_settings: Settings, cx: &mut Context<Self>) {
        self.settings = new_settings;
        self.max_count = self.settings.max_history_count;
        if let Err(e) = self.db.clear_old_entries(self.max_count) {
            log::error!("Failed to clear old entries: {}", e);
        }
        self.load_entries(cx);
    }

    fn apply_theme_preview(&mut self, theme: SettingsTheme, cx: &mut Context<Self>) {
        self.settings.theme = theme;
        cx.notify();
    }

    fn sync_pin_window_level(&self, window: &mut Window) {
        set_window_always_on_top(window, self.is_pinned);
    }
}

#[cfg(target_os = "macos")]
fn set_window_always_on_top(window: &mut Window, always_on_top: bool) {
    const NS_NORMAL_WINDOW_LEVEL: isize = 0;
    const NS_FLOATING_WINDOW_LEVEL: isize = 3;

    let Ok(window_handle) = window.window_handle() else {
        log::warn!("Failed to acquire window handle for pin state sync");
        return;
    };

    let RawWindowHandle::AppKit(appkit_handle) = window_handle.as_raw() else {
        return;
    };

    unsafe {
        let ns_view = appkit_handle.ns_view.as_ptr() as *mut objc::runtime::Object;
        let ns_window: *mut objc::runtime::Object = msg_send![ns_view, window];
        if ns_window.is_null() {
            log::warn!("Failed to resolve NSWindow from app view for pin state sync");
            return;
        }
        let level = if always_on_top {
            NS_FLOATING_WINDOW_LEVEL
        } else {
            NS_NORMAL_WINDOW_LEVEL
        };
        let _: () = msg_send![ns_window, setLevel: level];
    }
}

#[cfg(not(target_os = "macos"))]
fn set_window_always_on_top(_window: &mut Window, _always_on_top: bool) {}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_theme(Some(window), cx);
        let entries = Arc::new(self.entries.clone());

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .pt(px(TITLEBAR_SAFE_INSET_PX))
            .bg(cx.theme().background)
            .child(
                // Search + pin row
                div()
                    .w_full()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().w(px(360.0)).child(Input::new(&self.search_input)))
                    .child(
                        Button::new("clear-all")
                            .tooltip("clear all")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.handle_clear_all_click(window, cx);
                            }))
                            .child(
                                svg()
                                    .size_4()
                                    .text_color(cx.theme().muted_foreground)
                                    .path("icons/clear.svg"),
                            ),
                    )
                    .child(
                        div()
                            .id("pin-button")
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.is_pinned = !this.is_pinned;
                                this.sync_pin_window_level(window);
                                if this.is_pinned {
                                    window.activate_window();
                                }
                                cx.notify();
                            }))
                            .child(
                                svg()
                                    .size_4()
                                    .text_color(if self.is_pinned {
                                        cx.theme().primary
                                    } else {
                                        cx.theme().muted_foreground
                                    })
                                    .path("icons/pin.svg"),
                            ),
                    ),
            )
            .child(
                // Main content
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when(entries.is_empty(), |this| {
                        this.flex_1().child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_lg()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("No clipboard history"),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("Copy something to get started"),
                                        ),
                                ),
                        )
                    })
                    .when(!entries.is_empty(), |this| {
                        let row_sizes = Rc::new(
                            (0..entries.len())
                                .map(|_| size(px(1.0), px(LIST_ITEM_HEIGHT_PX)))
                                .collect::<Vec<_>>(),
                        );
                        let this_entity = cx.entity().downgrade();
                        let scroll_handle = self.list_scroll_handle.clone();
                        this.child(
                            div()
                                .relative()
                                .size_full()
                                .overflow_hidden()
                                .child(
                                    v_virtual_list(
                                        cx.entity(),
                                        "clipboard-list",
                                        row_sizes,
                                        move |this, visible_range, _window, _cx| {
                                            let mut rows = Vec::with_capacity(visible_range.len());
                                            for display_idx in visible_range {
                                                let actual_idx =
                                                    this.entries.len() - 1 - display_idx;
                                                let entry = this.entries[actual_idx].clone();
                                                let entry_id = entry.id;
                                                let this_entity = this_entity.clone();

                                                rows.push(
                                                    div()
                                                        .w_full()
                                                        .min_h(px(LIST_ITEM_HEIGHT_PX))
                                                        .child(
                                                        super::list_item::ClipboardListItem::new(
                                                            entry,
                                                            display_idx + 1,
                                                        )
                                                        .on_click({
                                                            let this_entity = this_entity.clone();
                                                            move |window, app| {
                                                                if let Some(entity) =
                                                                    this_entity.upgrade()
                                                                {
                                                                    let _ = entity.update(
                                                                        app,
                                                                        |this, cx| {
                                                                            this.handle_item_click(
                                                                                entry_id, window,
                                                                                cx,
                                                                            );
                                                                        },
                                                                    );
                                                                }
                                                            }
                                                        })
                                                        .on_delete(move |_window, app| {
                                                            if let Some(entity) =
                                                                this_entity.upgrade()
                                                            {
                                                                let _ = entity.update(
                                                                    app,
                                                                    |this, cx| {
                                                                        this.handle_item_delete(
                                                                            entry_id, cx,
                                                                        );
                                                                    },
                                                                );
                                                            }
                                                        }),
                                                    ),
                                                );
                                            }
                                            rows
                                        },
                                    )
                                    .w_full()
                                    .h_full()
                                    .with_sizing_behavior(ListSizingBehavior::Infer)
                                    .track_scroll(&scroll_handle),
                                )
                                .vertical_scrollbar(&scroll_handle),
                        )
                    }),
            )
    }
}
