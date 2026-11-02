//! Estado único de la aplicación y composición de sus componentes GPUI.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Local;
use gpui::{
    Context, FileDropEvent, FocusHandle, FontWeight, IntoElement, KeyDownEvent, PathPromptOptions,
    Render, ScrollHandle, UniformListScrollHandle, Window, canvas, div, prelude::*, px,
};
use rust_decimal::Decimal;

use crate::{
    application::{
        ports::ExcelExporter,
        procesar_zip::{ProcesarZip, ResultadoProceso},
    },
    infrastructure::{
        xlsx_exporter::XlsxExporter, xml_parser::XmlCfdiParser, zip_reader::ZipXmlReader,
    },
};

use super::{
    data_table::{CFDI_COLUMNS, CLASIFICACION_COLUMNS, DataTable, ERROR_COLUMNS, RESUMEN_COLUMNS},
    drop_zone::DropZone,
    metrics::Metrics,
    status_bar::StatusBar,
    tabs::Tabs,
    theme::{self, Palette},
    toolbar::Toolbar,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Vista {
    #[default]
    Cfdi,
    Resumen,
    Clasificacion,
    Errores,
}

impl Vista {
    pub fn index(self) -> usize {
        match self {
            Self::Cfdi => 0,
            Self::Resumen => 1,
            Self::Clasificacion => 2,
            Self::Errores => 3,
        }
    }

    fn desde_indice(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Cfdi),
            1 => Some(Self::Resumen),
            2 => Some(Self::Clasificacion),
            3 => Some(Self::Errores),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EstadoApp {
    #[default]
    Vacio,
    ZipCargado,
    Procesando,
    Resultados,
    Error,
}

#[derive(Clone, Debug)]
pub struct ResultadosVista {
    pub filas: Arc<Vec<crate::domain::cfdi::CfdiRow>>,
    pub resumen: Arc<Vec<crate::application::resumen::ResumenProducto>>,
    pub clasificaciones: Arc<Vec<crate::domain::clasificacion::FilaClasificada>>,
    pub errores: Arc<Vec<crate::domain::error::ErrorArchivo>>,
    pub total_xml_leidos: usize,
    pub total_cfdi_ok: usize,
}

impl From<ResultadoProceso> for ResultadosVista {
    fn from(resultado: ResultadoProceso) -> Self {
        Self {
            filas: Arc::new(resultado.filas),
            resumen: Arc::new(resultado.resumen),
            clasificaciones: Arc::new(resultado.clasificaciones),
            errores: Arc::new(resultado.errores),
            total_xml_leidos: resultado.total_xml_leidos,
            total_cfdi_ok: resultado.total_cfdi_ok,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub estado: EstadoApp,
    pub ruta_zip: Option<PathBuf>,
    pub rfc_propio: String,
    pub vista: Vista,
    pub resultados: Option<Arc<ResultadosVista>>,
    pub procesados: usize,
    pub total: usize,
    pub mensaje_error: Option<String>,
    pub ruta_exportada: Option<PathBuf>,
    pub exportando: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            estado: EstadoApp::Vacio,
            ruta_zip: None,
            rfc_propio: String::new(),
            vista: Vista::Cfdi,
            resultados: None,
            procesados: 0,
            total: 0,
            mensaje_error: None,
            ruta_exportada: None,
            exportando: false,
        }
    }
}

impl AppState {
    pub fn set_zip(&mut self, path: PathBuf) {
        self.ruta_zip = Some(path);
        self.estado = EstadoApp::ZipCargado;
        self.resultados = None;
        self.procesados = 0;
        self.total = 0;
        self.mensaje_error = None;
        self.ruta_exportada = None;
        self.exportando = false;
        self.vista = Vista::Cfdi;
    }

    pub fn start_processing(&mut self) -> bool {
        if self.ruta_zip.is_none()
            || !matches!(self.estado, EstadoApp::ZipCargado | EstadoApp::Resultados)
        {
            return false;
        }
        self.estado = EstadoApp::Procesando;
        self.resultados = None;
        self.procesados = 0;
        self.total = 0;
        self.mensaje_error = None;
        self.ruta_exportada = None;
        true
    }

    pub fn set_progress(&mut self, processed: usize, total: usize) {
        self.procesados = processed.min(total);
        self.total = total;
    }

    pub fn finish_processing(&mut self, resultado: ResultadoProceso) {
        if resultado.total_xml_leidos == 0 {
            let mensaje = if resultado.errores.is_empty() {
                "El ZIP no contiene archivos .xml.".to_owned()
            } else {
                "No se pudo abrir el ZIP. Comprueba que sea un archivo ZIP válido.".to_owned()
            };
            self.set_error(mensaje);
            return;
        }
        self.resultados = Some(Arc::new(resultado.into()));
        self.estado = EstadoApp::Resultados;
        self.procesados = self.total;
        self.mensaje_error = None;
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.estado = EstadoApp::Error;
        self.resultados = None;
        self.mensaje_error = Some(message.into());
        self.exportando = false;
    }

    pub fn metric_text(&self) -> String {
        let Some(resultados) = &self.resultados else {
            return String::new();
        };
        let conceptos = resultados.filas.len();
        format!(
            "{} XML · {} CFDI · {} errores · {} conceptos",
            format_integer(resultados.total_xml_leidos),
            format_integer(resultados.total_cfdi_ok),
            format_integer(resultados.errores.len()),
            format_integer(conceptos)
        )
    }

    fn status_text(&self) -> String {
        if self.exportando {
            return "Exportando Excel…".to_owned();
        }
        if let Some(message) = &self.mensaje_error {
            return message.clone();
        }
        if self.estado == EstadoApp::Procesando {
            return format!(
                "Procesando {} de {}",
                format_integer(self.procesados),
                format_integer(self.total)
            );
        }
        if let Some(path) = &self.ruta_exportada {
            return format!("Exportado en {}", path.display());
        }
        "Listo".to_owned()
    }
}

pub struct AppView {
    pub state: AppState,
    root_focus: FocusHandle,
    rfc_focus: FocusHandle,
    scroll_handles: [UniformListScrollHandle; 4],
    horizontal_scroll_handles: [ScrollHandle; 4],
    pending_drop: Option<PathBuf>,
}

impl AppView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            state: AppState::default(),
            root_focus: cx.focus_handle(),
            rfc_focus: cx.focus_handle(),
            scroll_handles: std::array::from_fn(|_| UniformListScrollHandle::new()),
            horizontal_scroll_handles: std::array::from_fn(|_| ScrollHandle::new()),
            pending_drop: None,
        }
    }

    fn open_zip_dialog(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Seleccionar archivo ZIP".into()),
        });
        cx.spawn(async move |this, cx| match receiver.await {
            Ok(Ok(Some(paths))) => {
                if let Some(path) = paths.into_iter().next() {
                    this.update(cx, |this, cx| this.load_zip(path, cx)).ok();
                }
            }
            Ok(Err(error)) => {
                eprintln!("No se pudo abrir el selector de archivos: {error}");
                this.update(cx, |this, cx| {
                    this.state
                        .set_error("No se pudo abrir el selector de archivos.");
                    cx.notify();
                })
                .ok();
            }
            Ok(Ok(None)) | Err(_) => {}
        })
        .detach();
    }

    fn load_zip(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.state.set_zip(path.clone());
        if !es_zip(&path) {
            self.state
                .set_error("Selecciona un archivo con extensión .zip.");
        }
        cx.notify();
    }

    fn process_zip(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.state.ruta_zip.clone() else {
            return;
        };
        if !self.state.start_processing() {
            return;
        }
        cx.notify();

        let rfc = (!self.state.rfc_propio.trim().is_empty())
            .then(|| self.state.rfc_propio.trim().to_owned());
        let (sender, receiver) = async_channel::unbounded();
        cx.background_executor()
            .spawn(async move {
                let processor = ProcesarZip::new(ZipXmlReader, XmlCfdiParser);
                let mut last_update = Instant::now();
                let result = processor.ejecutar_con_progreso(path, rfc, |processed, total| {
                    if processed == 0
                        || processed == total
                        || last_update.elapsed() >= Duration::from_millis(50)
                    {
                        last_update = Instant::now();
                        let _ = sender.send_blocking(WorkerMessage::Progress(processed, total));
                    }
                });
                let _ = sender.send_blocking(WorkerMessage::Complete(result));
            })
            .detach();

        cx.spawn(async move |this, cx| {
            while let Ok(message) = receiver.recv().await {
                let complete = matches!(message, WorkerMessage::Complete(_));
                if this
                    .update(cx, |this, cx| this.handle_worker_message(message, cx))
                    .is_err()
                    || complete
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn handle_worker_message(&mut self, message: WorkerMessage, cx: &mut Context<Self>) {
        match message {
            WorkerMessage::Progress(processed, total) => {
                self.state.set_progress(processed, total);
            }
            WorkerMessage::Complete(result) => {
                self.state.finish_processing(result);
            }
        }
        cx.notify();
    }

    fn choose_export_path(&mut self, cx: &mut Context<Self>) {
        if self.state.resultados.is_none() || self.state.exportando {
            return;
        }
        let directory = self
            .state
            .ruta_zip
            .as_deref()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new("."));
        let suggested = format!("cfdi_{}.xlsx", Local::now().format("%Y%m%d_%H%M"));
        let receiver = cx.prompt_for_new_path(directory, Some(&suggested));

        cx.spawn(async move |this, cx| match receiver.await {
            Ok(Ok(Some(path))) => {
                this.update(cx, |this, cx| this.export_to(path, cx)).ok();
            }
            Ok(Err(error)) => {
                eprintln!("No se pudo abrir el selector para guardar: {error}");
                this.update(cx, |this, cx| {
                    this.state
                        .set_error("No se pudo abrir el selector para guardar.");
                    cx.notify();
                })
                .ok();
            }
            Ok(Ok(None)) | Err(_) => {}
        })
        .detach();
    }

    fn export_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(resultados) = self.state.resultados.clone() else {
            return;
        };
        let path = ensure_extension(path, "xlsx");
        self.state.exportando = true;
        self.state.mensaje_error = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let path_for_export = path.clone();
            let result = cx
                .background_executor()
                .spawn(async move {
                    XlsxExporter.exportar(
                        resultados.filas.as_slice(),
                        resultados.resumen.as_slice(),
                        resultados.clasificaciones.as_slice(),
                        resultados.errores.as_slice(),
                        &path_for_export,
                    )
                })
                .await;

            this.update(cx, |this, cx| {
                this.state.exportando = false;
                match result {
                    Ok(()) => {
                        this.state.ruta_exportada = Some(path);
                        this.state.mensaje_error = None;
                    }
                    Err(error) => {
                        eprintln!("No se pudo exportar el Excel: {error}");
                        this.state.mensaje_error =
                            Some("No se pudo exportar el archivo Excel.".to_owned());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn on_shortcut(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if !(modifiers.control || modifiers.platform) {
            return;
        }
        let handled = match event.keystroke.key.to_ascii_lowercase().as_str() {
            "o" => {
                self.open_zip_dialog(cx);
                true
            }
            "e" if self.state.resultados.is_some() => {
                self.choose_export_path(cx);
                true
            }
            "1" => {
                self.select_tab(Vista::Cfdi, cx);
                true
            }
            "2" => {
                self.select_tab(Vista::Resumen, cx);
                true
            }
            "3" => {
                self.select_tab(Vista::Clasificacion, cx);
                true
            }
            "4" => {
                self.select_tab(Vista::Errores, cx);
                true
            }
            _ => false,
        };
        if handled {
            window.focus(&self.root_focus);
        }
    }

    fn select_tab(&mut self, tab: Vista, cx: &mut Context<Self>) {
        self.state.vista = tab;
        cx.notify();
    }

    fn update_rfc_from_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        let key = event.keystroke.key.as_str();
        if key == "backspace" {
            self.state.rfc_propio.pop();
        } else if key == "delete" {
            self.state.rfc_propio.clear();
        } else if let Some(character) = event.keystroke.key_char.as_deref() {
            self.state
                .rfc_propio
                .push_str(&normalizar_caja_rfc(character));
        }
        cx.notify();
    }

    fn render_content(&mut self, palette: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        match self.state.estado {
            EstadoApp::Vacio => DropZone::new(
                palette,
                "Seleccionar archivo",
                cx.listener(|this, _, _, cx| this.open_zip_dialog(cx)),
            )
            .into_any_element(),
            EstadoApp::ZipCargado => div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(theme::SPACING))
                .child(
                    div()
                        .text_size(px(theme::FONT_LABEL))
                        .text_color(palette.text)
                        .child(
                            self.state
                                .ruta_zip
                                .as_deref()
                                .and_then(Path::file_name)
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        ),
                )
                .child(
                    DropZone::new(
                        palette,
                        "Seleccionar otro ZIP",
                        cx.listener(|this, _, _, cx| this.open_zip_dialog(cx)),
                    )
                    .into_any_element(),
                )
                .into_any_element(),
            EstadoApp::Error => div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(theme::SPACING))
                .text_size(px(theme::FONT_LABEL))
                .text_color(palette.error)
                .child(self.state.mensaje_error.clone().unwrap_or_default())
                .child(DropZone::new(
                    palette,
                    "Seleccionar otro ZIP",
                    cx.listener(|this, _, _, cx| this.open_zip_dialog(cx)),
                ))
                .into_any_element(),
            EstadoApp::Procesando | EstadoApp::Resultados => {
                self.render_preview(palette).into_any_element()
            }
        }
    }

    fn render_preview(&mut self, palette: Palette) -> impl IntoElement {
        let Some(resultados) = &self.state.resultados else {
            return div().flex_1().into_any_element();
        };
        match self.state.vista {
            Vista::Cfdi => DataTable::new(
                palette,
                &CFDI_COLUMNS,
                Arc::clone(&resultados.filas),
                cfdi_cell,
                self.scroll_handles[0].clone(),
                self.horizontal_scroll_handles[0].clone(),
                "cfdi-rows",
            )
            .into_any_element(),
            Vista::Resumen => DataTable::new(
                palette,
                &RESUMEN_COLUMNS,
                Arc::clone(&resultados.resumen),
                resumen_cell,
                self.scroll_handles[1].clone(),
                self.horizontal_scroll_handles[1].clone(),
                "summary-rows",
            )
            .into_any_element(),
            Vista::Clasificacion => DataTable::new(
                palette,
                &CLASIFICACION_COLUMNS,
                Arc::clone(&resultados.clasificaciones),
                clasificacion_cell,
                self.scroll_handles[2].clone(),
                self.horizontal_scroll_handles[2].clone(),
                "classification-rows",
            )
            .into_any_element(),
            Vista::Errores => DataTable::new(
                palette,
                &ERROR_COLUMNS,
                Arc::clone(&resultados.errores),
                error_cell,
                self.scroll_handles[3].clone(),
                self.horizontal_scroll_handles[3].clone(),
                "error-rows",
            )
            .into_any_element(),
        }
    }
}

enum WorkerMessage {
    Progress(usize, usize),
    Complete(ResultadoProceso),
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = theme::for_appearance(window.appearance());
        let status = self.state.status_text();
        let exported = self.state.ruta_exportada.clone();
        let progress = self.state.estado == EstadoApp::Procesando;
        let progress_fraction = if self.state.total == 0 {
            0.0
        } else {
            self.state.procesados as f32 / self.state.total as f32
        };
        let progress_label = format!(
            "Procesando {} de {}",
            format_integer(self.state.procesados),
            format_integer(self.state.total)
        );
        let error_count = self
            .state
            .resultados
            .as_ref()
            .map_or(0, |result| result.errores.len());
        let is_exportable = self.state.resultados.is_some() && !self.state.exportando;

        let drop_view = cx.entity().downgrade();
        let drop_listener = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                let drop_view = drop_view.clone();
                window.on_mouse_event(move |event: &FileDropEvent, _, _, app| {
                    drop_view
                        .update(app, |this, cx| match event {
                            FileDropEvent::Entered { paths, .. } => {
                                this.pending_drop = paths.paths().first().cloned();
                            }
                            FileDropEvent::Submit { .. } => {
                                if let Some(path) = this.pending_drop.take() {
                                    this.load_zip(path, cx);
                                }
                            }
                            FileDropEvent::Exited => this.pending_drop = None,
                            FileDropEvent::Pending { .. } => {}
                        })
                        .ok();
                });
            },
        )
        .absolute()
        .size_full();

        let root_focus = self.root_focus.clone();
        let rfc_focus = self.rfc_focus.clone();
        let state = self.state.clone();
        let accent = palette.accent;
        let progress_element = if progress {
            div()
                .w_full()
                .h(px(theme::PROGRESS_HEIGHT))
                .bg(palette.border)
                .child(
                    div()
                        .h_full()
                        .w(gpui::relative(progress_fraction))
                        .bg(accent),
                )
                .into_any_element()
        } else {
            div()
                .w_full()
                .h(px(theme::PROGRESS_HEIGHT))
                .into_any_element()
        };

        div()
            .id("app-root")
            .track_focus(&root_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let command =
                    event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
                if command {
                    this.on_shortcut(event, window, cx);
                }
            }))
            .on_click(cx.listener(|this, _, window, _| {
                if !this.rfc_focus.is_focused(window) {
                    window.focus(&this.root_focus);
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            .font(gpui::Font {
                features: theme::tabular_numbers(),
                ..gpui::font("Inter")
            })
            .text_size(px(theme::FONT_BODY))
            .text_color(palette.text)
            .bg(palette.background)
            .child(Toolbar::new(
                palette,
                is_exportable,
                cx.listener(|this, _, _, cx| this.choose_export_path(cx)),
            ))
            .child(self.render_options(palette, rfc_focus, cx))
            .child(progress_element)
            .when(progress, |element| {
                element.child(
                    div()
                        .h(px(theme::METRICS_HEIGHT))
                        .flex()
                        .items_center()
                        .px(px(theme::CONTENT_PADDING))
                        .text_size(px(theme::FONT_SMALL))
                        .text_color(palette.secondary)
                        .child(progress_label),
                )
            })
            .when(state.resultados.is_some(), |element| {
                element
                    .child(Metrics::new(palette, state.clone()))
                    .child(Tabs::new(
                        palette,
                        state.vista,
                        error_count,
                        cx.listener(|this, index: &usize, _, cx| {
                            if let Some(tab) = Vista::desde_indice(*index) {
                                this.select_tab(tab, cx);
                            }
                        }),
                    ))
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .px(px(theme::CONTENT_PADDING))
                    .when(state.resultados.is_some(), |element| {
                        element.items_start().justify_start()
                    })
                    .child(self.render_content(palette, cx)),
            )
            .child(StatusBar::new(
                palette,
                state.ruta_zip.clone(),
                status,
                exported.clone(),
                cx.listener(move |this, _, _, cx| {
                    if let Some(path) = this.state.ruta_exportada.as_deref()
                        && let Some(directory) = path.parent()
                    {
                        cx.reveal_path(directory);
                    }
                }),
            ))
            .child(drop_listener)
    }
}

impl AppView {
    fn render_options(
        &mut self,
        palette: Palette,
        rfc_focus: FocusHandle,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let enabled = self.state.ruta_zip.is_some()
            && matches!(
                self.state.estado,
                EstadoApp::ZipCargado | EstadoApp::Resultados
            );
        let validation = validar_rfc(&self.state.rfc_propio)
            .unwrap_or("RFC opcional; se normaliza a mayúsculas");
        div()
            .w_full()
            .h(px(theme::OPTIONS_HEIGHT))
            .flex()
            .items_center()
            .gap(px(theme::SPACING))
            .px(px(theme::CONTENT_PADDING))
            .border_b_1()
            .border_color(palette.border)
            .bg(palette.surface)
            .child(
                div()
                    .text_size(px(theme::FONT_BODY))
                    .text_color(palette.secondary)
                    .child("RFC propio (opcional)"),
            )
            .child(
                div()
                    .id("rfc-input")
                    .w(px(theme::RFC_INPUT_WIDTH))
                    .h(px(theme::CONTROL_HEIGHT))
                    .flex()
                    .items_center()
                    .px(px(theme::SPACING))
                    .rounded(px(theme::CORNER_RADIUS))
                    .border_1()
                    .border_color(palette.border)
                    .bg(palette.background)
                    .track_focus(&rfc_focus)
                    .on_click(cx.listener(move |this, _, window, _| {
                        window.focus(&this.rfc_focus);
                    }))
                    .on_key_down(cx.listener(|this, event, _, cx| {
                        this.update_rfc_from_key(event, cx);
                    }))
                    .text_size(px(theme::FONT_BODY))
                    .text_color(palette.text)
                    .child(if self.state.rfc_propio.is_empty() {
                        "RFC".to_owned()
                    } else {
                        self.state.rfc_propio.clone()
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(px(theme::FONT_SMALL))
                    .text_color(palette.secondary)
                    .child(validation),
            )
            .child(
                div()
                    .id("process-button")
                    .h(px(theme::CONTROL_HEIGHT))
                    .px(px(theme::SPACING))
                    .flex()
                    .items_center()
                    .rounded(px(theme::CORNER_RADIUS))
                    .bg(if enabled {
                        palette.accent
                    } else {
                        palette.border
                    })
                    .text_size(px(theme::FONT_BODY))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(palette.accent_text)
                    .when(enabled, |button| button.cursor_pointer())
                    .on_click(cx.listener(|this, _, _, cx| this.process_zip(cx)))
                    .child("Procesar"),
            )
    }
}

pub fn validar_rfc(value: &str) -> Option<&'static str> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let length = value.chars().count();
    if !(12..=13).contains(&length) {
        return Some("El RFC debe tener 12 o 13 caracteres");
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, 'Ñ' | '&'))
    {
        return Some("Usa solo letras, números, Ñ o &");
    }
    None
}

fn normalizar_caja_rfc(value: &str) -> String {
    value.chars().flat_map(char::to_uppercase).collect()
}

pub fn format_decimal(value: Decimal, decimals: usize) -> String {
    let formatted = format!("{value:.decimals$}");
    let (sign, number) = formatted
        .strip_prefix('-')
        .map_or(("", formatted.as_str()), |number| ("-", number));
    let (integer, fraction) = number.split_once('.').unwrap_or((number, ""));
    let mut reversed = String::with_capacity(formatted.len() + integer.len() / 3);
    for (index, character) in integer.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            reversed.push(',');
        }
        reversed.push(character);
    }
    let integer = reversed.chars().rev().collect::<String>();
    let mut grouped = format!("{sign}{integer}");
    if decimals > 0 {
        grouped.push('.');
        grouped.push_str(fraction);
    }
    grouped
}

fn format_integer(value: usize) -> String {
    let mut result = String::new();
    for (index, character) in value.to_string().chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            result.push(',');
        }
        result.push(character);
    }
    result.chars().rev().collect()
}

