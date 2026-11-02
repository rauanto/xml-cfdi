//! Adaptador de quick-xml para extraer datos de CFDI 3.3 y 4.0.

use std::{collections::HashMap, str::FromStr};

use quick_xml::{
    XmlVersion,
    events::{BytesStart, Event},
    name::{Namespace, ResolveResult},
    reader::NsReader,
};
use rust_decimal::Decimal;

use crate::{
    application::ports::CfdiParser,
    domain::{cfdi::CfdiRow, error::ErrorCfdi},
};

const CFDI_33_NAMESPACE: &str = "http://www.sat.gob.mx/cfd/3";
const CFDI_40_NAMESPACE: &str = "http://www.sat.gob.mx/cfd/4";
const TFD_NAMESPACE: &str = "http://www.sat.gob.mx/TimbreFiscalDigital";

#[derive(Default)]
pub struct XmlCfdiParser;

#[derive(Default)]
struct Comprobante {
    uuid: Option<String>,
    fecha: Option<String>,
    rfc_emisor: Option<String>,
    nombre_emisor: Option<String>,
    rfc_receptor: Option<String>,
    tipo_comprobante: Option<String>,
    subtotal: Option<Decimal>,
    descuento: Option<Decimal>,
    iva_trasladado_002: Option<Decimal>,
    isr_retenido_001: Option<Decimal>,
    iva_retenido_002: Option<Decimal>,
    total: Option<Decimal>,
    uso_cfdi: Option<String>,
    metodo_pago: Option<String>,
    forma_pago: Option<String>,
    moneda: Option<String>,
    tipo_cambio: Option<Decimal>,
}

#[derive(Default)]
struct Concepto {
    clave_prod_serv: Option<String>,
    descripcion: Option<String>,
    importe: Option<Decimal>,
}

#[derive(Default)]
struct AcumuladorImpuesto {
    total: Option<Decimal>,
    encontrado: bool,
    incompleto: bool,
}

impl AcumuladorImpuesto {
    fn agregar(&mut self, importe: Option<Decimal>, campo: &'static str) -> Result<(), ErrorCfdi> {
        self.encontrado = true;
        if let Some(importe) = importe {
            self.total = Some(match self.total {
                Some(total) => total
                    .checked_add(importe)
                    .ok_or(ErrorCfdi::MontoFueraDeRango(campo))?,
                None => importe,
            });
        } else {
            self.incompleto = true;
        }
        Ok(())
    }

    fn valor(&self) -> Option<Decimal> {
        if self.encontrado && !self.incompleto {
            self.total
        } else {
            None
        }
    }
}

#[derive(Default)]
struct Impuestos {
    iva_trasladado_002: AcumuladorImpuesto,
    isr_retenido_001: AcumuladorImpuesto,
    iva_retenido_002: AcumuladorImpuesto,
}

struct Elemento {
    nombre: Vec<u8>,
    namespace_cfdi: bool,
}

