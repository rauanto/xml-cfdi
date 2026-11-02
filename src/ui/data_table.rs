//! Adaptador de las cuatro vistas al componente Table de gpuikit con soporte
//! para ajuste dinámico de columnas y desplazamiento horizontal asistido.
//!
//! Rendimiento de scroll:
//! - Las filas se formatean UNA vez (`construir_filas`) y se guardan como
//!   `SharedString` (clonar = incrementar un contador, sin copiar texto).
//! - Virtualización vertical con `uniform_list` (fluidez instantánea con miles de filas).
//! - Desplazamiento horizontal nativo con `overflow_x_scroll` + `ScrollHandle`
//!   y controles de deslizamiento rápido (inicio, anterior, siguiente, final).
//! - Columnas con ajuste dinámico: en tablas con pocas columnas (Resumen, Errores),
//!   las columnas principales se expanden para llenar el ancho completo de la ventana.

use std::sync::Arc;

use gpui::{
    IntoElement, ParentElement, RenderOnce, ScrollHandle, SharedString, UniformListScrollHandle,
    Window, div, point, prelude::*, px, uniform_list,
};
use gpuikit::{
    elements::{badge::badge, button::button, typography::small},
    layout::h_stack,
    theme::{ActiveTheme, ControlSize, Themeable},
    traits::control_sized::ControlSized,
};

use super::{app_view::format_integer, theme};

#[derive(Clone, Copy)]
pub enum Alineacion {
    Izquierda,
    Derecha,
}

#[derive(Clone, Copy)]
pub struct Columna {
    pub titulo: &'static str,
    pub ancho_min: f32,
    pub flex: bool,
    pub alineacion: Alineacion,
}

/// Fila ya formateada: un texto por columna.
pub type Fila = Box<[SharedString]>;

