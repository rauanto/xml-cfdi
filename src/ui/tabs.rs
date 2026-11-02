//! Pestañas de texto para elegir la tabla de vista previa.

use std::rc::Rc;

use gpui::{App, ClickEvent, IntoElement, ParentElement, RenderOnce, Window, div, prelude::*, px};

use super::{TabSelectHandler, app_view::Vista, theme, theme::Palette};

const LABELS: [&str; 4] = [
    "CFDI",
    "Resumen por producto",
    "Clasificación sugerida",
    "Errores",
];

#[derive(IntoElement)]
pub struct Tabs {
    palette: Palette,
    active: Vista,
    error_count: usize,
    on_select: Rc<TabSelectHandler>,
}

impl Tabs {
    pub fn new(
        palette: Palette,
        active: Vista,
        error_count: usize,
        on_select: impl Fn(&usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            palette,
            active,
            error_count,
            on_select: Rc::new(on_select),
        }
    }
}

impl RenderOnce for Tabs {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let palette = self.palette;
        let active = self.active;
        let error_count = self.error_count;
        let on_select = self.on_select;
        div()
            .w_full()
            .h(px(theme::TABS_HEIGHT))
            .flex()
            .items_end()
            .gap(px(theme::SPACING))
            .px(px(theme::CONTENT_PADDING))
            .border_b_1()
            .border_color(palette.border)
            .children(LABELS.iter().enumerate().map(move |(index, label)| {
                let selected = active.index() == index;
                let text = if index == 3 && error_count > 0 {
                    format!("{label} ({error_count})")
                } else {
                    (*label).to_owned()
                };
                let on_select = on_select.clone();
                div()
                    .id(("tab", index))
                    .h(px(theme::TABS_HEIGHT - theme::SMALL_SPACING))
                    .flex()
                    .items_center()
                    .px(px(theme::SMALL_SPACING))
                    .border_b_1()
                    .border_color(if selected {
                        palette.accent
                    } else {
                        palette.surface
                    })
                    .text_size(px(theme::FONT_BODY))
                    .font_weight(if selected {
                        gpui::FontWeight::MEDIUM
                    } else {
                        gpui::FontWeight::default()
                    })
                    .text_color(if selected {
                        palette.accent
                    } else {
                        palette.secondary
                    })
                    .cursor_pointer()
                    .on_click(move |_: &ClickEvent, window, cx| on_select(&index, window, cx))
                    .child(text)
            }))
    }
}
