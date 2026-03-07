mod formats;
use anyhow::Result;
use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext, ClipboardWatcherContext};
use clipboard_rs::{ClipboardHandler, ClipboardWatcher};
use log::{error, info};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

use crate::db::{ClipboardDatabase, ClipboardEntry};
pub use formats::{extract_clipboard_data, is_same_content};

/// Clipboard monitor that watches for changes using event-based system
pub struct ClipboardMonitor {
    db: Arc<ClipboardDatabase>,
    ignore_next: Arc<AtomicBool>,
    last_entry: Arc<parking_lot::Mutex<Option<ClipboardEntry>>>,
}

impl ClipboardMonitor {
    pub fn new(db: Arc<ClipboardDatabase>) -> Self {
        Self {
            db,
            ignore_next: Arc::new(AtomicBool::new(false)),
            last_entry: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    /// Start monitoring clipboard using event-based watcher
    pub fn start(&self, handler: ClipboardMonitorHandler) -> Result<()> {
        // Create clipboard watcher with callback
        let mut watcher = ClipboardWatcherContext::<ClipboardMonitorHandler>::new()
            .map_err(|e| anyhow::anyhow!("Failed to create clipboard watcher: {}", e))?;
        watcher.add_handler(handler);
        std::thread::spawn(move || {
            info!("Clipboard monitor started with event-based watching");
            // Start watching - this blocks the thread
            watcher.start_watch();
        });

        Ok(())
    }

    /// Set flag to ignore the next clipboard change
    /// Used when programmatically setting clipboard content
    pub fn ignore_next_change(&self) {
        self.ignore_next.store(true, Ordering::Relaxed);
    }

    /// Copy data to clipboard
    pub fn copy_to_clipboard(&self, data: &crate::db::ClipboardData) -> Result<()> {
        self.ignore_next_change();

        let ctx = ClipboardContext::new()
            .map_err(|e| anyhow::anyhow!("Failed to access clipboard: {}", e))?;

        match data {
            crate::db::ClipboardData::Text(text) => {
                ctx.set_text(text.clone())
                    .map_err(|e| anyhow::anyhow!("Failed to set text: {}", e))?;
            }
            crate::db::ClipboardData::RichText { rtf, .. } => {
                // Set RTF data
                ctx.set_rich_text(rtf.clone())
                    .map_err(|e| anyhow::anyhow!("Failed to set RTF: {}", e))?;
            }
            crate::db::ClipboardData::Html { html, .. } => {
                ctx.set_html(html.clone())
                    .map_err(|e| anyhow::anyhow!("Failed to set HTML: {}", e))?;
            }
            crate::db::ClipboardData::Image { data, .. } => {
                // Decode PNG and set image
                let img = image::load_from_memory(data)?;
                let img_data = clipboard_rs::common::RustImageData::from_dynamic_image(img);

                ctx.set_image(img_data)
                    .map_err(|e| anyhow::anyhow!("Failed to set image: {}", e))?;
            }
        }

        Ok(())
    }
}

pub struct ClipboardMonitorHandler {
    monitor: Arc<ClipboardMonitor>,
    ui_refresh_tx: Arc<Mutex<Option<Sender<ClipboardEntry>>>>,
}

impl ClipboardMonitorHandler {
    pub fn new(
        monitor: Arc<ClipboardMonitor>,
        ui_refresh_tx: Arc<Mutex<Option<Sender<ClipboardEntry>>>>,
    ) -> Self {
        Self {
            monitor,
            ui_refresh_tx,
        }
    }
}

impl ClipboardHandler for ClipboardMonitorHandler {
    fn on_clipboard_change(&mut self) {
        println!("Received on clipboard change event");
        let db = Arc::clone(&self.monitor.db);
        let ignore_next = Arc::clone(&self.monitor.ignore_next);
        let last_entry = Arc::clone(&self.monitor.last_entry);
        // Check if we should ignore this change
        if ignore_next.load(Ordering::Relaxed) {
            ignore_next.store(false, Ordering::Relaxed);
            return;
        }
        // Try to extract clipboard data
        match extract_clipboard_data() {
            Ok(Some((content_type, data))) => {
                // Check for duplicates
                let mut last = last_entry.lock();
                if let Some(ref prev) = *last {
                    if is_same_content(&prev.data, &data) {
                        return;
                    }
                }

                // Create and store entry
                let mut entry = ClipboardEntry::new(content_type, data);

                match db.insert_entry(entry.clone()) {
                    Ok(id) => {
                        info!("Stored clipboard entry {}", id);
                        entry.id = id;
                        *last = Some(entry.clone());
                        let mut maybe_sender = match self.ui_refresh_tx.lock() {
                            Ok(sender) => sender,
                            Err(e) => {
                                error!("Failed to lock UI refresh sender: {}", e);
                                return;
                            }
                        };

                        if let Some(tx) = maybe_sender.as_ref()
                            && tx.send(entry).is_err()
                        {
                            *maybe_sender = None;
                        }
                    }
                    Err(e) => {
                        error!("Failed to store clipboard entry: {}", e);
                    }
                }
            }
            Ok(None) => {
                // Empty clipboard, ignore
            }
            Err(e) => {
                error!("Failed to extract clipboard data: {}", e);
            }
        }
    }
}
