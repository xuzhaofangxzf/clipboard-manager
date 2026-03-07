use gpui::prelude::InteractiveElement as _;
use gpui::prelude::StatefulInteractiveElement as _;
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::StyledExt;

use crate::db::{ClipboardData, ClipboardEntry};

const PREVIEW_LINE_HEIGHT_PX: f32 = 16.0;
const PREVIEW_MAX_LINES: f32 = 3.0;

#[derive(IntoElement)]
pub struct ClipboardListItem {
    entry: ClipboardEntry,
    index: usize,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    on_delete: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl ClipboardListItem {
    fn normalized_preview_text(entry: &ClipboardEntry) -> String {
        entry
            .preview
            .replace('\n', " ")
            .replace('\r', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn new(entry: ClipboardEntry, index: usize) -> Self {
        Self {
            entry,
            index,
            on_click: None,
            on_delete: None,
        }
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn on_delete(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_delete = Some(Box::new(handler));
        self
    }

    fn render_thumbnail(entry: &ClipboardEntry, cx: &App) -> Option<impl IntoElement> {
        if let ClipboardData::Image { data, thumbnail } = &entry.data {
            let image_bytes = if !thumbnail.is_empty() && image::guess_format(thumbnail).is_ok() {
                thumbnail.clone()
            } else if !data.is_empty() && image::guess_format(data).is_ok() {
                data.clone()
            } else {
                return None;
            };
            let image = std::sync::Arc::new(Image::from_bytes(ImageFormat::Png, image_bytes));

            return Some(
                div()
                    .flex_shrink_0()
                    .w_10()
                    .h_10()
                    .rounded_md()
                    .bg(cx.theme().muted)
                    .border_1()
                    .border_color(cx.theme().border)
                    .overflow_hidden()
                    .child(img(image).size_full().object_fit(ObjectFit::Contain)),
            );
        }
        None
    }

    fn render_preview(entry: &ClipboardEntry, cx: &App) -> impl IntoElement {
        let preview_text = Self::normalized_preview_text(entry);
        match &entry.data {
            ClipboardData::Image { .. } => {
                if let Some(thumbnail) = Self::render_thumbnail(entry, cx) {
                    div().pt_0p5().child(thumbnail)
                } else {
                    div()
                        .text_xs()
                        .text_color(cx.theme().foreground)
                        .min_h(px(PREVIEW_LINE_HEIGHT_PX))
                        .max_h(px(PREVIEW_LINE_HEIGHT_PX * PREVIEW_MAX_LINES))
                        .overflow_hidden()
                        .whitespace_normal()
                        .child(preview_text.clone())
                }
            }
            _ => div()
                .text_xs()
                .text_color(cx.theme().foreground)
                .min_h(px(PREVIEW_LINE_HEIGHT_PX))
                .max_h(px(PREVIEW_LINE_HEIGHT_PX * PREVIEW_MAX_LINES))
                .overflow_hidden()
                .whitespace_normal()
                .child(preview_text),
        }
    }
}

impl RenderOnce for ClipboardListItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ClipboardListItem {
            entry,
            index,
            on_click,
            on_delete,
        } = self;

        let row = div()
            .w_full()
            .h_full()
            .px_3()
            .py_2()
            .flex()
            .items_start()
            .gap_2()
            .bg(cx.theme().background)
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .hover(|style| style.bg(cx.theme().colors.list_hover))
            .cursor_pointer()
            .id(format!("clipboard-item-{}", entry.id))
            .on_click(move |_event, window, cx| {
                if let Some(ref handler) = on_click {
                    handler(window, cx);
                }
            })
            .child(
                // Content preview
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        // Preview content
                        Self::render_preview(&entry, cx),
                    )
                    .child(
                        // Meta row (bottom of content)
                        div()
                            .w_full()
                            .mt_auto()
                            .pt_1()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .px_0p5()
                                    .py_0p5()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .bg(cx.theme().muted)
                                    .text_color(cx.theme().muted_foreground)
                                    .text_xs()
                                    .child(format!("{}", index)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .opacity(0.75)
                                    .child(format_timestamp(&entry.timestamp)),
                            ),
                    ),
            )
            .child(
                // Delete button
                div()
                    .flex_shrink_0()
                    .id(format!("clipboard-item-delete-{}", entry.id))
                    .cursor_pointer()
                    .mt_0p5()
                    .px_1()
                    .py_0p5()
                    .rounded_md()
                    .hover(|style| style.bg(cx.theme().colors.list_hover))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        if let Some(ref handler) = on_delete {
                            handler(window, cx);
                        }
                    })
                    .child(
                        div().flex().items_center().child(
                            svg()
                                .size_3()
                                .text_color(cx.theme().muted_foreground)
                                .path("icons/trash-2.svg"),
                        ),
                    ),
            );

        row
    }
}

fn format_timestamp(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    timestamp
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}
