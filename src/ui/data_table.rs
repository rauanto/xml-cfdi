//! Tabla genérica con virtualización de filas y columnas tipadas por ancho/alineación.

use std::sync::Arc;

use gpui::{
    IntoElement, ParentElement, RenderOnce, ScrollHandle, UniformListScrollHandle, Window, div,
    point, prelude::*, px, uniform_list,
};

use super::{theme, theme::Palette};

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
    palette: Palette,
    columns: &'static [Columna],
    rows: Arc<Vec<T>>,
    cell: fn(&T, usize) -> String,
    scroll_handle: UniformListScrollHandle,
    horizontal_scroll: ScrollHandle,
    list_id: &'static str,
}

impl<T: 'static> DataTable<T> {
    pub fn new(
        palette: Palette,
        columns: &'static [Columna],
        rows: Arc<Vec<T>>,
        cell: fn(&T, usize) -> String,
        scroll_handle: UniformListScrollHandle,
        horizontal_scroll: ScrollHandle,
        list_id: &'static str,
    ) -> Self {
        Self {
            palette,
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
    fn render(self, _: &mut Window, _: &mut gpui::App) -> impl IntoElement {
        let palette = self.palette;
        let columns = self.columns;
        let rows = self.rows;
        let cell = self.cell;
        let total_width = columns.iter().map(|column| column.ancho).sum::<f32>();
        let horizontal_scroll = self.horizontal_scroll;
        let row_palette = palette;
        let list = uniform_list(self.list_id, rows.len(), move |range, _, _| {
            range
                .map(|index| {
                    let row = &rows[index];
                    div()
                        .id(("data-row", index))
                        .flex()
                        .items_center()
                        .h(px(theme::ROW_HEIGHT))
                        .w(px(total_width))
                        .border_b_1()
                        .border_color(row_palette.border)
                        .text_size(px(theme::FONT_BODY))
                        .text_color(row_palette.text)
                        .hover(|style| style.bg(row_palette.hover))
                        .children(columns.iter().enumerate().map(|(column_index, column)| {
                            let value = cell(row, column_index);
                            let mut cell = div()
                                .flex_shrink_0()
                                .w(px(column.ancho))
                                .h_full()
                                .flex()
                                .items_center()
                                .px(px(theme::SPACING))
                                .whitespace_nowrap()
                                .truncate();
                            if matches!(column.alineacion, Alineacion::Derecha) {
                                cell = cell.justify_end().text_right();
                            }
                            cell.child(value)
                        }))
                })
                .collect::<Vec<_>>()
        })
        .size_full()
        .track_scroll(self.scroll_handle);

        let header = div()
            .w(px(total_width))
            .h(px(theme::HEADER_HEIGHT))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(palette.border)
            .bg(palette.surface)
            .text_size(px(theme::FONT_SMALL))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(palette.secondary)
            .children(columns.iter().map(|column| {
                let mut header = div()
                    .flex_shrink_0()
                    .w(px(column.ancho))
                    .h_full()
                    .flex()
                    .items_center()
                    .px(px(theme::SPACING))
                    .whitespace_nowrap()
                    .truncate();
                if matches!(column.alineacion, Alineacion::Derecha) {
                    header = header.justify_end().text_right();
                }
                header.child(column.titulo)
            }));

        div()
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .bg(palette.surface)
            .child(
                div()
                    .id("data-table-horizontal-scroll")
                    .w_full()
                    .min_w_0()
                    .h_full()
                    .overflow_x_scroll()
                    .track_scroll(&horizontal_scroll)
                    .child(
                        div()
                            .w(px(total_width))
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(header)
                            .child(div().flex_1().min_h_0().overflow_hidden().child(list)),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .h(px(theme::TABLE_NAV_HEIGHT))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(theme::SPACING))
                    .border_t_1()
                    .border_color(palette.border)
                    .text_size(px(theme::FONT_SMALL))
                    .text_color(palette.secondary)
                    .child("Usa Mayús + rueda o las flechas para ver más columnas")
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(theme::SMALL_SPACING))
                            .child(scroll_button(
                                palette,
                                "table-scroll-left",
                                "← Anterior",
                                horizontal_scroll.offset().x < px(0.),
                                horizontal_scroll.clone(),
                                true,
                            ))
                            .child(scroll_button(
                                palette,
                                "table-scroll-right",
                                "Siguiente →",
                                -horizontal_scroll.max_offset().width
                                    < horizontal_scroll.offset().x,
                                horizontal_scroll,
                                false,
                            )),
                    ),
            )
    }
}

fn scroll_button(
    palette: Palette,
    id: &'static str,
    label: &'static str,
    enabled: bool,
    scroll_handle: ScrollHandle,
    left: bool,
) -> impl IntoElement {
    div()
        .id(id)
        .h(px(theme::CONTROL_HEIGHT - theme::SMALL_SPACING))
        .px(px(theme::SPACING))
        .flex()
        .items_center()
        .rounded(px(theme::CORNER_RADIUS))
        .border_1()
        .border_color(palette.border)
        .text_size(px(theme::FONT_SMALL))
        .text_color(if enabled {
            palette.text
        } else {
            palette.secondary
        })
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(|style| style.bg(palette.hover))
                .on_click(move |_, window, _| {
                    let mut offset = scroll_handle.offset();
                    offset.x = if left {
                        (offset.x + px(320.)).min(px(0.))
                    } else {
                        (offset.x - px(320.)).max(-scroll_handle.max_offset().width)
                    };
                    scroll_handle.set_offset(point(offset.x, offset.y));
                    window.refresh();
                })
        })
        .child(label)
}
