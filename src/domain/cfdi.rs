//! Tipos de datos de CFDI y conceptos.

use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq)]
pub struct CfdiRow {
    pub uuid: Option<String>,
    pub fecha: Option<String>,
    pub rfc_emisor: Option<String>,
    pub nombre_emisor: Option<String>,
    pub rfc_receptor: Option<String>,
    pub tipo_comprobante: Option<String>,
    pub subtotal: Option<Decimal>,
    pub descuento: Option<Decimal>,
    pub iva_trasladado_002: Option<Decimal>,
    pub isr_retenido_001: Option<Decimal>,
    pub iva_retenido_002: Option<Decimal>,
    pub total: Option<Decimal>,
    pub uso_cfdi: Option<String>,
    pub metodo_pago: Option<String>,
    pub forma_pago: Option<String>,
    pub moneda: Option<String>,
    pub tipo_cambio: Option<Decimal>,
    pub clave_prod_serv: Option<String>,
    pub descripcion: Option<String>,
    pub importe_concepto: Option<Decimal>,
    pub archivo_origen: String,
}
