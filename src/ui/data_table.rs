//! Adaptador de las cuatro vistas al componente Table de gpuikit.

use std::sync::Arc;

use gpui::{
    IntoElement, ParentElement, RenderOnce, ScrollHandle, StatefulInteractiveElement,
    UniformListScrollHandle, Window, div, prelude::*, px, uniform_list,
};
use gpuikit::theme::{ActiveTheme, Themeable};

use super::theme;

#[derive(Clone, Copy)]
pub enum Alineacion {
    Izquierda,
    Derecha,
}

#[derive(Clone, Copy)]
pub struct Columna {
    pub titulo: &'static str,
    pub ancho: f32,
    pub alineacion: Alineacion,
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
    col("Descripción", theme::WIDTH_DESCRIPTION),
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
    col("Criterio", theme::WIDTH_CRITERION),
];

pub const ERROR_COLUMNS: [Columna; 2] = [
    col("Archivo", theme::WIDTH_FILE),
    col("Motivo", theme::WIDTH_ERROR),
];

const fn col(titulo: &'static str, ancho: f32) -> Columna {
    Columna {
        titulo,
        ancho,
        alineacion: Alineacion::Izquierda,
    }
}

const fn col_r(titulo: &'static str, ancho: f32) -> Columna {
    Columna {
        titulo,
        ancho,
        alineacion: Alineacion::Derecha,
    }
}

#[derive(IntoElement)]
pub struct DataTable<T: 'static> {
    columns: &'static [Columna],
    rows: Arc<Vec<T>>,
    cell: fn(&T, usize) -> String,
    scroll_handle: UniformListScrollHandle,
    horizontal_scroll: ScrollHandle,
    list_id: &'static str,
}

impl<T: 'static> DataTable<T> {
    pub fn new(
        columns: &'static [Columna],
        rows: Arc<Vec<T>>,
        cell: fn(&T, usize) -> String,
        scroll_handle: UniformListScrollHandle,
        horizontal_scroll: ScrollHandle,
        list_id: &'static str,
    ) -> Self {
        Self {
            columns,
            rows,
            cell,
            scroll_handle,
            horizontal_scroll,
            list_id,
        }
    }
}

impl<T: 'static> RenderOnce for DataTable<T> {
    fn render(self, window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let columns = self.columns;
        let rows = self.rows;
        let cell = self.cell;
        let scroll_handle = self.scroll_handle;
        let horizontal_scroll = self.horizontal_scroll;
        let list_id = self.list_id;
        let max_height = (window.bounds().size.height - px(280.)).max(px(160.));
        let total_width = columns.iter().map(|column| column.ancho).sum::<f32>();
        let theme = cx.theme();
        let border = theme.border();
        let surface = theme.surface();
        let foreground = theme.fg();
        let muted_foreground = theme.fg_muted();
        let row_hover = theme.surface_tertiary();

        let body = if rows.is_empty() {
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .items_center()
                .justify_center()
                .text_color(muted_foreground)
                .child("Sin filas para mostrar")
                .into_any_element()
        } else {
            uniform_list(list_id, rows.len(), move |range, _, _| {
                range
                    .map(|index| {
                        let item = &rows[index];
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
                            .hover(|style| style.bg(row_hover))
                            .children(columns.iter().enumerate().map(|(column_index, column)| {
                                let mut cell_element = div()
                                    .flex_shrink_0()
                                    .w(px(column.ancho))
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .px(px(8.))
                                    .whitespace_nowrap()
                                    .truncate();
                                if matches!(column.alineacion, Alineacion::Derecha) {
                                    cell_element = cell_element.justify_end().text_right();
                                }
                                cell_element.child(cell(item, column_index))
                            }))
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
            .bg(surface)
            .text_size(px(12.))
            .text_color(muted_foreground)
            .children(columns.iter().map(|column| {
                let mut header_cell = div()
                    .flex_shrink_0()
                    .w(px(column.ancho))
                    .h_full()
                    .flex()
                    .items_center()
                    .px(px(8.))
                    .whitespace_nowrap()
                    .truncate();
                if matches!(column.alineacion, Alineacion::Derecha) {
                    header_cell = header_cell.justify_end().text_right();
                }
                header_cell.child(column.titulo)
            }));

        div()
            .id("data-table-horizontal-scroll")
            .w_full()
            .h_full()
            .min_w_0()
            .min_h_0()
            .max_h(max_height)
            .overflow_x_scroll()
            .scrollbar_width(px(8.))
            .track_scroll(&horizontal_scroll)
            .bg(surface)
            .child(
                div()
                    .w(px(total_width))
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(div().flex_1().min_h_0().overflow_hidden().child(body)),
            )
    }
}
