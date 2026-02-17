use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::button::*;
use gpui_component::input::{Input, InputState};
use std::sync::Arc;

use crate::settings::Settings;

pub struct SettingsWindow {
    settings: Settings,
    on_close: Option<Arc<dyn Fn(Settings, &mut Window, &mut App) + 'static>>,
}

impl SettingsWindow {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            on_close: None,
        }
    }

    pub fn on_close(mut self, handler: impl Fn(Settings, &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Arc::new(handler));
        self
    }
}

impl RenderOnce for SettingsWindow {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let settings = self.settings.clone();
        let on_close = self.on_close.clone();
        let max_count_value = settings.max_history_count.to_string();
        let cancel_settings = settings.clone();
        let save_settings = settings.clone();

        let max_count_input = window.use_keyed_state("settings-max-count", cx, |window, cx| {
            InputState::new(window, cx).default_value(max_count_value.clone())
        });

        div()
            .absolute()
            .inset_0()
            .bg(hsla(0., 0., 0., 0.5)) // Backdrop
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w_96()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_lg()
                    .shadow_xl()
                    .p_6()
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
                                // Simple text choice for now, ideally a Dropdown
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(Button::new("theme-light").label("Light"))
                                    .child(Button::new("theme-dark").label("Dark"))
                                    .child(Button::new("theme-system").label("System")),
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
                            .mt_4()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(Button::new("cancel").label("Cancel").on_click({
                                let on_close = on_close.clone();
                                move |_, window, cx| {
                                    if let Some(ref handler) = on_close {
                                        handler(cancel_settings.clone(), window, cx);
                                    }
                                }
                            }))
                            .child(Button::new("save").label("Save").on_click({
                                let on_close = on_close.clone();
                                move |_, window, cx| {
                                    if let Some(ref handler) = on_close {
                                        handler(save_settings.clone(), window, cx);
                                    }
                                }
                            })),
                    ),
            )
    }
}
