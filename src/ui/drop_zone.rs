//! Área simple para seleccionar o soltar un archivo ZIP.

use gpui::{App, ClickEvent, IntoElement, ParentElement, RenderOnce, Window, div, prelude::*, px};

use super::{UiClickHandler, theme, theme::Palette};

#[derive(IntoElement)]
pub struct DropZone {
    palette: Palette,
    button_label: &'static str,
    on_click: Box<UiClickHandler>,
}

impl DropZone {
    pub fn new(
        palette: Palette,
        button_label: &'static str,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            palette,
            button_label,
            on_click: Box::new(on_click),
        }
    }
}

impl RenderOnce for DropZone {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let palette = self.palette;
        div()
            .id("zip-drop-zone")
            .w_full()
            .max_w(px(theme::DROP_ZONE_WIDTH))
            .h(px(theme::DROP_ZONE_HEIGHT))
            .flex()
            .flex_col()
            .justify_center()
            .items_center()
            .gap(px(theme::SPACING))
            .rounded(px(theme::CORNER_RADIUS))
            .border_dashed()
            .border_1()
            .border_color(palette.border)
            .bg(palette.surface)
            .cursor_pointer()
            .hover(|style| style.bg(palette.hover))
            .on_click(self.on_click)
            .child(
                div()
                    .text_size(px(theme::FONT_LABEL))
                    .text_color(palette.text)
                    .child("Arrastra un .zip o haz clic para seleccionarlo"),
            )
            .child(
                div()
                    .text_size(px(theme::FONT_SMALL))
                    .text_color(palette.secondary)
                    .child(self.button_label),
            )
    }
}
