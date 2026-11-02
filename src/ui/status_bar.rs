//! Ruta y estado discreto con controles y tipografía gpuikit.

use std::path::PathBuf;

use gpui::{App, ClickEvent, IntoElement, ParentElement, RenderOnce, Window, div, prelude::*};
use gpuikit::{
    elements::{button::button, typography::text},
    layout::h_stack,
    theme::{ActiveTheme, Themeable},
};

use super::UiClickHandler;

#[derive(IntoElement)]
pub struct StatusBar {
    path: Option<PathBuf>,
    status: String,
    exported: Option<PathBuf>,
    on_reveal: Box<UiClickHandler>,
}

impl StatusBar {
    pub fn new(
        path: Option<PathBuf>,
        status: String,
        exported: Option<PathBuf>,
        on_reveal: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            path,
            status,
            exported,
            on_reveal: Box::new(on_reveal),
        }
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let path = self
            .path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();

        h_stack()
            .w_full()
            .h(gpui::rems(1.75))
            .items_center()
            .gap_3()
            .px_6()
            .border_t_1()
            .border_color(theme.border())
            .bg(theme.surface())
            .text_color(theme.fg_muted())
            .child(text(path).small().truncate(true))
            .child(div().flex_1())
            .when_some(self.exported, |bar, _| {
                bar.child(button("reveal-export-folder", "Abrir carpeta").on_click(self.on_reveal))
            })
            .child(text(self.status).small().truncate(true))
    }
}
