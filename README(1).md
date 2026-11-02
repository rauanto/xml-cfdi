# CFDI ZIP Processor

Aplicación de escritorio (Rust + GPUI) que procesa un archivo ZIP con CFDI en XML (emitidos y recibidos), extrae los datos a nivel concepto y genera un Excel con 4 hojas y una vista previa en pantalla.

> Regla base: **solo extrae, no interpreta fiscalmente y no inventa datos**. Si un dato falta, la celda queda vacía.

## Stack

| Capa | Tecnología |
|------|-----------|
| UI | GPUI |
| ZIP | `zip` |
| XML con namespaces | `quick-xml` (`NsReader`) |
| Excel | `rust_xlsxwriter` |
| Decimales | `rust_decimal` (nunca `f64` para dinero) |
| Errores | `thiserror` |
| Tests | `cargo test` + fixtures XML en `tests/fixtures/` |

## Flujo

1. El usuario selecciona un `.zip`.
2. Se leen todos los `.xml` del ZIP sin importar su nombre ni subcarpeta.
3. Cada XML se parsea; si falla, se registra en **Errores** y se continúa.
4. Se genera una fila por concepto (CFDI con 3 conceptos = 3 filas).
5. Se muestra la vista previa (tabla con pestañas, igual que las hojas).
6. El usuario exporta el `.xlsx`.

## Namespaces

| Prefijo | URI | Uso |
|---------|-----|-----|
| `cfdi` | `http://www.sat.gob.mx/cfd/4` (y `/cfd/3`) | Comprobante, Emisor, Receptor, Conceptos, Impuestos |
| `tfd` | `http://www.sat.gob.mx/TimbreFiscalDigital` | `TimbreFiscalDigital` → `UUID` |

Se resuelve por **URI del namespace**, nunca por el texto del prefijo (un XML puede usar otro prefijo). Se soportan CFDI 3.3 y 4.0.

## Campos extraídos

**Comprobante (se repite en cada fila de concepto):**
UUID, Fecha, RFC Emisor, Nombre Emisor, RFC Receptor, Tipo de Comprobante (I/E/P/N/T), Subtotal, Descuento, IVA trasladado (002), ISR retenido (001), IVA retenido (002), Total, Uso CFDI, Método de Pago, Forma de Pago, Moneda, Tipo de Cambio.

**Concepto (una fila cada uno):**
ClaveProdServ, Descripción, Importe.

Los impuestos se toman del nodo `cfdi:Impuestos` a nivel comprobante (`TotalImpuestosTrasladados` / `TotalImpuestosRetenidos` y el desglose por código de impuesto).

## Hojas del Excel

1. **CFDI (nivel concepto)**: todos los registros, una fila por concepto.
2. **Resumen por producto**: agrupa `ClaveProdServ + Descripción` y suma el importe.
3. **Clasificación contable sugerida**: Ingreso / Gasto / Honorarios / Nómina / Inversión / Otros, según tipo de comprobante y palabras clave de la descripción. Es una **sugerencia**, no una interpretación fiscal.
4. **Errores**: archivo, ruta dentro del ZIP y motivo (XML inválido, sin `Comprobante`, sin UUID, etc.).

## Arquitectura (hexagonal)

```
src/
├── main.rs
├── domain/            # Entidades y reglas puras (sin I/O)
│   ├── cfdi.rs        # CfdiRow, ConceptoRow
│   ├── clasificacion.rs
│   └── error.rs
├── application/       # Casos de uso
│   ├── procesar_zip.rs
│   ├── resumen.rs
│   └── ports.rs       # traits: ZipReader, CfdiParser, ExcelExporter
├── infrastructure/    # Adaptadores
│   ├── zip_reader.rs
│   ├── xml_parser.rs  # quick-xml + namespaces
│   └── xlsx_exporter.rs
└── ui/                # GPUI (vistas, estado, tabla de vista previa)
tests/fixtures/        # XML de ejemplo (3.3, 4.0, con retenciones, pago, nómina, corrupto)
```

La UI nunca parsea ni escribe archivos directamente: llama a los casos de uso. El procesamiento corre fuera del hilo de UI y reporta progreso.

## Preguntas abiertas (resolver antes de implementar)

1. **Emitidas vs. Recibidas:** el XML no indica por sí solo cuál es "tuyo". ¿Se captura el RFC propio en la app para marcar cada CFDI como emitido/recibido? ¿O la columna se omite?
2. **"ffd":** se asume que se refiere a `tfd` (TimbreFiscalDigital). ¿Correcto?
3. **Impuestos por fila:** los impuestos son del comprobante completo. ¿Repetirlos en cada fila de concepto (propuesto) o dejarlos solo en la primera fila de cada CFDI para no duplicar sumas?
4. **CFDI tipo P (pagos) y N (nómina):** pueden no tener importes útiles en conceptos (los pagos traen importe 0). ¿Se extraen tal cual (propuesto)?
5. **Hoja 1** se llama "CFDI Emitidas" pero contendrá emitidas y recibidas. ¿Renombrar a "CFDI"?
6. **Clasificación:** ¿se usa un catálogo de palabras clave fijo (propuesto) o editable por el usuario?

## Ejecución

```bash
cargo run --release
cargo test
```

## Licencia

Por definir.