fn es_zip(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

fn ensure_extension(mut path: PathBuf, extension: &str) -> PathBuf {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case(extension))
    {
        path.set_extension(extension);
    }
    path
}

fn cfdi_cell(row: &crate::domain::cfdi::CfdiRow, column: usize) -> String {
    let field: Option<String> = match column {
        0 => row.uuid.clone(),
        1 => row.fecha.clone(),
        2 => row.rfc_emisor.clone(),
        3 => row.nombre_emisor.clone(),
        4 => row.rfc_receptor.clone(),
        5 => row.tipo_comprobante.clone(),
        6 => decimal_cell(row.subtotal, 2),
        7 => decimal_cell(row.descuento, 2),
        8 => decimal_cell(row.iva_trasladado_002, 2),
        9 => decimal_cell(row.isr_retenido_001, 2),
        10 => decimal_cell(row.iva_retenido_002, 2),
        11 => decimal_cell(row.total, 2),
        12 => row.uso_cfdi.clone(),
        13 => row.metodo_pago.clone(),
        14 => row.forma_pago.clone(),
        15 => row.moneda.clone(),
        16 => decimal_cell(row.tipo_cambio, 6),
        17 => row.clave_prod_serv.clone(),
        18 => row.descripcion.clone(),
        19 => decimal_cell(row.importe_concepto, 2),
        20 => Some(row.archivo_origen.clone()),
        _ => None,
    };
    field.unwrap_or_default()
}

