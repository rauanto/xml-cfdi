//! Ventanas y presentación GPUI; sin parseo ni escritura directa de archivos.

use gpui::{Context, IntoElement, Render, Window, div};

pub struct AppView;

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
