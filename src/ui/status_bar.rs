//! Ruta y estado discreto con controles y tipografía gpuikit.

use std::path::PathBuf;

use gpui::{App, ClickEvent, IntoElement, ParentElement, RenderOnce, Window, prelude::*};
use gpuikit::{
    elements::{
        badge::badge,
        button::button,
        typography::{small, text},
    },
    layout::h_stack,
    theme::{ActiveTheme, ControlSize, Themeable},
    traits::control_sized::ControlSized,
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
            .unwrap_or_else(|| "Ningún archivo cargado".to_owned());

        h_stack()
            .w_full()
            .h(gpui::rems(2.2))
            .items_center()
            .justify_between()
            .px_6()
            .border_t_1()
            .border_color(theme.border())
            .bg(theme.surface())
            .text_color(theme.fg_muted())
            .child(
                h_stack()
                    .gap_2()
                    .items_center()
                    .child(small("Archivo:").muted())
                    .child(text(path).small().truncate(true)),
            )
            .child(
                small("Atajos: Ctrl+O (Abrir) · Ctrl+E (Exportar) · Ctrl+1..4 (Pestañas)").muted(),
            )
            .child(
                h_stack()
                    .gap_2()
                    .items_center()
                    .when_some(self.exported, |bar, _| {
                        bar.child(
                            button("reveal-export-folder", "Abrir carpeta")
                                .control_size(ControlSize::Small)
                                .on_click(self.on_reveal),
                        )
                    })
                    .child(
                        badge(self.status)
                            .outline()
                            .control_size(ControlSize::Small),
                    ),
            )
    }
}
