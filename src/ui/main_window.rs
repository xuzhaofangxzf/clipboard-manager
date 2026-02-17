use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::input::{InputEvent, InputState};
use smol::Timer;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use crate::clipboard::ClipboardMonitor;
use crate::db::{ClipboardDatabase, ClipboardEntry};

pub struct MainWindow {
    db: Arc<ClipboardDatabase>,
    clipboard_monitor: Arc<ClipboardMonitor>,
    entries: Vec<ClipboardEntry>,
    search_query: String,
    search_input: Entity<InputState>,
    max_count: usize,
    _subscriptions: Vec<Subscription>,
}

impl MainWindow {
    pub fn new(
        db: Arc<ClipboardDatabase>,
        clipboard_monitor: Arc<ClipboardMonitor>,
        max_count: usize,
        search_input: Entity<InputState>,
        ui_refresh_rx: Receiver<ClipboardEntry>,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let mut window = Self {
                db,
                clipboard_monitor,
                entries: Vec::new(),
                search_query: String::new(),
                search_input: search_input.clone(),
                max_count,
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

            cx.spawn(async move |this, cx| {
                loop {
                    Timer::after(Duration::from_millis(120)).await;

                    loop {
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

    fn handle_item_click(&mut self, entry_id: u64, _cx: &mut Context<Self>) {
        // Get the entry and copy to clipboard
        if let Ok(Some(entry)) = self.db.get_entry_by_id(entry_id) {
            if let Err(e) = self.clipboard_monitor.copy_to_clipboard(&entry.data) {
                log::error!("Failed to copy to clipboard: {}", e);
            } else {
                log::info!("Copied entry {} to clipboard", entry_id);
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

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.load_entries(cx);
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = Arc::new(self.entries.clone());

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .child(
                // Search bar
                super::search_bar::SearchBar::new(self.search_input.clone()),
            )
            .child(
                // Main content
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when(entries.is_empty(), |this| {
                        this.child(
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
                        this.child(
                            super::virtual_list::VirtualList::new("clipboard-list", entries)
                                .newest_first(true)
                                .viewport_height(px(520.0))
                                .on_click(cx.listener(|this, entry_id, _window, cx| {
                                    this.handle_item_click(*entry_id, cx);
                                }))
                                .on_delete(cx.listener(|this, entry_id, _window, cx| {
                                    this.handle_item_delete(*entry_id, cx);
                                })),
                        )
                    }),
            )
    }
}
