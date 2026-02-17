use gpui::prelude::InteractiveElement as _;
use gpui::prelude::StatefulInteractiveElement as _;
use gpui::*;
use gpui_component::ActiveTheme;

use crate::db::{ClipboardData, ClipboardEntry, ContentType};

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

    fn render_type_badge(entry: &ClipboardEntry, cx: &App) -> impl IntoElement {
        let (text, color) = match entry.content_type {
            ContentType::Text => ("Text", cx.theme().accent_foreground),
            ContentType::RichText => ("RTF", cx.theme().accent_foreground),
            ContentType::Html => ("HTML", cx.theme().accent_foreground),
            ContentType::Image => ("Image", cx.theme().accent_foreground),
        };

        div()
            .px_1p5()
            .py_0p5()
            .rounded_sm()
            .bg(cx.theme().muted)
            .text_xs()
            .text_color(color)
            .child(text)
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
                        .text_sm()
                        .text_color(cx.theme().foreground)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(preview_text.clone())
                }
            }
            _ => div()
                .text_sm()
                .text_color(cx.theme().foreground)
                .overflow_hidden()
                .whitespace_nowrap()
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
            .py_1p5()
            .flex()
            .items_center()
            .gap_2()
            .bg(cx.theme().colors.list)
            .border_b_1()
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
                // Index number
                div()
                    .flex_shrink_0()
                    .w_7()
                    .h_7()
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
                // Content preview
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .overflow_hidden()
                    .child(
                        // Content type badge
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Self::render_type_badge(&entry, cx))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format_timestamp(&entry.timestamp)),
                            ),
                    )
                    .child(
                        // Preview content
                        Self::render_preview(&entry, cx),
                    ),
            )
            .child(
                // Delete button
                div()
                    .flex_shrink_0()
                    .id(format!("clipboard-item-delete-{}", entry.id))
                    .cursor_pointer()
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
                        div().size_4().child(
                            svg()
                                .size_full()
                                .text_color(cx.theme().danger)
                                .path("icons/trash-2.svg"),
                        ),
                    ),
            );

        row
    }
}

fn format_timestamp(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*timestamp);

    if duration.num_seconds() < 60 {
        "Just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{} min ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{} days ago", duration.num_days())
    } else {
        timestamp.format("%Y-%m-%d %H:%M").to_string()
    }
}
