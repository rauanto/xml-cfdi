//! Errores propios del dominio.

use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{archivo}: {motivo}")]
pub struct ErrorArchivo {
    pub archivo: String,
    pub motivo: String,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ErrorCfdi {
    #[error("XML inválido: {0}")]
    XmlInvalido(String),
    #[error("el archivo no contiene un Comprobante CFDI 3.3 o 4.0")]
    SinComprobante,
    #[error("el Comprobante no contiene conceptos")]
    SinConceptos,
    #[error("monto inválido en {campo}: {valor}")]
    MontoInvalido { campo: &'static str, valor: String },
    #[error("la suma de montos excede el rango decimal en {0}")]
    MontoFueraDeRango(&'static str),
}
