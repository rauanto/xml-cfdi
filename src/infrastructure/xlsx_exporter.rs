//! Adaptador de rust_xlsxwriter para exportar las cuatro hojas del contrato.

use std::path::Path;

use rust_xlsxwriter::{Format, Workbook, Worksheet, XlsxError};

use crate::{
    application::{ports::ExcelExporter, resumen::ResumenProducto},
    domain::{cfdi::CfdiRow, clasificacion::FilaClasificada, error::ErrorArchivo},
};

const CFDI_HEADERS: [&str; 21] = [
    "UUID",
    "Fecha",
    "RFC Emisor",
    "Nombre Emisor",
    "RFC Receptor",
    "Tipo de Comprobante",
    "Subtotal",
    "Descuento",
    "IVA trasladado (002)",
    "ISR retenido (001)",
    "IVA retenido (002)",
    "Total",
    "Uso CFDI",
    "Método de Pago",
    "Forma de Pago",
    "Moneda",
    "Tipo de Cambio",
    "ClaveProdServ",
    "Descripción",
    "Importe",
    "Archivo",
];

const RESUMEN_HEADERS: [&str; 4] = [
    "ClaveProdServ",
    "Descripción",
    "Importe total",
    "Número de conceptos",
];

const CLASIFICACION_HEADERS: [&str; 7] = [
    "Archivo",
    "UUID",
    "Tipo de Comprobante",
    "ClaveProdServ",
    "Descripción",
    "Categoría sugerida",
    "Criterio",
];

const ERRORES_HEADERS: [&str; 2] = ["Archivo", "Motivo"];

pub struct XlsxExporter;

impl ExcelExporter for XlsxExporter {
    type Error = XlsxError;

    fn exportar(
        &self,
        filas: &[CfdiRow],
        resumen: &[ResumenProducto],
        clasificaciones: &[FilaClasificada],
        errores: &[ErrorArchivo],
        destino: &Path,
    ) -> Result<(), Self::Error> {
        let encabezado = Format::new().set_bold();
        let formato_dinero = Format::new().set_num_format("#,##0.00");
        let formato_tipo_cambio = Format::new().set_num_format("#,##0.000000");
        let mut workbook = Workbook::new();

        escribir_cfdi(
            workbook.add_worksheet().set_name("CFDI")?,
            filas,
            &encabezado,
            &formato_dinero,
            &formato_tipo_cambio,
        )?;
        escribir_resumen(
            workbook.add_worksheet().set_name("Resumen por producto")?,
            resumen,
            &encabezado,
            &formato_dinero,
        )?;
        escribir_clasificaciones(
            workbook
                .add_worksheet()
                .set_name("Clasificación sugerida")?,
            clasificaciones,
            &encabezado,
        )?;
        escribir_errores(
            workbook.add_worksheet().set_name("Errores")?,
            errores,
            &encabezado,
        )?;

        workbook.save(destino)
    }
}

fn escribir_cfdi(
    worksheet: &mut Worksheet,
    filas: &[CfdiRow],
    encabezado: &Format,
    formato_dinero: &Format,
    formato_tipo_cambio: &Format,
) -> Result<(), XlsxError> {
    preparar_hoja(
        worksheet,
        &CFDI_HEADERS,
        &[
            38.0, 22.0, 18.0, 30.0, 18.0, 20.0, 15.0, 15.0, 19.0, 18.0, 18.0, 15.0, 14.0, 16.0,
            14.0, 12.0, 16.0, 16.0, 35.0, 15.0, 35.0,
        ],
        encabezado,
        filas.len(),
    )?;

    for (index, fila) in filas.iter().enumerate() {
        let row = index as u32 + 1;
        escribir_texto_opcional(worksheet, row, 0, fila.uuid.as_deref())?;
        escribir_texto_opcional(worksheet, row, 1, fila.fecha.as_deref())?;
        escribir_texto_opcional(worksheet, row, 2, fila.rfc_emisor.as_deref())?;
        escribir_texto_opcional(worksheet, row, 3, fila.nombre_emisor.as_deref())?;
        escribir_texto_opcional(worksheet, row, 4, fila.rfc_receptor.as_deref())?;
        escribir_texto_opcional(worksheet, row, 5, fila.tipo_comprobante.as_deref())?;
        escribir_decimal_opcional(worksheet, row, 6, fila.subtotal, formato_dinero)?;
        escribir_decimal_opcional(worksheet, row, 7, fila.descuento, formato_dinero)?;
        escribir_decimal_opcional(worksheet, row, 8, fila.iva_trasladado_002, formato_dinero)?;
        escribir_decimal_opcional(worksheet, row, 9, fila.isr_retenido_001, formato_dinero)?;
        escribir_decimal_opcional(worksheet, row, 10, fila.iva_retenido_002, formato_dinero)?;
        escribir_decimal_opcional(worksheet, row, 11, fila.total, formato_dinero)?;
        escribir_texto_opcional(worksheet, row, 12, fila.uso_cfdi.as_deref())?;
        escribir_texto_opcional(worksheet, row, 13, fila.metodo_pago.as_deref())?;
        escribir_texto_opcional(worksheet, row, 14, fila.forma_pago.as_deref())?;
        escribir_texto_opcional(worksheet, row, 15, fila.moneda.as_deref())?;
        escribir_decimal_opcional(worksheet, row, 16, fila.tipo_cambio, formato_tipo_cambio)?;
        escribir_texto_opcional(worksheet, row, 17, fila.clave_prod_serv.as_deref())?;
        escribir_texto_opcional(worksheet, row, 18, fila.descripcion.as_deref())?;
        escribir_decimal_opcional(worksheet, row, 19, fila.importe_concepto, formato_dinero)?;
        worksheet.write_string(row, 20, &fila.archivo_origen)?;
    }

    Ok(())
}

