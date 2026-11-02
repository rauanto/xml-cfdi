//! Área de carga con botón, layout y tokens de gpuikit.

use gpui::{App, ClickEvent, IntoElement, ParentElement, RenderOnce, Window, prelude::*};
use gpuikit::{
    elements::typography::text,
    layout::v_stack,
    theme::{ActiveTheme, Themeable},
};

use super::UiClickHandler;
use super::theme;

#[derive(IntoElement)]
pub struct DropZone {
    button_label: &'static str,
    on_click: Box<UiClickHandler>,
}

impl DropZone {
    pub fn new(
        button_label: &'static str,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            button_label,
            on_click: Box::new(on_click),
        }
    }
}

impl RenderOnce for DropZone {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        v_stack()
            .id("drop-zone")
            .w_full()
            .max_w(gpui::px(theme::DROP_ZONE_WIDTH))
            .h(gpui::px(theme::DROP_ZONE_HEIGHT))
            .items_center()
            .justify_center()
            .gap_3()
            .rounded_lg()
            .border_dashed()
            .border_1()
            .border_color(theme.border())
            .bg(theme.surface())
            .cursor_pointer()
            .on_click(self.on_click)
            .child(text("Arrastra un .zip o haz clic para seleccionarlo"))
            .child(text(self.button_label).accent())
    }
}