impl CfdiParser for XmlCfdiParser {
    fn parsear(&self, nombre: &str, xml: &[u8]) -> Result<Vec<CfdiRow>, ErrorCfdi> {
        let mut reader = NsReader::from_reader(xml);
        let mut stack = Vec::new();
        let mut root_seen = false;
        let mut root_namespace = None;
        let mut comprobante = Comprobante::default();
        let mut impuestos = Impuestos::default();
        let mut conceptos = Vec::new();

        loop {
            let (namespace, event) = reader
                .read_resolved_event()
                .map_err(|error| ErrorCfdi::XmlInvalido(error.to_string()))?;

            match event {
                Event::Start(element) => {
                    let name = element.local_name().as_ref().as_bytes().to_vec();
                    let current_namespace = namespace_uri(&namespace);
                    if stack.is_empty() {
                        if root_seen {
                            return Err(ErrorCfdi::XmlInvalido(
                                "el documento tiene más de un elemento raíz".to_owned(),
                            ));
                        }
                        root_seen = true;
                        if name == b"Comprobante" {
                            root_namespace = supported_cfdi_namespace(current_namespace);
                        }
                    }

                    let is_cfdi =
                        root_namespace.is_some_and(|root| current_namespace == Some(root));
                    let attributes = leer_atributos(&element)?;
                    procesar_elemento(
                        &name,
                        is_cfdi,
                        current_namespace == Some(TFD_NAMESPACE),
                        &stack,
                        &attributes,
                        &mut comprobante,
                        &mut impuestos,
                        &mut conceptos,
                    )?;
                    stack.push(Elemento {
                        nombre: name,
                        namespace_cfdi: is_cfdi,
                    });
                }
                Event::Empty(element) => {
                    let name = element.local_name().as_ref().as_bytes().to_vec();
                    let current_namespace = namespace_uri(&namespace);
                    if stack.is_empty() {
                        if root_seen {
                            return Err(ErrorCfdi::XmlInvalido(
                                "el documento tiene más de un elemento raíz".to_owned(),
                            ));
                        }
                        root_seen = true;
                        if name == b"Comprobante" {
                            root_namespace = supported_cfdi_namespace(current_namespace);
                        }
                    }

                    let is_cfdi =
                        root_namespace.is_some_and(|root| current_namespace == Some(root));
                    let attributes = leer_atributos(&element)?;
                    procesar_elemento(
                        &name,
                        is_cfdi,
                        current_namespace == Some(TFD_NAMESPACE),
                        &stack,
                        &attributes,
                        &mut comprobante,
                        &mut impuestos,
                        &mut conceptos,
                    )?;
                }
                Event::End(_) => {
                    if stack.pop().is_none() {
                        return Err(ErrorCfdi::XmlInvalido(
                            "elemento de cierre sin elemento de apertura".to_owned(),
                        ));
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        if !stack.is_empty() {
            return Err(ErrorCfdi::XmlInvalido(
                "el documento tiene elementos sin cerrar".to_owned(),
            ));
        }
        if !root_seen || root_namespace.is_none() {
            return Err(ErrorCfdi::SinComprobante);
        }
        if conceptos.is_empty() {
            return Err(ErrorCfdi::SinConceptos);
        }

        comprobante.iva_trasladado_002 = impuestos.iva_trasladado_002.valor();
        comprobante.isr_retenido_001 = impuestos.isr_retenido_001.valor();
        comprobante.iva_retenido_002 = impuestos.iva_retenido_002.valor();

        Ok(conceptos
            .into_iter()
            .map(|concepto| CfdiRow {
                uuid: comprobante.uuid.clone(),
                fecha: comprobante.fecha.clone(),
                rfc_emisor: comprobante.rfc_emisor.clone(),
                nombre_emisor: comprobante.nombre_emisor.clone(),
                rfc_receptor: comprobante.rfc_receptor.clone(),
                tipo_comprobante: comprobante.tipo_comprobante.clone(),
                subtotal: comprobante.subtotal,
                descuento: comprobante.descuento,
                iva_trasladado_002: comprobante.iva_trasladado_002,
                isr_retenido_001: comprobante.isr_retenido_001,
                iva_retenido_002: comprobante.iva_retenido_002,
                total: comprobante.total,
                uso_cfdi: comprobante.uso_cfdi.clone(),
                metodo_pago: comprobante.metodo_pago.clone(),
                forma_pago: comprobante.forma_pago.clone(),
                moneda: comprobante.moneda.clone(),
                tipo_cambio: comprobante.tipo_cambio,
                clave_prod_serv: concepto.clave_prod_serv,
                descripcion: concepto.descripcion,
                importe_concepto: concepto.importe,
                archivo_origen: nombre.to_owned(),
            })
            .collect())
    }
}

fn namespace_uri<'ns>(namespace: &ResolveResult<'ns>) -> Option<&'ns str> {
    match namespace {
        ResolveResult::Bound(Namespace(uri)) => Some(*uri),
        ResolveResult::Unbound | ResolveResult::Unknown(_) => None,
    }
}

fn supported_cfdi_namespace(namespace: Option<&str>) -> Option<&'static str> {
    match namespace {
        Some(CFDI_33_NAMESPACE) => Some(CFDI_33_NAMESPACE),
        Some(CFDI_40_NAMESPACE) => Some(CFDI_40_NAMESPACE),
        _ => None,
    }
}

fn leer_atributos(element: &BytesStart<'_>) -> Result<HashMap<Vec<u8>, String>, ErrorCfdi> {
    element
        .attributes()
        .map(|attribute| {
            let attribute = attribute.map_err(|error| ErrorCfdi::XmlInvalido(error.to_string()))?;
            let value = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|error| ErrorCfdi::XmlInvalido(error.to_string()))?;
            Ok((
                attribute.key.as_ref().as_bytes().to_vec(),
                value.into_owned(),
            ))
        })
        .collect()
}