fn escribir_resumen(
    worksheet: &mut Worksheet,
    filas: &[ResumenProducto],
    encabezado: &Format,
    formato_dinero: &Format,
) -> Result<(), XlsxError> {
    preparar_hoja(
        worksheet,
        &RESUMEN_HEADERS,
        &[16.0, 35.0, 18.0, 20.0],
        encabezado,
        filas.len(),
    )?;

    for (index, fila) in filas.iter().enumerate() {
        let row = index as u32 + 1;
        escribir_texto_opcional(worksheet, row, 0, fila.clave_prod_serv.as_deref())?;
        escribir_texto_opcional(worksheet, row, 1, fila.descripcion.as_deref())?;
        worksheet.write_with_format(row, 2, fila.importe_total, formato_dinero)?;
        let num_conceptos = u32::try_from(fila.num_conceptos).map_err(|_| {
            XlsxError::ParameterError("el número de conceptos excede el límite de Excel".into())
        })?;
        worksheet.write_number(row, 3, num_conceptos)?;
    }

    Ok(())
}

fn escribir_clasificaciones(
    worksheet: &mut Worksheet,
    filas: &[FilaClasificada],
    encabezado: &Format,
) -> Result<(), XlsxError> {
    preparar_hoja(
        worksheet,
        &CLASIFICACION_HEADERS,
        &[35.0, 38.0, 20.0, 16.0, 35.0, 22.0, 48.0],
        encabezado,
        filas.len(),
    )?;

    for (index, clasificada) in filas.iter().enumerate() {
        let row = index as u32 + 1;
        escribir_texto_opcional(worksheet, row, 0, Some(&clasificada.fila.archivo_origen))?;
        escribir_texto_opcional(worksheet, row, 1, clasificada.fila.uuid.as_deref())?;
        escribir_texto_opcional(
            worksheet,
            row,
            2,
            clasificada.fila.tipo_comprobante.as_deref(),
        )?;
        escribir_texto_opcional(
            worksheet,
            row,
            3,
            clasificada.fila.clave_prod_serv.as_deref(),
        )?;
        escribir_texto_opcional(worksheet, row, 4, clasificada.fila.descripcion.as_deref())?;
        worksheet.write_string(row, 5, clasificada.clasificacion.categoria.nombre())?;
        worksheet.write_string(row, 6, &clasificada.clasificacion.criterio)?;
    }

    Ok(())
}

fn escribir_errores(
    worksheet: &mut Worksheet,
    errores: &[ErrorArchivo],
    encabezado: &Format,
) -> Result<(), XlsxError> {
    preparar_hoja(
        worksheet,
        &ERRORES_HEADERS,
        &[38.0, 70.0],
        encabezado,
        errores.len(),
    )?;

    for (index, error) in errores.iter().enumerate() {
        let row = index as u32 + 1;
        worksheet.write_string(row, 0, &error.archivo)?;
        worksheet.write_string(row, 1, &error.motivo)?;
    }

    Ok(())
}

fn preparar_hoja(
    worksheet: &mut Worksheet,
    encabezados: &[&str],
    anchos: &[f64],
    formato_encabezado: &Format,
    filas: usize,
) -> Result<(), XlsxError> {
    for (columna, encabezado) in encabezados.iter().enumerate() {
        let columna = columna as u16;
        worksheet.write_string_with_format(0, columna, *encabezado, formato_encabezado)?;
        worksheet.set_column_width(columna, anchos[columna as usize])?;
    }
    worksheet.set_freeze_panes(1, 0)?;
    worksheet.autofilter(0, 0, filas as u32, encabezados.len() as u16 - 1)?;
    Ok(())
}

fn escribir_texto_opcional(
    worksheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: Option<&str>,
) -> Result<(), XlsxError> {
    if let Some(value) = value {
        worksheet.write_string(row, col, value)?;
    }
    Ok(())
}

fn escribir_decimal_opcional(
    worksheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: Option<rust_decimal::Decimal>,
    format: &Format,
) -> Result<(), XlsxError> {
    if let Some(value) = value {
        worksheet.write_with_format(row, col, value, format)?;
    }
    Ok(())
}
