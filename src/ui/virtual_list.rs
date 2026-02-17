use gpui::*;
use gpui_component::scroll::ScrollableElement as _;
use std::sync::Arc;

use crate::db::ClipboardEntry;

const ITEM_HEIGHT_PX: f32 = 74.0;
const ITEM_HEIGHT: Pixels = px(ITEM_HEIGHT_PX);
const VIEWPORT_PADDING: usize = 5;

#[derive(IntoElement)]
pub struct VirtualList {
    id: ElementId,
    entries: Arc<Vec<ClipboardEntry>>,
    scroll_offset: f32,
    viewport_height: Pixels,
    newest_first: bool,
    on_item_click: Option<Arc<dyn Fn(&u64, &mut Window, &mut App) + 'static>>,
    on_item_delete: Option<Arc<dyn Fn(&u64, &mut Window, &mut App) + 'static>>,
}

impl VirtualList {
    pub fn new(id: impl Into<ElementId>, entries: Arc<Vec<ClipboardEntry>>) -> Self {
        Self {
            id: id.into(),
            entries,
            scroll_offset: 0.0,
            viewport_height: px(520.0),
            newest_first: false,
            on_item_click: None,
            on_item_delete: None,
        }
    }

    pub fn on_click(mut self, handler: impl Fn(&u64, &mut Window, &mut App) + 'static) -> Self {
        self.on_item_click = Some(Arc::new(handler));
        self
    }

    pub fn on_delete(mut self, handler: impl Fn(&u64, &mut Window, &mut App) + 'static) -> Self {
        self.on_item_delete = Some(Arc::new(handler));
        self
    }

    pub fn viewport_height(mut self, height: Pixels) -> Self {
        self.viewport_height = height;
        self
    }

    pub fn newest_first(mut self, newest_first: bool) -> Self {
        self.newest_first = newest_first;
        self
    }

    fn calculate_visible_range(&self) -> (usize, usize) {
        let total_items = self.entries.len();
        if total_items == 0 {
            return (0, 0);
        }

        let item_height_f32 = ITEM_HEIGHT_PX;
        let viewport_height_f32 = self.viewport_height.to_f64() as f32;

        let start_index = (self.scroll_offset / item_height_f32).floor() as usize;
        let visible_count = (viewport_height_f32 / item_height_f32).ceil() as usize;

        let start = start_index.saturating_sub(VIEWPORT_PADDING);
        let end = (start_index + visible_count + VIEWPORT_PADDING).min(total_items);

        (start, end)
    }
}

impl RenderOnce for VirtualList {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let total_height = px(self.entries.len() as f32 * ITEM_HEIGHT_PX);
        let (start_idx, end_idx) = self.calculate_visible_range();

        div()
            .id(self.id.clone())
            .size_full()
            .overflow_y_scrollbar()
            .child(
                div().h(total_height).relative().children(
                    self.entries[start_idx..end_idx]
                        .iter()
                        .enumerate()
                        .map(|(idx, _entry)| {
                            let display_idx = start_idx + idx;
                            let actual_idx = if self.newest_first {
                                self.entries.len() - 1 - display_idx
                            } else {
                                display_idx
                            };
                            let entry = self.entries[actual_idx].clone();
                            let entry_id = entry.id;
                            let on_click = self.on_item_click.clone();
                            let on_delete = self.on_item_delete.clone();

                            div()
                                .absolute()
                                .top(px(display_idx as f32 * ITEM_HEIGHT_PX))
                                .w_full()
                                .h(ITEM_HEIGHT)
                                .child(
                                    super::list_item::ClipboardListItem::new(
                                        entry,
                                        display_idx + 1,
                                    )
                                    .on_click(move |window, cx| {
                                        if let Some(handler) = on_click.as_ref() {
                                            handler(&entry_id, window, cx);
                                        }
                                    })
                                    .on_delete(
                                        move |window, cx| {
                                            if let Some(handler) = on_delete.as_ref() {
                                                handler(&entry_id, window, cx);
                                            }
                                        },
                                    ),
                                )
                        }),
                ),
            )
    }
}
