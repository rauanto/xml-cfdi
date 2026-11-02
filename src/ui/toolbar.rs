//! Barra superior y acciones principales.

use gpui::{App, ClickEvent, IntoElement, ParentElement, RenderOnce, Window, div, prelude::*, px};

use super::{UiClickHandler, theme, theme::Palette};

#[derive(IntoElement)]
pub struct Toolbar {
    palette: Palette,
    export_enabled: bool,
    on_export: Box<UiClickHandler>,
}

impl Toolbar {
    pub fn new(
        palette: Palette,
        export_enabled: bool,
        on_export: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            palette,
            export_enabled,
            on_export: Box::new(on_export),
        }
    }
}

impl RenderOnce for Toolbar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let palette = self.palette;
        let enabled = self.export_enabled;
        div()
            .w_full()
            .h(px(theme::TOP_BAR_HEIGHT))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(theme::CONTENT_PADDING))
            .border_b_1()
            .border_color(palette.border)
            .bg(palette.surface)
            .child(
                div()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_size(px(theme::FONT_TITLE))
                    .text_color(palette.text)
                    .child("CFDI ZIP Processor"),
            )
            .child(
                div()
                    .id("export-button")
                    .h(px(theme::CONTROL_HEIGHT))
                    .px(px(theme::SPACING))
                    .flex()
                    .items_center()
                    .rounded(px(theme::CORNER_RADIUS))
                    .border_1()
                    .border_color(palette.border)
                    .text_size(px(theme::FONT_BODY))
                    .text_color(if enabled {
                        palette.text
                    } else {
                        palette.secondary
                    })
                    .when(enabled, |style| {
                        style
                            .cursor_pointer()
                            .hover(|style| style.bg(palette.hover))
                    })
                    .on_click(self.on_export)
                    .child("Exportar Excel"),
            )
    }
}
