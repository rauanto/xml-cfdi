//! Métricas en tarjetas con la escala tipográfica y tokens del tema gpuikit.

use gpui::{IntoElement, ParentElement, RenderOnce, Window, div, prelude::*};
use gpuikit::{
    elements::{
        badge::badge,
        typography::{h4, small},
    },
    layout::h_stack,
    theme::{ActiveTheme, ControlSize, Themeable},
    traits::control_sized::ControlSized,
};

use super::app_view::{AppState, format_integer};

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
    fn render(self, _: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let (xml_count, cfdi_count, error_count, conceptos_count) = match &self.state.resultados {
            Some(r) => (
                r.total_xml_leidos,
                r.total_cfdi_ok,
                r.errores.len(),
                r.filas.len(),
            ),
            None => (0, 0, 0, 0),
        };

        let card_metric = |label: &'static str, value: String, is_error: bool| {
            div()
                .flex_1()
                .min_w_0()
                .px_4()
                .py_2()
                .rounded_lg()
                .border_1()
                .border_color(theme.border())
                .bg(theme.surface())
                .flex()
                .flex_col()
                .gap_1()
                .child(small(label).muted())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(h4(value).map(|this| {
                            if is_error && error_count > 0 {
                                this.destructive()
                            } else {
                                this
                            }
                        }))
                        .child(if is_error {
                            if error_count > 0 {
                                badge("Requiere revisión")
                                    .destructive()
                                    .control_size(ControlSize::Small)
                            } else {
                                badge("Sin errores")
                                    .secondary()
                                    .control_size(ControlSize::Small)
                            }
                        } else {
                            badge("OK").secondary().control_size(ControlSize::Small)
                        }),
                )
        };

        h_stack()
            .w_full()
            .items_center()
            .gap_3()
            .px_6()
            .py_2()
            .child(card_metric("XMLs Leídos", format_integer(xml_count), false))
            .child(card_metric(
                "CFDIs Válidos",
                format_integer(cfdi_count),
                false,
            ))
            .child(card_metric(
                "Conceptos Extraídos",
                format_integer(conceptos_count),
                false,
            ))
            .child(card_metric(
                "Errores Detectados",
                format_integer(error_count),
                true,
            ))
    }
}