fn resumen_cell(row: &crate::application::resumen::ResumenProducto, column: usize) -> String {
    match column {
        0 => row.clave_prod_serv.clone().unwrap_or_default(),
        1 => row.descripcion.clone().unwrap_or_default(),
        2 => format_decimal(row.importe_total, 2),
        3 => format_integer(row.num_conceptos),
        _ => String::new(),
    }
}

fn clasificacion_cell(
    row: &crate::domain::clasificacion::FilaClasificada,
    column: usize,
) -> String {
    match column {
        0 => row.fila.archivo_origen.clone(),
        1 => row.fila.uuid.clone().unwrap_or_default(),
        2 => row.fila.tipo_comprobante.clone().unwrap_or_default(),
        3 => row.fila.clave_prod_serv.clone().unwrap_or_default(),
        4 => row.fila.descripcion.clone().unwrap_or_default(),
        5 => row.clasificacion.categoria.nombre().to_owned(),
        6 => row.clasificacion.criterio.clone(),
        _ => String::new(),
    }
}

fn error_cell(row: &crate::domain::error::ErrorArchivo, column: usize) -> String {
    match column {
        0 => row.archivo.clone(),
        1 => row.motivo.clone(),
        _ => String::new(),
    }
}

fn decimal_cell(value: Option<Decimal>, precision: usize) -> Option<String> {
    value.map(|value| format_decimal(value, precision))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validar_rfc_acepta_12_y_13_caracteres_y_avisa_sin_bloquear() {
        assert_eq!(validar_rfc("ASF180425991"), None);
        assert_eq!(validar_rfc("CAR060613CDA"), None);
        assert_eq!(validar_rfc(""), None);
        assert!(validar_rfc("CORTO").is_some());
        assert!(validar_rfc("ASF18042599!").is_some());
        assert_eq!(normalizar_caja_rfc("asf180425991"), "ASF180425991");
    }

    #[test]
    fn format_decimal_usa_miles_y_dos_decimales_sin_float() {
        assert_eq!(format_decimal("1248.5".parse().unwrap(), 2), "1,248.50");
        assert_eq!(
            format_decimal("-1000000.00".parse().unwrap(), 2),
            "-1,000,000.00"
        );
        assert_eq!(format_decimal("0".parse().unwrap(), 2), "0.00");
    }

    #[test]
    fn app_state_recoge_transiciones_de_carga_a_resultados_y_error() {
        let mut state = AppState::default();
        assert_eq!(state.estado, EstadoApp::Vacio);
        state.set_zip(PathBuf::from("archivo.zip"));
        assert_eq!(state.estado, EstadoApp::ZipCargado);
        assert!(state.start_processing());
        assert_eq!(state.estado, EstadoApp::Procesando);
        state.finish_processing(ResultadoProceso {
            filas: Vec::new(),
            resumen: Vec::new(),
            clasificaciones: Vec::new(),
            errores: Vec::new(),
            total_xml_leidos: 1,
            total_cfdi_ok: 1,
        });
        assert_eq!(state.estado, EstadoApp::Resultados);
        assert_eq!(
            state.metric_text(),
            "1 XML · 1 CFDI · 0 errores · 0 conceptos"
        );
        assert!(state.start_processing());
        state.set_error("falló");
        assert_eq!(state.estado, EstadoApp::Error);
        state.set_zip(PathBuf::from("otro.zip"));
        assert_eq!(state.estado, EstadoApp::ZipCargado);
        assert!(state.resultados.is_none());
    }
}
