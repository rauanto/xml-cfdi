//! Barra superior compuesta con controles y tipografía de gpuikit.

use gpui::{App, ClickEvent, IntoElement, RenderOnce, Window, prelude::*};
use gpuikit::{
    elements::{button::button, typography::h4},
    layout::h_stack,
    theme::{ActiveTheme, Themeable},
};

use super::UiClickHandler;

#[derive(IntoElement)]
pub struct Toolbar {
    export_enabled: bool,
    on_export: Box<UiClickHandler>,
}

impl Toolbar {
    pub fn new(
        export_enabled: bool,
        on_export: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            export_enabled,
            on_export: Box::new(on_export),
        }
    }
}

impl RenderOnce for Toolbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        h_stack()
            .w_full()
            .h(gpui::rems(3.0))
            .items_center()
            .justify_between()
            .px_6()
            .border_b_1()
            .border_color(theme.border())
            .bg(theme.bg())
            .child(h4("CFDI ZIP Processor"))
            .child(
                button("export-button", "Exportar Excel")
                    .disabled(!self.export_enabled)
                    .on_click(self.on_export),
            )
    }
}
