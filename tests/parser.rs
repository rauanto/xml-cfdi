use std::io::{Cursor, Write};

use cfdi_zip_processor::{
    application::ports::{CfdiParser, ZipReader},
    domain::error::ErrorCfdi,
    infrastructure::{xml_parser::XmlCfdiParser, zip_reader::ZipXmlReader},
};
use rust_decimal::Decimal;
use tempfile::NamedTempFile;
use zip::{
    CompressionMethod,
    write::{SimpleFileOptions, ZipWriter},
};

fn decimal(value: &str) -> Decimal {
    value.parse().expect("fixture decimal is valid")
}

fn parsear_fixture(
    nombre: &str,
    xml: &[u8],
) -> Result<Vec<cfdi_zip_processor::domain::cfdi::CfdiRow>, ErrorCfdi> {
    XmlCfdiParser.parsear(nombre, xml)
}

#[test]
fn cfdi_40_crea_una_fila_por_concepto_y_repite_impuestos() {
    let rows = parsear_fixture(
        "recibidos/factura_arbitraria.xml",
        include_bytes!("fixtures/cfdi_40_tres_conceptos_impuestos.xml"),
    )
    .expect("CFDI 4.0 fixture parses");

    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[0].uuid.as_deref(),
        Some("123e4567-e89b-12d3-a456-426614174000")
    );
    assert_eq!(rows[0].nombre_emisor.as_deref(), Some("Empresa & Socios"));
    assert_eq!(rows[0].uso_cfdi.as_deref(), Some("G03"));
    assert_eq!(rows[0].subtotal, Some(decimal("100.00")));
    assert_eq!(rows[0].descuento, Some(decimal("5.00")));
    assert_eq!(rows[0].iva_trasladado_002, Some(decimal("16.00")));
    assert_eq!(rows[0].isr_retenido_001, Some(decimal("2.00")));
    assert_eq!(rows[0].iva_retenido_002, Some(decimal("3.00")));
    assert_eq!(rows[0].total, Some(decimal("109.00")));
    assert_eq!(rows[0].archivo_origen, "recibidos/factura_arbitraria.xml");
    assert_eq!(rows[0].clave_prod_serv.as_deref(), Some("10101500"));
    assert_eq!(rows[0].descripcion.as_deref(), Some("Producto uno"));
    assert_eq!(rows[0].importe_concepto, Some(decimal("10.00")));
    assert_eq!(rows[1].descripcion.as_deref(), Some("Producto dos"));
    assert_eq!(rows[1].importe_concepto, Some(decimal("40.00")));
    assert_eq!(rows[2].descripcion.as_deref(), Some("Producto tres"));
    assert_eq!(rows[2].importe_concepto, Some(decimal("50.00")));

    for row in &rows {
        assert_eq!(row.iva_trasladado_002, Some(decimal("16.00")));
        assert_eq!(row.isr_retenido_001, Some(decimal("2.00")));
        assert_eq!(row.iva_retenido_002, Some(decimal("3.00")));
    }
}

#[test]
fn cfdi_33_extrae_campos_y_montos_decimales() {
    let rows = parsear_fixture("cfdi_33.xml", include_bytes!("fixtures/cfdi_33.xml"))
        .expect("CFDI 3.3 fixture parses");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].tipo_comprobante.as_deref(), Some("E"));
    assert_eq!(rows[0].rfc_emisor.as_deref(), Some("AAA010101AAA"));
    assert_eq!(rows[0].rfc_receptor.as_deref(), Some("BBB010101BBB"));
    assert_eq!(rows[0].subtotal, Some(decimal("250.50")));
    assert_eq!(rows[0].total, Some(decimal("290.58")));
    assert_eq!(rows[0].uso_cfdi.as_deref(), Some("G02"));
}

#[test]
fn tipos_p_n_y_t_se_extraen_sin_tratamiento_especial() {
    for (nombre, xml, tipo_esperado) in [
        (
            "tipo_p.xml",
            include_bytes!("fixtures/tipo_p.xml").as_slice(),
            "P",
        ),
        (
            "tipo_n.xml",
            include_bytes!("fixtures/tipo_n.xml").as_slice(),
            "N",
        ),
        (
            "tipo_t.xml",
            include_bytes!("fixtures/campos_faltantes_tipo_t.xml").as_slice(),
            "T",
        ),
    ] {
        let rows = parsear_fixture(nombre, xml).expect("CFDI type fixture parses");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].tipo_comprobante.as_deref(), Some(tipo_esperado));
    }
}

#[test]
fn campos_ausentes_y_timbre_ausente_se_conservan_como_none() {
    let rows = parsear_fixture(
        "campos_faltantes_tipo_t.xml",
        include_bytes!("fixtures/campos_faltantes_tipo_t.xml"),
    )
    .expect("CFDI without optional attributes parses");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].uuid, None);
    assert_eq!(rows[0].descuento, None);
    assert_eq!(rows[0].tipo_cambio, None);
    assert_eq!(rows[0].metodo_pago, None);
}

