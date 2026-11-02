//! Caso de uso para leer, parsear y resumir CFDI de un archivo ZIP.

use std::path::Path;

use crate::domain::{
    cfdi::CfdiRow,
    clasificacion::{FilaClasificada, clasificar},
    error::ErrorArchivo,
};

use super::{
    ports::{CfdiParser, ZipReader},
    resumen::{ResumenProducto, resumir},
};

#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoProceso {
    pub filas: Vec<CfdiRow>,
    pub resumen: Vec<ResumenProducto>,
    pub clasificaciones: Vec<FilaClasificada>,
    pub errores: Vec<ErrorArchivo>,
    pub total_xml_leidos: usize,
    pub total_cfdi_ok: usize,
}

pub struct ProcesarZip<Z, P> {
    zip_reader: Z,
    cfdi_parser: P,
}

impl<Z: ZipReader, P: CfdiParser> ProcesarZip<Z, P> {
    pub fn new(zip_reader: Z, cfdi_parser: P) -> Self {
        Self {
            zip_reader,
            cfdi_parser,
        }
    }

    pub fn ejecutar(
        &self,
        ruta_zip: impl AsRef<Path>,
        rfc_propio: Option<String>,
    ) -> ResultadoProceso {
        self.ejecutar_con_progreso(ruta_zip, rfc_propio, |_, _| {})
    }

    pub fn ejecutar_con_progreso<F>(
        &self,
        ruta_zip: impl AsRef<Path>,
        rfc_propio: Option<String>,
        mut progreso: F,
    ) -> ResultadoProceso
    where
        F: FnMut(usize, usize),
    {
        let ruta_zip = ruta_zip.as_ref();
        let nombre_zip = ruta_zip.to_string_lossy().into_owned();
        let (entradas, mut errores) = match self.zip_reader.leer_xmls(ruta_zip) {
            Ok(entradas) => (entradas, Vec::new()),
            Err(error) => (Vec::new(), vec![error]),
        };
        let total_xml_leidos = entradas.len();
        let mut filas = Vec::new();
        let mut total_cfdi_ok = 0;

        progreso(0, total_xml_leidos);
        for (indice, entrada) in entradas.into_iter().enumerate() {
            match entrada {
                Ok((nombre, xml)) => match self.cfdi_parser.parsear(&nombre, &xml) {
                    Ok(mut filas_xml) => {
                        total_cfdi_ok += 1;
                        filas.append(&mut filas_xml);
                    }
                    Err(error) => errores.push(ErrorArchivo {
                        archivo: nombre,
                        motivo: error.to_string(),
                    }),
                },
                Err(error) => errores.push(error),
            }
            progreso(indice + 1, total_xml_leidos);
        }

        let resumen = match resumir(&filas) {
            Ok(resumen) => resumen,
            Err(error) => {
                errores.push(ErrorArchivo {
                    archivo: nombre_zip,
                    motivo: error.to_string(),
                });
                Vec::new()
            }
        };
        let clasificaciones = filas
            .iter()
            .cloned()
            .map(|fila| FilaClasificada {
                clasificacion: clasificar(&fila, rfc_propio.as_deref()),
                fila,
            })
            .collect();

        ResultadoProceso {
            filas,
            resumen,
            clasificaciones,
            errores,
            total_xml_leidos,
            total_cfdi_ok,
        }
    }
}
