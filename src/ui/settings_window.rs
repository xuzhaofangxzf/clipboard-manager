use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::button::*;
use gpui_component::input::{Input, InputState};
use gpui_component::theme::{Theme as UiTheme, ThemeMode};
use std::sync::Arc;

use crate::settings::{Settings, Theme};

const SETTINGS_TOP_PADDING_PX: f32 = if cfg!(target_os = "macos") {
    44.0
} else {
    24.0
};

pub struct SettingsWindow {
    settings: Settings,
    draft_theme: Theme,
    on_theme_change: Option<Arc<dyn Fn(Theme, &mut Window, &mut App) + 'static>>,
    on_save: Option<Arc<dyn Fn(Settings, &mut Window, &mut App) + 'static>>,
}

impl SettingsWindow {
    pub fn new(settings: Settings) -> Self {
        Self {
            draft_theme: settings.theme,
            settings,
            on_theme_change: None,
            on_save: None,
        }
    }

    pub fn on_theme_change(
        mut self,
        handler: impl Fn(Theme, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_theme_change = Some(Arc::new(handler));
        self
    }

    pub fn on_save(mut self, handler: impl Fn(Settings, &mut Window, &mut App) + 'static) -> Self {
        self.on_save = Some(Arc::new(handler));
        self
    }

    fn apply_theme_preview(theme: Theme, window: &mut Window, cx: &mut Context<Self>) {
        match theme {
            Theme::Light => UiTheme::change(ThemeMode::Light, Some(window), cx),
            Theme::Dark => UiTheme::change(ThemeMode::Dark, Some(window), cx),
            Theme::System => UiTheme::sync_system_appearance(Some(window), cx),
        }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = self.settings.clone();
        let draft_theme = self.draft_theme;
        let on_theme_change = self.on_theme_change.clone();
        let on_save = self.on_save.clone();
        let max_count_value = settings.max_history_count.to_string();
        let shortcut_value = settings.global_shortcut.clone();

        let max_count_input = window.use_keyed_state("settings-max-count", cx, |window, cx| {
            InputState::new(window, cx).default_value(max_count_value.clone())
        });
        let shortcut_input = window.use_keyed_state("settings-shortcut", cx, |window, cx| {
            InputState::new(window, cx)
                .placeholder("Cmd+Shift+V")
                .default_value(shortcut_value.clone())
        });

        div()
            .size_full()
            .bg(cx.theme().background)
            .flex()
            .flex_col()
            .p_6()
            .pt(px(SETTINGS_TOP_PADDING_PX))
            .child(
                div()
                    .w_full()
                    .max_w(px(520.0))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(div().text_xl().child("Settings"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().text_sm().child("Theme"))
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        Button::new("theme-light")
                                            .label(if draft_theme == Theme::Light {
                                                "Light (Selected)"
                                            } else {
                                                "Light"
                                            })
                                            .on_click({
                                                let on_theme_change = on_theme_change.clone();
                                                cx.listener(move |this, _, window, cx| {
                                                    this.draft_theme = Theme::Light;
                                                    if let Some(ref handler) = on_theme_change {
                                                        handler(this.draft_theme, window, cx);
                                                    }
                                                    Self::apply_theme_preview(
                                                        this.draft_theme,
                                                        window,
                                                        cx,
                                                    );
                                                    cx.notify();
                                                })
                                            }),
                                    )
                                    .child(
                                        Button::new("theme-dark")
                                            .label(if draft_theme == Theme::Dark {
                                                "Dark (Selected)"
                                            } else {
                                                "Dark"
                                            })
                                            .on_click({
                                                let on_theme_change = on_theme_change.clone();
                                                cx.listener(move |this, _, window, cx| {
                                                    this.draft_theme = Theme::Dark;
                                                    if let Some(ref handler) = on_theme_change {
                                                        handler(this.draft_theme, window, cx);
                                                    }
                                                    Self::apply_theme_preview(
                                                        this.draft_theme,
                                                        window,
                                                        cx,
                                                    );
                                                    cx.notify();
                                                })
                                            }),
                                    )
                                    .child(
                                        Button::new("theme-system")
                                            .label(if draft_theme == Theme::System {
                                                "System (Selected)"
                                            } else {
                                                "System"
                                            })
                                            .on_click({
                                                let on_theme_change = on_theme_change.clone();
                                                cx.listener(move |this, _, window, cx| {
                                                    this.draft_theme = Theme::System;
                                                    if let Some(ref handler) = on_theme_change {
                                                        handler(this.draft_theme, window, cx);
                                                    }
                                                    Self::apply_theme_preview(
                                                        this.draft_theme,
                                                        window,
                                                        cx,
                                                    );
                                                    cx.notify();
                                                })
                                            }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().text_sm().child("Max History Count"))
                            .child(Input::new(&max_count_input)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().text_sm().child("Wake Window Shortcut"))
                            .child(Input::new(&shortcut_input))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Example: Cmd+Shift+V"),
                            ),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(Button::new("cancel").label("Cancel").on_click(
                                move |_, window, _cx| {
                                    window.remove_window();
                                },
                            ))
                            .child(Button::new("save").label("Save").on_click({
                                let on_save = on_save.clone();
                                let settings = settings.clone();
                                let draft_theme = draft_theme;
                                let max_count_input = max_count_input.clone();
                                let shortcut_input = shortcut_input.clone();
                                move |_, window, cx| {
                                    let max_count_text =
                                        max_count_input.read(cx).value().to_string();
                                    let shortcut_text = shortcut_input.read(cx).value().to_string();

                                    let Ok(max_history_count) =
                                        max_count_text.trim().parse::<usize>()
                                    else {
                                        log::error!(
                                            "Invalid max history count: {}",
                                            max_count_text
                                        );
                                        return;
                                    };

                                    let global_shortcut = shortcut_text.trim().to_string();
                                    if global_shortcut.is_empty() {
                                        log::error!("Wake window shortcut cannot be empty");
                                        return;
                                    }

                                    let mut new_settings = settings.clone();
                                    new_settings.theme = draft_theme;
                                    new_settings.max_history_count = max_history_count;
                                    new_settings.global_shortcut = global_shortcut;

                                    if let Some(ref handler) = on_save {
                                        handler(new_settings, window, cx);
                                    }
                                }
                            })),
                    ),
            )
    }
}