fn valor_atributo(attributes: &HashMap<Vec<u8>, String>, name: &[u8]) -> Option<String> {
    attributes.get(name).cloned()
}

fn decimal_atributo(
    attributes: &HashMap<Vec<u8>, String>,
    name: &[u8],
    campo: &'static str,
) -> Result<Option<Decimal>, ErrorCfdi> {
    attributes
        .get(name)
        .map(|value| {
            Decimal::from_str(value).map_err(|_| ErrorCfdi::MontoInvalido {
                campo,
                valor: value.clone(),
            })
        })
        .transpose()
}

fn ruta_cfdi(stack: &[Elemento], expected: &[&[u8]]) -> bool {
    stack.len() == expected.len()
        && stack
            .iter()
            .zip(expected)
            .all(|(element, name)| element.namespace_cfdi && element.nombre == *name)
}

#[allow(clippy::too_many_arguments)]
fn procesar_elemento(
    name: &[u8],
    is_cfdi: bool,
    is_tfd: bool,
    stack: &[Elemento],
    attributes: &HashMap<Vec<u8>, String>,
    comprobante: &mut Comprobante,
    impuestos: &mut Impuestos,
    conceptos: &mut Vec<Concepto>,
) -> Result<(), ErrorCfdi> {
    if is_cfdi && stack.is_empty() && name == b"Comprobante" {
        comprobante.fecha = valor_atributo(attributes, b"Fecha");
        comprobante.tipo_comprobante = valor_atributo(attributes, b"TipoDeComprobante");
        comprobante.subtotal = decimal_atributo(attributes, b"SubTotal", "SubTotal")?;
        comprobante.descuento = decimal_atributo(attributes, b"Descuento", "Descuento")?;
        comprobante.total = decimal_atributo(attributes, b"Total", "Total")?;
        comprobante.moneda = valor_atributo(attributes, b"Moneda");
        comprobante.tipo_cambio = decimal_atributo(attributes, b"TipoCambio", "TipoCambio")?;
        comprobante.metodo_pago = valor_atributo(attributes, b"MetodoPago");
        comprobante.forma_pago = valor_atributo(attributes, b"FormaPago");
    } else if is_cfdi && ruta_cfdi(stack, &[b"Comprobante"]) && name == b"Emisor" {
        comprobante.rfc_emisor = valor_atributo(attributes, b"Rfc");
        comprobante.nombre_emisor = valor_atributo(attributes, b"Nombre");
    } else if is_cfdi && ruta_cfdi(stack, &[b"Comprobante"]) && name == b"Receptor" {
        comprobante.rfc_receptor = valor_atributo(attributes, b"Rfc");
        comprobante.uso_cfdi = valor_atributo(attributes, b"UsoCFDI");
    } else if is_cfdi && ruta_cfdi(stack, &[b"Comprobante", b"Conceptos"]) && name == b"Concepto" {
        conceptos.push(Concepto {
            clave_prod_serv: valor_atributo(attributes, b"ClaveProdServ"),
            descripcion: valor_atributo(attributes, b"Descripcion"),
            importe: decimal_atributo(attributes, b"Importe", "Importe de concepto")?,
        });
    } else if is_tfd
        && ruta_cfdi(stack, &[b"Comprobante", b"Complemento"])
        && name == b"TimbreFiscalDigital"
    {
        comprobante.uuid = valor_atributo(attributes, b"UUID");
    } else if is_cfdi
        && ruta_cfdi(stack, &[b"Comprobante", b"Impuestos", b"Traslados"])
        && name == b"Traslado"
    {
        if valor_atributo(attributes, b"Impuesto").as_deref() == Some("002") {
            impuestos.iva_trasladado_002.agregar(
                decimal_atributo(attributes, b"Importe", "Importe de IVA trasladado")?,
                "IVA trasladado",
            )?;
        }
    } else if is_cfdi
        && ruta_cfdi(stack, &[b"Comprobante", b"Impuestos", b"Retenciones"])
        && name == b"Retencion"
    {
        match valor_atributo(attributes, b"Impuesto").as_deref() {
            Some("001") => impuestos.isr_retenido_001.agregar(
                decimal_atributo(attributes, b"Importe", "Importe de ISR retenido")?,
                "ISR retenido",
            )?,
            Some("002") => impuestos.iva_retenido_002.agregar(
                decimal_atributo(attributes, b"Importe", "Importe de IVA retenido")?,
                "IVA retenido",
            )?,
            _ => {}
        }
    }

    Ok(())
}
