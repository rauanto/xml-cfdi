//! Agrupación exacta de importes de conceptos por producto.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use thiserror::Error;

use crate::domain::cfdi::CfdiRow;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumenProducto {
    pub clave_prod_serv: Option<String>,
    pub descripcion: Option<String>,
    pub importe_total: Decimal,
    pub num_conceptos: usize,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ErrorResumen {
    #[error(
        "la suma excede el rango decimal para producto {clave_prod_serv:?}, descripción {descripcion:?}"
    )]
    ImporteFueraDeRango {
        clave_prod_serv: Option<String>,
        descripcion: Option<String>,
    },
    #[error("el número de conceptos excede el rango admitido")]
    DemasiadosConceptos,
}

pub fn resumir(filas: &[CfdiRow]) -> Result<Vec<ResumenProducto>, ErrorResumen> {
    let mut grupos: BTreeMap<(Option<String>, Option<String>), ResumenProducto> = BTreeMap::new();

    for fila in filas {
        let clave = (fila.clave_prod_serv.clone(), fila.descripcion.clone());
        let resumen = grupos
            .entry(clave.clone())
            .or_insert_with(|| ResumenProducto {
                clave_prod_serv: clave.0.clone(),
                descripcion: clave.1.clone(),
                importe_total: Decimal::ZERO,
                num_conceptos: 0,
            });
        resumen.num_conceptos = resumen
            .num_conceptos
            .checked_add(1)
            .ok_or(ErrorResumen::DemasiadosConceptos)?;
        if let Some(importe) = fila.importe_concepto {
            resumen.importe_total = match resumen.importe_total.checked_add(importe) {
                Some(total) => total,
                None => {
                    return Err(ErrorResumen::ImporteFueraDeRango {
                        clave_prod_serv: clave.0,
                        descripcion: clave.1,
                    });
                }
            };
        }
    }

    let mut resumen: Vec<_> = grupos.into_values().collect();
    resumen.sort_by(|izquierda, derecha| {
        derecha
            .importe_total
            .cmp(&izquierda.importe_total)
            .then_with(|| izquierda.clave_prod_serv.cmp(&derecha.clave_prod_serv))
            .then_with(|| izquierda.descripcion.cmp(&derecha.descripcion))
    });
    Ok(resumen)
}
