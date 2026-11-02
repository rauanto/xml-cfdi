//! Métricas discretas con la escala tipográfica del tema gpuikit.

use gpui::{IntoElement, ParentElement, RenderOnce, Window, div, prelude::*};
use gpuikit::{elements::typography::text, layout::h_stack};

use super::app_view::AppState;

#[derive(IntoElement)]
pub struct Metrics {
    state: AppState,
}

impl Metrics {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl RenderOnce for Metrics {
    fn render(self, _: &mut Window, _: &mut gpui::App) -> impl IntoElement {
        h_stack()
            .w_full()
            .items_center()
            .px_6()
            .py_2()
            .child(text(self.state.metric_text()).muted())
            .child(div().flex_1())
    }
}
