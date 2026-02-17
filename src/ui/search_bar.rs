use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::input::{Input, InputState};

#[derive(IntoElement)]
pub struct SearchBar {
    state: Entity<InputState>,
}

impl SearchBar {
    pub fn new(state: Entity<InputState>) -> Self {
        Self { state }
    }
}

impl RenderOnce for SearchBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w_full()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(div().w_full().child(Input::new(&self.state)))
    }
}
