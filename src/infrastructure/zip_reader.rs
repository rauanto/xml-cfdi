//! Adaptador de zip para leer entradas XML y reportar errores por archivo.

use std::{fs::File, io::Read, path::Path};

use zip::ZipArchive;

use crate::{
    application::ports::{ResultadoEntradaXml, ZipReader},
    domain::error::ErrorArchivo,
};

#[derive(Default)]
pub struct ZipXmlReader;

impl ZipReader for ZipXmlReader {
    fn leer_xmls(&self, ruta_zip: &Path) -> Result<Vec<ResultadoEntradaXml>, ErrorArchivo> {
        let nombre_zip = ruta_zip.display().to_string();
        let archivo = match File::open(ruta_zip) {
            Ok(archivo) => archivo,
            Err(error) => {
                return Err(ErrorArchivo {
                    archivo: nombre_zip,
                    motivo: error.to_string(),
                });
            }
        };
        let mut archive = match ZipArchive::new(archivo) {
            Ok(archive) => archive,
            Err(error) => {
                return Err(ErrorArchivo {
                    archivo: nombre_zip,
                    motivo: error.to_string(),
                });
            }
        };
        let mut xmls = Vec::new();

        for index in 0..archive.len() {
            let mut file = match archive.by_index(index) {
                Ok(file) => file,
                Err(error) => {
                    xmls.push(Err(ErrorArchivo {
                        archivo: format!("entrada {index}"),
                        motivo: error.to_string(),
                    }));
                    continue;
                }
            };

            let nombre = match file.name() {
                Ok(nombre) => nombre.into_owned(),
                Err(error) => {
                    xmls.push(Err(ErrorArchivo {
                        archivo: format!("entrada {index}"),
                        motivo: error.to_string(),
                    }));
                    continue;
                }
            };

            if file.is_dir() || contiene_directorio_macosx(&nombre) || !es_archivo_xml(&nombre) {
                continue;
            }

            let mut contenido = Vec::new();
            match file.read_to_end(&mut contenido) {
                Ok(_) => xmls.push(Ok((nombre, contenido))),
                Err(error) => xmls.push(Err(ErrorArchivo {
                    archivo: nombre,
                    motivo: error.to_string(),
                })),
            }
        }

        Ok(xmls)
    }
}

fn contiene_directorio_macosx(nombre: &str) -> bool {
    nombre.split('/').any(|componente| componente == "__MACOSX")
}

fn es_archivo_xml(nombre: &str) -> bool {
    Path::new(nombre)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
}
