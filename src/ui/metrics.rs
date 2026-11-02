//! Línea compacta con conteos del procesamiento.

use gpui::{IntoElement, ParentElement, RenderOnce, Window, div, prelude::*, px};

use super::{app_view::AppState, theme, theme::Palette};

#[derive(IntoElement)]
pub struct Metrics {
    palette: Palette,
    state: AppState,
}

impl Metrics {
    pub fn new(palette: Palette, state: AppState) -> Self {
        Self { palette, state }
    }
}

impl RenderOnce for Metrics {
    fn render(self, _: &mut Window, _: &mut gpui::App) -> impl IntoElement {
        let values = self.state.metric_text();
        div()
            .w_full()
            .h(px(theme::METRICS_HEIGHT))
            .flex()
            .items_center()
            .px(px(theme::CONTENT_PADDING))
            .text_size(px(theme::FONT_BODY))
            .text_color(self.palette.secondary)
            .child(values)
    }
}
