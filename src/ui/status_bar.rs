//! Ruta y estado discreto en el pie de ventana.

use std::path::PathBuf;

use gpui::{App, ClickEvent, IntoElement, ParentElement, RenderOnce, Window, div, prelude::*, px};

use super::{UiClickHandler, theme, theme::Palette};

#[derive(IntoElement)]
pub struct StatusBar {
    palette: Palette,
    path: Option<PathBuf>,
    status: String,
    exported: Option<PathBuf>,
    on_reveal: Box<UiClickHandler>,
}

impl StatusBar {
    pub fn new(
        palette: Palette,
        path: Option<PathBuf>,
        status: String,
        exported: Option<PathBuf>,
        on_reveal: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            palette,
            path,
            status,
            exported,
            on_reveal: Box::new(on_reveal),
        }
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let palette = self.palette;
        let path = self
            .path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let mut footer = div()
            .w_full()
            .h(px(theme::STATUS_HEIGHT))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(theme::SPACING))
            .px(px(theme::CONTENT_PADDING))
            .border_t_1()
            .border_color(palette.border)
            .bg(palette.surface)
            .text_size(px(theme::FONT_SMALL))
            .text_color(palette.secondary)
            .child(div().flex_1().truncate().child(path));

        footer = footer.child(
            div()
                .flex()
                .items_center()
                .gap(px(theme::SPACING))
                .child(div().truncate().child(self.status)),
        );

        if self.exported.is_some() {
            footer = footer.child(
                div()
                    .id("reveal-export-folder")
                    .cursor_pointer()
                    .text_color(palette.accent)
                    .on_click(self.on_reveal)
                    .child("Abrir carpeta"),
            );
        }
        footer
    }
}
