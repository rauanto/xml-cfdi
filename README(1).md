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
| Tests | `cargo test` + fixtures XML en `tests/fixtures/` + lectura XLSX con `calamine` |

## Flujo

1. El usuario selecciona un `.zip`.
2. Se leen todos los `.xml` del ZIP sin importar su nombre ni subcarpeta.
3. Cada XML se parsea; si falla, se registra en **Errores** y se continúa.
4. Se genera una fila por concepto (CFDI con 3 conceptos = 3 filas).
5. Se resumen importes por `ClaveProdServ + Descripción`; conceptos sin importe cuentan, pero no suman.
6. Se genera una clasificación contable sugerida con el criterio de cada fila.
7. El usuario exporta el `.xlsx`.

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

Los impuestos se toman del nodo `cfdi:Impuestos` hijo directo de `Comprobante`, por código de impuesto, y se repiten en cada fila de concepto. Si falta `TimbreFiscalDigital`, UUID queda vacío. Los tipos P y N se extraen tal como aparecen, sin tratamiento especial.

## Clasificación sugerida

La clasificación no es interpretación fiscal. Se evalúa en este orden y se conserva el criterio aplicado:

1. Tipo N → Nómina.
2. Descripción contiene una palabra clave de honorarios → Honorarios.
3. Descripción contiene una palabra clave de inversión → Inversión.
4. Tipo I con RFC propio: emisor coincide → Ingreso; receptor coincide → Gasto.
5. Otros; el criterio explica por qué no aplicó una regla.

La comparación de palabras clave ignora mayúsculas y acentos. RFC propio es opcional; sin RFC no se distingue Ingreso de Gasto. La lista editable está en `src/domain/clasificacion.rs`.

## Hojas del Excel

1. **CFDI (nivel concepto)**: todos los registros, una fila por concepto.
2. **Resumen por producto**: agrupa `ClaveProdServ + Descripción` y suma el importe.
3. **Clasificación sugerida**: campos clave del concepto, categoría y criterio. Es una **sugerencia**, no una interpretación fiscal.
4. **Errores**: archivo, ruta dentro del ZIP y motivo (XML inválido, sin `Comprobante`, sin UUID, etc.).

Los importes se calculan con `rust_decimal`; el XLSX los guarda como celdas numéricas con formato de dos decimales (seis para tipo de cambio). Los datos ausentes quedan en celdas vacías.

## Arquitectura (hexagonal)

```
src/
├── main.rs
├── lib.rs
├── domain/            # Entidades y reglas puras (sin I/O)
│   ├── cfdi.rs        # CfdiRow (una fila por concepto)
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
tests/
├── parser.rs
├── use_cases.rs
└── fixtures/          # XML 3.3/4.0, IVA tasa 0, tipos, corruptos y namespaces
```

La UI nunca parsea ni escribe archivos directamente: llama a los casos de uso. El procesamiento corre fuera del hilo de UI y reporta progreso.

## Interfaz de escritorio

La ventana abre a 1200 × 760 px y admite un mínimo de 900 × 560 px. El tema claro es predeterminado; GPUI sigue la apariencia clara u oscura del sistema. Inter permite cifras tabulares cuando está instalado; GPUI usa sus fuentes de respaldo si no está disponible.

Selecciona un ZIP con el área de carga o arrástralo a la ventana. Añade un RFC propio opcional y pulsa **Procesar**. El progreso se actualiza sin bloquear la ventana. La vista previa muestra solo las filas visibles mediante una lista virtualizada; las columnas mantienen su encabezado al desplazarse horizontalmente. Para recorrer columnas usa `Mayús + rueda` o los controles **Anterior/Siguiente** bajo la tabla.

Atajos:

- `Ctrl/Cmd+O`: seleccionar un ZIP.
- `Ctrl/Cmd+E`: exportar el Excel.
- `Ctrl/Cmd+1` a `Ctrl/Cmd+4`: cambiar de pestaña.

GPUI 0.2.2 no expone filtros de extensión en `PathPromptOptions`. El selector nativo permite elegir archivos y la aplicación valida `.zip` después de la selección. Al guardar, la aplicación añade `.xlsx` si falta. Tras exportar, **Abrir carpeta** revela el archivo en el explorador del sistema.

Para generar un ZIP de carga con 5,000 CFDI sintéticos basados en el fixture:

```bash
python3 tools/generate_load_test_zip.py /tmp/cfdi_5000.zip --count 5000
cargo run
```

Abre `/tmp/cfdi_5000.zip`, pulsa **Procesar** y verifica que la ventana siga respondiendo mientras avanza la barra de progreso. La tabla virtualiza las filas del resultado.

La prueba automatizada `cargo test procesar_zip_procesa_5000_xml_sin_perder_filas` verifica que el caso de uso procese 5,000 XML y conserve todas sus filas.

## Preguntas abiertas

1. **Emitidas vs. Recibidas:** no hay columna por ahora. RFC propio opcional solo guía sugerencias de Ingreso/Gasto.
2. **Clasificación:** las palabras clave son fijas en código por ahora; se puede decidir después si el catálogo debe ser editable.

## Ejecución

```bash
cargo run --release
cargo test
```

## Licencia

Por definir.