/// Formatea todas las filas una sola vez. Llamar al recibir resultados
/// (idealmente en una tarea en segundo plano), NUNCA dentro de `render`.
pub fn construir_filas<T>(
    items: &[T],
    ncols: usize,
    cell: impl Fn(&T, usize) -> String,
) -> Arc<Vec<Fila>> {
    Arc::new(
        items
            .iter()
            .map(|item| {
                (0..ncols)
                    .map(|c| SharedString::from(cell(item, c)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice()
            })
            .collect(),
    )
}

pub const CFDI_COLUMNS: [Columna; 21] = [
    col("UUID", theme::WIDTH_UUID),
    col("Fecha", theme::WIDTH_DATE),
    col("RFC Emisor", theme::WIDTH_RFC),
    col("Nombre Emisor", theme::WIDTH_NAME),
    col("RFC Receptor", theme::WIDTH_RFC),
    col("Tipo de Comprobante", theme::WIDTH_TYPE),
    col_r("Subtotal", theme::WIDTH_MONEY),
    col_r("Descuento", theme::WIDTH_MONEY),
    col_r("IVA trasladado (002)", theme::WIDTH_MONEY),
    col_r("ISR retenido (001)", theme::WIDTH_MONEY),
    col_r("IVA retenido (002)", theme::WIDTH_MONEY),
    col_r("Total", theme::WIDTH_MONEY),
    col("Uso CFDI", theme::WIDTH_CODE),
    col("Método de Pago", theme::WIDTH_METHOD),
    col("Forma de Pago", theme::WIDTH_METHOD),
    col("Moneda", theme::WIDTH_CURRENCY),
    col_r("Tipo de Cambio", theme::WIDTH_EXCHANGE),
    col("ClaveProdServ", theme::WIDTH_CODE),
    col("Descripción", theme::WIDTH_DESCRIPTION),
    col_r("Importe", theme::WIDTH_MONEY),
    col("Archivo", theme::WIDTH_FILE),
];

pub const RESUMEN_COLUMNS: [Columna; 4] = [
    col("ClaveProdServ", theme::WIDTH_CODE),
    col_flex("Descripción", theme::WIDTH_DESCRIPTION),
    col_r("Importe total", theme::WIDTH_MONEY),
    col_r("Número de conceptos", theme::WIDTH_COUNT),
];

pub const CLASIFICACION_COLUMNS: [Columna; 7] = [
    col("Archivo", theme::WIDTH_FILE),
    col("UUID", theme::WIDTH_UUID),
    col("Tipo de Comprobante", theme::WIDTH_TYPE),
    col("ClaveProdServ", theme::WIDTH_CODE),
    col("Descripción", theme::WIDTH_DESCRIPTION),
    col("Categoría sugerida", theme::WIDTH_CATEGORY),
    col_flex("Criterio", theme::WIDTH_CRITERION),
];

pub const ERROR_COLUMNS: [Columna; 2] = [
    col("Archivo", theme::WIDTH_FILE),
    col_flex("Motivo", theme::WIDTH_ERROR),
];

const fn col(titulo: &'static str, ancho: f32) -> Columna {
    Columna {
        titulo,
        ancho_min: ancho,
        flex: false,
        alineacion: Alineacion::Izquierda,
    }
}

const fn col_flex(titulo: &'static str, ancho_min: f32) -> Columna {
    Columna {
        titulo,
        ancho_min,
        flex: true,
        alineacion: Alineacion::Izquierda,
    }
}

const fn col_r(titulo: &'static str, ancho: f32) -> Columna {
    Columna {
        titulo,
        ancho_min: ancho,
        flex: false,
        alineacion: Alineacion::Derecha,
    }
}

fn celda(ancho: f32, alineacion: Alineacion, texto: SharedString) -> impl IntoElement {
    let mut el = div()
        .flex_shrink_0()
        .w(px(ancho))
        .h_full()
        .flex()
        .items_center()
        .px(px(8.))
        .truncate();
    if matches!(alineacion, Alineacion::Derecha) {
        el = el.justify_end().text_right();
    }
    el.child(texto)
}

#[derive(IntoElement)]
pub struct DataTable {
    columns: &'static [Columna],
    rows: Arc<Vec<Fila>>,
    scroll_handle: UniformListScrollHandle,
    horizontal_scroll: ScrollHandle,
    list_id: &'static str,
}

impl DataTable {
    pub fn new(
        columns: &'static [Columna],
        rows: Arc<Vec<Fila>>,
        scroll_handle: UniformListScrollHandle,
        horizontal_scroll: ScrollHandle,
        list_id: &'static str,
    ) -> Self {
        Self {
            columns,
            rows,
            scroll_handle,
            horizontal_scroll,
            list_id,
        }
    }
}

impl RenderOnce for DataTable {
    fn render(self, window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let Self {
            columns,
            rows,
            scroll_handle,
            horizontal_scroll,
            list_id,
        } = self;

        let t = cx.theme();
        let border = t.border();
        let surface = t.surface();
        let surface_secondary = t.surface_secondary();
        let foreground = t.fg();
        let muted_foreground = t.fg_muted();
        let row_hover = t.surface_tertiary();

        // Cálculo dinámico de ancho disponible y distribución de columnas
        let available_width = f32::from((window.bounds().size.width - px(48.)).max(px(500.)));
        let min_total: f32 = columns.iter().map(|c| c.ancho_min).sum();
        let flex_cols: Vec<usize> = columns
            .iter()
            .enumerate()
            .filter_map(|(i, c)| if c.flex { Some(i) } else { None })
            .collect();

        let (effective_widths, total_width) =
            if available_width > min_total && !flex_cols.is_empty() {
                let extra = available_width - min_total;
                let extra_per_col = extra / flex_cols.len() as f32;
                let mut widths: Vec<f32> = columns.iter().map(|c| c.ancho_min).collect();
                for &i in &flex_cols {
                    widths[i] += extra_per_col;
                }
                (widths, available_width)
            } else {
                let widths: Vec<f32> = columns.iter().map(|c| c.ancho_min).collect();
                (widths, min_total)
            };

        let is_scrollable = total_width > available_width;
        let row_count = rows.len();

        let body = if rows.is_empty() {
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .text_color(muted_foreground)
                .child(
                    badge("Sin registros")
                        .secondary()
                        .control_size(ControlSize::Small),
                )
                .child(small("No hay filas para mostrar en esta vista").muted())
                .into_any_element()
        } else {
            let cols = columns;
            let widths = effective_widths.clone();
            let rows_data = Arc::clone(&rows);
            uniform_list(list_id, row_count, move |range, _, _| {
                range
                    .map(|index| {
                        let fila = &rows_data[index];
                        div()
                            .id(("data-row", index))
                            .flex()
                            .items_center()
                            .h(px(theme::ROW_HEIGHT))
                            .w(px(total_width))
                            .border_b_1()
                            .border_color(border)
                            .text_size(px(13.))
                            .text_color(foreground)
                            .when(index % 2 == 1, |s| s.bg(surface_secondary))
                            .hover(|style| style.bg(row_hover))
                            .children(cols.iter().zip(widths.iter()).enumerate().map(
                                |(i, (col, &ancho))| celda(ancho, col.alineacion, fila[i].clone()),
                            ))
                    })
                    .collect::<Vec<_>>()
            })
            .size_full()
            .track_scroll(&scroll_handle)
            .scrollbar_width(px(8.))
            .into_any_element()
        };

        let header = div()
            .w(px(total_width))
            .h(px(theme::HEADER_HEIGHT))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(border)
            .bg(surface_secondary)
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_size(px(12.))
            .text_color(muted_foreground)
            .children(
                columns
                    .iter()
                    .zip(effective_widths.iter())
                    .map(|(column, &ancho)| {
                        celda(ancho, column.alineacion, SharedString::from(column.titulo))
                    }),
            );

        let navigation_bar = div()
            .w_full()
            .h(px(38.))
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .border_t_1()
            .border_color(border)
            .bg(surface_secondary)
            .child(
                h_stack()
                    .gap_2()
                    .items_center()
                    .child(
                        badge(format!("{} columnas", columns.len()))
                            .secondary()
                            .control_size(ControlSize::Small),
                    )
                    .child(small(format!("{} filas", format_integer(row_count))).muted())
                    .when(is_scrollable, |stack| {
                        stack.child(small("· Desliza con Shift + rueda o usa los botones:").muted())
                    })
                    .when(!is_scrollable, |stack| {
                        stack.child(small("· Ajustado al ancho completo").muted())
                    }),
            )
            .child(
                h_stack()
                    .gap_1()
                    .items_center()
                    .when(is_scrollable, |stack| {
                        let max_scroll = (total_width - available_width).max(0.0);
                        let h1 = horizontal_scroll.clone();
                        let h2 = horizontal_scroll.clone();
                        let h3 = horizontal_scroll.clone();
                        let h4 = horizontal_scroll.clone();
                        stack
                            .child(
                                button("scroll-to-start", "⇤ Inicio")
                                    .control_size(ControlSize::Small)
                                    .on_click(move |_, window, _| {
                                        let y = h1.offset().y;
                                        h1.set_offset(point(px(0.), y));
                                        window.refresh();
                                    }),
                            )
                            .child(
                                button("scroll-step-left", "◀ Deslizar")
                                    .control_size(ControlSize::Small)
                                    .on_click(move |_, window, _| {
                                        let y = h2.offset().y;
                                        let cur_x = h2.offset().x;
                                        let new_x = (cur_x + px(360.)).min(px(0.));
                                        h2.set_offset(point(new_x, y));
                                        window.refresh();
                                    }),
                            )
                            .child(
                                button("scroll-step-right", "Deslizar ▶")
                                    .control_size(ControlSize::Small)
                                    .on_click(move |_, window, _| {
                                        let y = h3.offset().y;
                                        let cur_x = h3.offset().x;
                                        let limit = if h3.max_offset().x > px(0.) {
                                            -h3.max_offset().x
                                        } else {
                                            px(-max_scroll)
                                        };
                                        let new_x = (cur_x - px(360.)).max(limit);
                                        h3.set_offset(point(new_x, y));
                                        window.refresh();
                                    }),
                            )
                            .child(
                                button("scroll-to-end", "Final ⇥")
                                    .control_size(ControlSize::Small)
                                    .on_click(move |_, window, _| {
                                        let y = h4.offset().y;
                                        let limit = if h4.max_offset().x > px(0.) {
                                            -h4.max_offset().x
                                        } else {
                                            px(-max_scroll)
                                        };
                                        h4.set_offset(point(limit, y));
                                        window.refresh();
                                    }),
                            )
                    }),
            );

        div()
            .w_full()
            .h_full()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(border)
            .bg(surface)
            .overflow_hidden()
            .child(
                div()
                    .id(SharedString::from(format!("data-table-hscroll-{list_id}")))
                    .w_full()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .overflow_x_scroll()
                    .track_scroll(&horizontal_scroll)
                    .child(
                        div()
                            .w(px(total_width))
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(header)
                            .child(div().flex_1().min_h_0().overflow_hidden().child(body)),
                    ),
            )
            .child(navigation_bar)
    }
}