#[test]
fn namespace_se_resuelve_por_uri_no_por_prefijo() {
    let rows = parsear_fixture(
        "prefijo_no_estandar.xml",
        include_bytes!("fixtures/prefijo_no_estandar.xml"),
    )
    .expect("CFDI with a custom namespace prefix parses");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].rfc_emisor.as_deref(), Some("CCC010101CCC"));
    assert_eq!(rows[0].descripcion.as_deref(), Some("Equipo"));
    assert_eq!(rows[0].uuid.as_deref(), Some("custom-prefix-uuid"));
}

#[test]
fn xml_corrupto_y_documento_no_cfdi_devuelven_error() {
    assert!(matches!(
        parsear_fixture(
            "xml_corrupto.xml",
            include_bytes!("fixtures/xml_corrupto.xml")
        ),
        Err(ErrorCfdi::XmlInvalido(_))
    ));
    assert_eq!(
        parsear_fixture("no_cfdi.xml", include_bytes!("fixtures/no_cfdi.xml")),
        Err(ErrorCfdi::SinComprobante)
    );
}

#[test]
fn xml_truncado_devuelve_error_sin_panico() {
    let xml = br#"<c:Comprobante xmlns:c="http://www.sat.gob.mx/cfd/4"><c:Conceptos>"#;

    assert!(matches!(
        parsear_fixture("truncado.xml", xml),
        Err(ErrorCfdi::XmlInvalido(_))
    ));
}

#[test]
fn comprobante_sin_conceptos_devuelve_error() {
    let xml = br#"<c:Comprobante xmlns:c="http://www.sat.gob.mx/cfd/4"/>"#;

    assert_eq!(
        parsear_fixture("sin_conceptos.xml", xml),
        Err(ErrorCfdi::SinConceptos)
    );
}

#[test]
fn monto_decimal_invalido_devuelve_error() {
    let xml = br#"<c:Comprobante xmlns:c="http://www.sat.gob.mx/cfd/4"><c:Conceptos><c:Concepto Importe="no-es-decimal"/></c:Conceptos></c:Comprobante>"#;

    assert!(matches!(
        parsear_fixture("monto_invalido.xml", xml),
        Err(ErrorCfdi::MontoInvalido {
            campo: "Importe de concepto",
            ..
        })
    ));
}

fn crear_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    for (nombre, bytes) in entries {
        if nombre.ends_with('/') {
            writer
                .add_directory(*nombre, options)
                .expect("ZIP directory starts");
        } else {
            writer
                .start_file(*nombre, options)
                .expect("ZIP entry starts");
            writer.write_all(bytes).expect("ZIP entry writes");
        }
    }

    writer.finish().expect("ZIP finishes").into_inner()
}

fn escribir_archivo_temporal(bytes: &[u8]) -> NamedTempFile {
    let mut archivo = NamedTempFile::new().expect("temporary ZIP file is created");
    archivo
        .write_all(bytes)
        .expect("temporary ZIP file is written");
    archivo
}

#[test]
fn zip_lee_xml_de_subcarpetas_e_ignora_otros_archivos_y_macosx() {
    let xml = include_bytes!("fixtures/cfdi_33.xml");
    let archive = crear_zip(&[
        ("subcarpeta/", b""),
        ("subcarpeta/nombre-arbitrario.XML", xml),
        ("documentos/nota.txt", b"no XML"),
        ("__MACOSX/._oculto.xml", b"ignorar"),
    ]);
    let archivo = escribir_archivo_temporal(&archive);

    let entries = ZipXmlReader
        .leer_xmls(archivo.path())
        .expect("valid ZIP archive opens");

    assert_eq!(entries.len(), 1);
    let (name, bytes) = entries[0].as_ref().expect("XML entry reads");
    assert_eq!(name, "subcarpeta/nombre-arbitrario.XML");
    assert_eq!(bytes, xml);
}

#[test]
fn zip_corrupto_reporta_archivo_y_continua_con_las_demas_entradas() {
    const MARKER: &[u8] = b"UNIQUE_CORRUPT_XML_ENTRY";
    let mut archive = crear_zip(&[
        (
            "bien/otro-nombre.xml",
            include_bytes!("fixtures/cfdi_33.xml"),
        ),
        ("mal/corrupto.xml", MARKER),
    ]);
    let marker_position = archive
        .windows(MARKER.len())
        .position(|window| window == MARKER)
        .expect("marker exists in stored ZIP data");
    archive[marker_position] ^= 1;
    let archivo = escribir_archivo_temporal(&archive);

    let entries = ZipXmlReader
        .leer_xmls(archivo.path())
        .expect("valid ZIP archive opens");

    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries[0]
            .as_ref()
            .expect("valid XML entry remains readable")
            .0,
        "bien/otro-nombre.xml"
    );
    let error = entries[1]
        .as_ref()
        .expect_err("corrupt entry reports error");
    assert_eq!(error.archivo, "mal/corrupto.xml");
}

#[test]
fn zip_invalido_se_reporta_como_error_de_archivo() {
    let archivo = escribir_archivo_temporal(b"no es un ZIP");
    let error = ZipXmlReader
        .leer_xmls(archivo.path())
        .expect_err("invalid ZIP reports error");
    assert_eq!(error.archivo, archivo.path().display().to_string());
}
