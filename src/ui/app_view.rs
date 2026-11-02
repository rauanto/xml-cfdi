//! Estado único de la aplicación y composición de sus componentes GPUI.
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Local;
use gpui::{
    Context, Entity, FileDropEvent, FocusHandle, Focusable, IntoElement, KeyDownEvent,
    PathPromptOptions, Render, ScrollHandle, Subscription, UniformListScrollHandle, Window, canvas,
    div, prelude::*, px,
};
use gpuikit::{
    elements::{
        badge::badge,
        button::button,
        progress::progress as progress_bar,
        tabs::{TabChanged, Tabs as KitTabs},
        text_field::text_field,
        toast::ToastExt,
        typography::{small, text},
    },
    input::{InputState, InputStateEvent},
    layout::{h_stack, v_stack},
    theme::{ActiveTheme, ControlSize, Themeable},
    traits::control_sized::ControlSized,
};
use rust_decimal::Decimal;

use crate::{
    application::{
        config_service::ConfigService,
        ports::ExcelExporter,
        procesar_zip::{ProcesarZip, ResultadoProceso},
    },
    domain::config::AppConfig,
    infrastructure::{
        config_toml::TomlConfigRepository, xlsx_exporter::XlsxExporter, xml_parser::XmlCfdiParser,
        zip_reader::ZipXmlReader,
    },
};

use super::{
    data_table::{
        CFDI_COLUMNS, CLASIFICACION_COLUMNS, DataTable, ERROR_COLUMNS, Fila, RESUMEN_COLUMNS,
        construir_filas,
    },
    drop_zone::DropZone,
    metrics::Metrics,
    status_bar::StatusBar,
    tabs, theme,
    toolbar::Toolbar,
};

#[derive(Clone, Debug)]
pub struct TablasPreparadas {
    pub cfdi: Arc<Vec<Fila>>,
    pub resumen: Arc<Vec<Fila>>,
    pub clasificaciones: Arc<Vec<Fila>>,
    pub errores: Arc<Vec<Fila>>,
}

impl TablasPreparadas {
    pub fn desde(r: &ResultadoProceso) -> Self {
        Self {
            cfdi: construir_filas(r.filas.as_slice(), CFDI_COLUMNS.len(), cfdi_cell),
            resumen: construir_filas(r.resumen.as_slice(), RESUMEN_COLUMNS.len(), resumen_cell),
            clasificaciones: construir_filas(
                r.clasificaciones.as_slice(),
                CLASIFICACION_COLUMNS.len(),
                clasificacion_cell,
            ),
            errores: construir_filas(r.errores.as_slice(), ERROR_COLUMNS.len(), error_cell),
        }
    }
}

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

    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Cfdi => "cfdi",
            Self::Resumen => "resumen",
            Self::Clasificacion => "clasificacion",
            Self::Errores => "errores",
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SettingsSection {
    #[default]
    General,
    Extraccion,
    Clasificacion,
    Excel,
    Tabla,
    Apariencia,
    Atajos,
    Avanzado,
}

#[derive(Clone, Debug)]
pub struct ResultadosVista {
    pub filas: Arc<Vec<crate::domain::cfdi::CfdiRow>>,
    pub resumen: Arc<Vec<crate::application::resumen::ResumenProducto>>,
    pub clasificaciones: Arc<Vec<crate::domain::clasificacion::FilaClasificada>>,
    pub errores: Arc<Vec<crate::domain::error::ErrorArchivo>>,
    pub tablas: TablasPreparadas,
    pub total_xml_leidos: usize,
    pub total_cfdi_ok: usize,
}

impl ResultadosVista {
    fn new(resultado: ResultadoProceso, tablas: TablasPreparadas) -> Self {
        Self {
            filas: Arc::new(resultado.filas),
            resumen: Arc::new(resultado.resumen),
            clasificaciones: Arc::new(resultado.clasificaciones),
            errores: Arc::new(resultado.errores),
            tablas,
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
    pub config: AppConfig,
    pub vista: Vista,
    pub resultados: Option<Arc<ResultadosVista>>,
    pub procesados: usize,
    pub total: usize,
    pub mensaje_error: Option<String>,
    pub ruta_exportada: Option<PathBuf>,
    pub exportando: bool,
    pub settings_open: bool,
    pub settings_section: SettingsSection,
}

impl Default for AppState {
    fn default() -> Self {
        let config = AppConfig::default();
        Self {
            estado: EstadoApp::Vacio,
            ruta_zip: None,
            rfc_propio: config.general.rfc_propio.clone().unwrap_or_default(),
            config,
            vista: Vista::Cfdi,
            resultados: None,
            procesados: 0,
            total: 0,
            mensaje_error: None,
            ruta_exportada: None,
            exportando: false,
            settings_open: false,
            settings_section: SettingsSection::General,
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

    pub fn sync_rfc_from_config(&mut self) {
        self.rfc_propio = self.config.general.rfc_propio.clone().unwrap_or_default();
    }

    pub fn finish_processing(&mut self, resultado: ResultadoProceso, tablas: TablasPreparadas) {
        if resultado.total_xml_leidos == 0 {
            let mensaje = if resultado.errores.is_empty() {
                "El ZIP no contiene archivos .xml.".to_owned()
            } else {
                "No se pudo abrir el ZIP. Comprueba que sea un archivo ZIP válido.".to_owned()
            };
            self.set_error(mensaje);
            return;
        }
        self.resultados = Some(Arc::new(ResultadosVista::new(resultado, tablas)));
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
    rfc_input: Entity<InputState>,
    excel_format_input: Entity<InputState>,
    tabs: Entity<KitTabs>,
    config_repository: TomlConfigRepository,
    table_scroll_handles: [UniformListScrollHandle; 4],
    table_h_scroll_handles: [ScrollHandle; 4],
    _rfc_subscription: Subscription,
    _excel_format_subscription: Subscription,
    _shortcut_subscriptions: [Subscription; 5],
    tab_subscription: Subscription,
    pending_drop: Option<PathBuf>,
    pending_toast: Option<(String, bool)>,
    _appearance_subscription: Option<Subscription>,
}

impl AppView {
    pub fn new(cx: &mut Context<Self>, window: &mut Window) -> Self {
        let config_repository = TomlConfigRepository::new();
        let mut state = AppState::default();
        let loaded = ConfigService::new(config_repository.clone()).cargar_o_default();
        state.config = loaded.clone();
        state.sync_rfc_from_config();

        let rfc_input = cx.new(InputState::new_singleline);
        rfc_input.update(cx, |input, cx| {
            input.set_content_silent(state.rfc_propio.clone(), cx);
        });

        let excel_format_input = cx.new(InputState::new_singleline);
        excel_format_input.update(cx, |input, cx| {
            input.set_content_silent(state.config.excel.formato_nombre_archivo.clone(), cx);
        });

        let shortcut_inputs: [Entity<InputState>; 5] = std::array::from_fn(|index| {
            let input = cx.new(InputState::new_singleline);
            let value = match index {
                0 => state.config.atajos.abrir_zip.clone(),
                1 => state.config.atajos.exportar.clone(),
                2 => state.config.atajos.abrir_configuracion.clone(),
                3 => state.config.atajos.pestaña_1.clone(),
                4 => state.config.atajos.pestaña_2.clone(),
                _ => String::new(),
            };
            input.update(cx, |input, cx| input.set_content_silent(value, cx));
            input
        });

        let tabs = tabs::create(cx, None, Vista::Cfdi);
        let appearance_subscription = cx.observe_window_appearance(window, |_, window, cx| {
            theme::follow_system_appearance(window.appearance(), cx);
            cx.notify();
        });
        let rfc_subscription =
            cx.subscribe(&rfc_input, |this, input, event: &InputStateEvent, cx| {
                if matches!(event, InputStateEvent::TextChanged) {
                    let value = normalizar_caja_rfc(input.read(cx).content());
                    if value != input.read(cx).content() {
                        input.update(cx, |input, cx| input.set_content_silent(value.clone(), cx));
                    }
                    this.state.rfc_propio = value.clone();
                    this.state.config.general.rfc_propio = if value.trim().is_empty() {
                        None
                    } else {
                        Some(value)
                    };
                    cx.notify();
                }
            });
        let excel_format_subscription = cx.subscribe(
            &excel_format_input,
            |this, input, event: &InputStateEvent, cx| {
                if matches!(event, InputStateEvent::TextChanged) {
                    let value = input.read(cx).content();
                    let trimmed = value.trim();
                    this.state.config.excel.formato_nombre_archivo = if trimmed.is_empty() {
                        "cfdi_{YYYYMMDD}_{HHmm}.xlsx".to_owned()
                    } else {
                        trimmed.to_owned()
                    };
                    cx.notify();
                }
            },
        );
        let shortcut_subscriptions = std::array::from_fn(|index| {
            let input = shortcut_inputs[index].clone();
            cx.subscribe(&input, move |this, input, event: &InputStateEvent, cx| {
                if matches!(event, InputStateEvent::TextChanged) {
                    let value = input.read(cx).content().trim().to_owned();
                    let current = match index {
                        0 => &mut this.state.config.atajos.abrir_zip,
                        1 => &mut this.state.config.atajos.exportar,
                        2 => &mut this.state.config.atajos.abrir_configuracion,
                        3 => &mut this.state.config.atajos.pestaña_1,
                        4 => &mut this.state.config.atajos.pestaña_2,
                        _ => return,
                    };
                    *current = value;
                    cx.notify();
                }
            })
        });
        let tab_subscription = Self::subscribe_tabs(cx, &tabs);
        Self {
            state,
            root_focus: cx.focus_handle(),
            rfc_input,
            excel_format_input,
            tabs,
            config_repository,
            table_scroll_handles: std::array::from_fn(|_| UniformListScrollHandle::new()),
            table_h_scroll_handles: std::array::from_fn(|_| ScrollHandle::new()),
            _rfc_subscription: rfc_subscription,
            _excel_format_subscription: excel_format_subscription,
            _shortcut_subscriptions: shortcut_subscriptions,
            tab_subscription,
            pending_drop: None,
            pending_toast: None,
            _appearance_subscription: Some(appearance_subscription),
        }
    }

    fn subscribe_tabs(cx: &mut Context<Self>, tabs: &Entity<KitTabs>) -> Subscription {
        cx.subscribe(tabs, |this, _, event: &TabChanged, cx| {
            if let Some(tab) = tabs::from_id(event.tab_id.as_ref()) {
                this.state.vista = tab;
                cx.notify();
            }
        })
    }

    fn replace_tabs(&mut self, cx: &mut Context<Self>) {
        let counts = self.state.resultados.as_ref().map(|r| tabs::TabCounts {
            cfdi: r.filas.len(),
            resumen: r.resumen.len(),
            clasificacion: r.clasificaciones.len(),
            errores: r.errores.len(),
        });
        let tabs = tabs::create(cx, counts, self.state.vista);
        self.tab_subscription = Self::subscribe_tabs(cx, &tabs);
        self.tabs = tabs;
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
                    this.pending_toast = Some((
                        "No se pudo abrir el selector de archivos.".to_owned(),
                        false,
                    ));
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
            let message = "Selecciona un archivo con extensión .zip.";
            self.state.set_error(message);
            self.pending_toast = Some((message.to_owned(), false));
        }
        self.replace_tabs(cx);
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
                let tablas = TablasPreparadas::desde(&result);
                let _ = sender.send_blocking(WorkerMessage::Complete(result, tablas));
            })
            .detach();

        cx.spawn(async move |this, cx| {
            while let Ok(message) = receiver.recv().await {
                let complete = matches!(message, WorkerMessage::Complete(..));
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
            WorkerMessage::Complete(result, tablas) => {
                self.state.finish_processing(result, tablas);
                self.replace_tabs(cx);
                if self.state.estado == EstadoApp::Error {
                    let message = self
                        .state
                        .mensaje_error
                        .clone()
                        .unwrap_or_else(|| "No se pudo procesar el ZIP.".to_owned());
                    self.pending_toast = Some((message, false));
                }
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
                    this.pending_toast = Some((
                        "No se pudo abrir el selector para guardar.".to_owned(),
                        false,
                    ));
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
                        this.pending_toast =
                            Some(("Excel exportado correctamente.".to_owned(), true));
                    }
                    Err(error) => {
                        eprintln!("No se pudo exportar el Excel: {error}");
                        this.state.mensaje_error =
                            Some("No se pudo exportar el archivo Excel.".to_owned());
                        this.pending_toast =
                            Some(("No se pudo exportar el archivo Excel.".to_owned(), false));
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
            "," => {
                self.open_settings(cx);
                true
            }
            _ => false,
        };
        if handled {
            window.focus(&self.root_focus, cx);
        }
    }

    fn select_tab(&mut self, tab: Vista, cx: &mut Context<Self>) {
        self.state.vista = tab;
        self.tabs
            .update(cx, |tabs, cx| tabs.set_selected(tab.id().into(), cx));
        cx.notify();
    }

    fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.state.settings_open = true;
        self.state.settings_section = SettingsSection::General;
        cx.notify();
    }

    fn close_settings(&mut self, cx: &mut Context<Self>) {
        self.state.settings_open = false;
        cx.notify();
    }

    fn restore_section_defaults(&mut self) {
        match self.state.settings_section {
            SettingsSection::General => {
                self.state.config.general = crate::domain::config::GeneralConfig::default()
            }
            SettingsSection::Extraccion => {
                self.state.config.extraccion = crate::domain::config::ExtraccionConfig::default()
            }
            SettingsSection::Clasificacion => {
                self.state.config.clasificacion =
                    crate::domain::config::ClasificacionConfig::default()
            }
            SettingsSection::Excel => {
                self.state.config.excel = crate::domain::config::ExcelConfig::default()
            }
            SettingsSection::Tabla => {
                self.state.config.tabla = crate::domain::config::TablaConfig::default()
            }
            SettingsSection::Apariencia => {
                self.state.config.apariencia = crate::domain::config::AparienciaConfig::default()
            }
            SettingsSection::Atajos => {
                self.state.config.atajos = crate::domain::config::AtajosConfig::default()
            }
            SettingsSection::Avanzado => {
                self.state.config.avanzado = crate::domain::config::AvanzadoConfig::default()
            }
        }
        self.state.sync_rfc_from_config();
        self.pending_toast = Some(("Sección restaurada".to_owned(), true));
    }

    fn restore_all_defaults(&mut self) {
        self.state.config = AppConfig::default();
        self.state.sync_rfc_from_config();
        self.pending_toast = Some(("Configuración restaurada".to_owned(), true));
    }

    fn save_config(&mut self, cx: &mut Context<Self>) {
        let config = self.state.config.clone();
        let result = ConfigService::new(self.config_repository.clone()).guardar(&config);
        self.state.settings_open = false;
        if result.is_ok() {
            self.pending_toast = Some(("Configuración guardada".to_owned(), true));
        } else {
            self.pending_toast = Some(("No se pudo guardar la configuración".to_owned(), false));
        }
        cx.notify();
    }

    fn render_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.state.estado {
            EstadoApp::Vacio => DropZone::new(
                "Seleccionar archivo ZIP",
                cx.listener(|this, _, _, cx| this.open_zip_dialog(cx)),
            )
            .into_any_element(),
            EstadoApp::ZipCargado => v_stack()
                .items_center()
                .justify_center()
                .gap_4()
                .child(
                    text(
                        self.state
                            .ruta_zip
                            .as_deref()
                            .and_then(Path::file_name)
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    )
                    .muted(),
                )
                .child(DropZone::new(
                    "Seleccionar otro ZIP",
                    cx.listener(|this, _, _, cx| this.open_zip_dialog(cx)),
                ))
                .into_any_element(),
            EstadoApp::Error => v_stack()
                .items_center()
                .justify_center()
                .gap_4()
                .child(text(self.state.mensaje_error.clone().unwrap_or_default()).destructive())
                .child(DropZone::new(
                    "Seleccionar otro ZIP",
                    cx.listener(|this, _, _, cx| this.open_zip_dialog(cx)),
                ))
                .into_any_element(),
            EstadoApp::Procesando | EstadoApp::Resultados => {
                self.render_preview().into_any_element()
            }
        }
    }

    fn render_settings_modal(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let config = self.state.config.clone();
        let invalid = config.validar();
        let preview = format_filename_preview(&config.excel.formato_nombre_archivo);
        let section = self.state.settings_section;

        let nav = div()
            .w_full()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(
                button("settings-general", "General").on_click(cx.listener(|this, _, _, cx| {
                    this.state.settings_section = SettingsSection::General;
                    cx.notify();
                })),
            )
            .child(
                button("settings-extraccion", "Extracción").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.state.settings_section = SettingsSection::Extraccion;
                        cx.notify();
                    },
                )),
            )
            .child(
                button("settings-clasificacion", "Clasificación").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.state.settings_section = SettingsSection::Clasificacion;
                        cx.notify();
                    },
                )),
            )
            .child(
                button("settings-excel", "Excel").on_click(cx.listener(|this, _, _, cx| {
                    this.state.settings_section = SettingsSection::Excel;
                    cx.notify();
                })),
            )
            .child(
                button("settings-tabla", "Tabla").on_click(cx.listener(|this, _, _, cx| {
                    this.state.settings_section = SettingsSection::Tabla;
                    cx.notify();
                })),
            )
            .child(
                button("settings-apariencia", "Apariencia").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.state.settings_section = SettingsSection::Apariencia;
                        cx.notify();
                    },
                )),
            )
            .child(
                button("settings-atajos", "Atajos").on_click(cx.listener(|this, _, _, cx| {
                    this.state.settings_section = SettingsSection::Atajos;
                    cx.notify();
                })),
            )
            .child(
                button("settings-avanzado", "Avanzado").on_click(cx.listener(|this, _, _, cx| {
                    this.state.settings_section = SettingsSection::Avanzado;
                    cx.notify();
                })),
            )
            .child(
                button("restore-section", "Restaurar sección").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.restore_section_defaults();
                        cx.notify();
                    },
                )),
            )
            .child(
                button("restore-all", "Restaurar todo").on_click(cx.listener(|this, _, _, cx| {
                    this.restore_all_defaults();
                    cx.notify();
                })),
            )
            .child(
                button("import-settings", "Importar").on_click(cx.listener(|this, _, _, cx| {
                    this.pending_toast =
                        Some(("Importar configuración (próximamente)".to_owned(), true));
                    cx.notify();
                })),
            )
            .child(
                button("export-settings", "Exportar").on_click(cx.listener(|this, _, _, cx| {
                    this.pending_toast =
                        Some(("Exportar configuración (próximamente)".to_owned(), true));
                    cx.notify();
                })),
            )
            .child(
                button("save-settings", "Guardar")
                    .on_click(cx.listener(|this, _, _, cx| this.save_config(cx))),
            );

        let section_content = match section {
            SettingsSection::General => v_stack()
                .gap_3()
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("RFC propio").small().muted())
                        .child(div().w(px(220.)).child(
                            text_field(&self.rfc_input, cx).placeholder("Ej. ABC010101ABC"),
                        )),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Recordar RFC").small().muted())
                        .child(
                            button(
                                "remember-rfc",
                                if self.state.config.general.recordar_rfc {
                                    "Activado"
                                } else {
                                    "Desactivado"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.general.recordar_rfc =
                                    !this.state.config.general.recordar_rfc;
                                cx.notify();
                            })),
                        ),
                )
                .into_any_element(),
            SettingsSection::Extraccion => v_stack()
                .gap_3()
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Impuestos por concepto").small().muted())
                        .child(
                            button(
                                "repetir-impuestos",
                                if self
                                    .state
                                    .config
                                    .extraccion
                                    .repetir_impuestos_en_cada_concepto
                                {
                                    "Repetir en cada concepto"
                                } else {
                                    "Solo en la primera fila"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state
                                    .config
                                    .extraccion
                                    .repetir_impuestos_en_cada_concepto = !this
                                    .state
                                    .config
                                    .extraccion
                                    .repetir_impuestos_en_cada_concepto;
                                cx.notify();
                            })),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Tipos incluidos").small().muted())
                        .child(
                            h_stack()
                                .gap_2()
                                .child(
                                    button(
                                        "tipo-i",
                                        if self.state.config.extraccion.incluir_tipos.i {
                                            "I: Sí"
                                        } else {
                                            "I: No"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.extraccion.incluir_tipos.i =
                                                !this.state.config.extraccion.incluir_tipos.i;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "tipo-e",
                                        if self.state.config.extraccion.incluir_tipos.e {
                                            "E: Sí"
                                        } else {
                                            "E: No"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.extraccion.incluir_tipos.e =
                                                !this.state.config.extraccion.incluir_tipos.e;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "tipo-p",
                                        if self.state.config.extraccion.incluir_tipos.p {
                                            "P: Sí"
                                        } else {
                                            "P: No"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.extraccion.incluir_tipos.p =
                                                !this.state.config.extraccion.incluir_tipos.p;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "tipo-n",
                                        if self.state.config.extraccion.incluir_tipos.n {
                                            "N: Sí"
                                        } else {
                                            "N: No"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.extraccion.incluir_tipos.n =
                                                !this.state.config.extraccion.incluir_tipos.n;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "tipo-t",
                                        if self.state.config.extraccion.incluir_tipos.t {
                                            "T: Sí"
                                        } else {
                                            "T: No"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.extraccion.incluir_tipos.t =
                                                !this.state.config.extraccion.incluir_tipos.t;
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Origen emitida/recibida").small().muted())
                        .child(
                            button(
                                "columna-origen",
                                if self.state.config.extraccion.columna_origen_emitida_recibida {
                                    "Activada"
                                } else {
                                    "Desactivada"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.extraccion.columna_origen_emitida_recibida =
                                    !this.state.config.extraccion.columna_origen_emitida_recibida;
                                cx.notify();
                            })),
                        ),
                )
                .into_any_element(),
            SettingsSection::Clasificacion => v_stack()
                .gap_3()
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Regla por defecto").small().muted())
                        .child(
                            text(&self.state.config.clasificacion.categoria_por_defecto)
                                .small()
                                .muted(),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Nómina por tipo N").small().muted())
                        .child(
                            button(
                                "regla-nomina",
                                if self.state.config.clasificacion.regla_nomina_tipo_n {
                                    "Activada"
                                } else {
                                    "Desactivada"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.clasificacion.regla_nomina_tipo_n =
                                    !this.state.config.clasificacion.regla_nomina_tipo_n;
                                cx.notify();
                            })),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Ingreso/Gasto por RFC").small().muted())
                        .child(
                            button(
                                "regla-rfc",
                                if self.state.config.clasificacion.regla_ingreso_gasto_por_rfc {
                                    "Activada"
                                } else {
                                    "Desactivada"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.clasificacion.regla_ingreso_gasto_por_rfc =
                                    !this.state.config.clasificacion.regla_ingreso_gasto_por_rfc;
                                cx.notify();
                            })),
                        ),
                )
                .into_any_element(),
            SettingsSection::Excel => v_stack()
                .gap_3()
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Formato Excel").small().muted())
                        .child(
                            div().w(px(260.)).child(
                                text_field(&self.excel_format_input, cx)
                                    .placeholder("cfdi_{YYYYMMDD}_{HHmm}.xlsx"),
                            ),
                        ),
                )
                .child(div().child(text(format!("Vista previa: {}", preview)).small().muted()))
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Cabeceras congeladas").small().muted())
                        .child(
                            button(
                                "excel-freeze-header",
                                if self.state.config.excel.congelar_encabezado {
                                    "Sí"
                                } else {
                                    "No"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.excel.congelar_encabezado =
                                    !this.state.config.excel.congelar_encabezado;
                                cx.notify();
                            })),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Autofiltro").small().muted())
                        .child(
                            button(
                                "excel-autofilter",
                                if self.state.config.excel.autofiltro {
                                    "Sí"
                                } else {
                                    "No"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.excel.autofiltro =
                                    !this.state.config.excel.autofiltro;
                                cx.notify();
                            })),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Ancho automático").small().muted())
                        .child(
                            button(
                                "excel-auto-width",
                                if self.state.config.excel.ancho_automatico {
                                    "Sí"
                                } else {
                                    "No"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.excel.ancho_automatico =
                                    !this.state.config.excel.ancho_automatico;
                                cx.notify();
                            })),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Abrir carpeta al exportar").small().muted())
                        .child(
                            button(
                                "open-folder-export",
                                if self.state.config.excel.abrir_carpeta_al_exportar {
                                    "Sí"
                                } else {
                                    "No"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.excel.abrir_carpeta_al_exportar =
                                    !this.state.config.excel.abrir_carpeta_al_exportar;
                                cx.notify();
                            })),
                        ),
                )
                .into_any_element(),
            SettingsSection::Tabla => v_stack()
                .gap_3()
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Altura de fila").small().muted())
                        .child(
                            h_stack()
                                .gap_2()
                                .child(
                                    button(
                                        "row-height-28",
                                        if self.state.config.tabla.altura_fila == 28 {
                                            "28 ✓"
                                        } else {
                                            "28"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.tabla.altura_fila = 28;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "row-height-32",
                                        if self.state.config.tabla.altura_fila == 32 {
                                            "32 ✓"
                                        } else {
                                            "32"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.tabla.altura_fila = 32;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "row-height-36",
                                        if self.state.config.tabla.altura_fila == 36 {
                                            "36 ✓"
                                        } else {
                                            "36"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.tabla.altura_fila = 36;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "row-height-40",
                                        if self.state.config.tabla.altura_fila == 40 {
                                            "40 ✓"
                                        } else {
                                            "40"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.tabla.altura_fila = 40;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "row-height-44",
                                        if self.state.config.tabla.altura_fila == 44 {
                                            "44 ✓"
                                        } else {
                                            "44"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.tabla.altura_fila = 44;
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Hover").small().muted())
                        .child(
                            button(
                                "mostrar-hover",
                                if self.state.config.tabla.mostrar_hover {
                                    "Activado"
                                } else {
                                    "Desactivado"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.tabla.mostrar_hover =
                                    !this.state.config.tabla.mostrar_hover;
                                cx.notify();
                            })),
                        ),
                )
                .into_any_element(),
            SettingsSection::Apariencia => v_stack()
                .gap_3()
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Tema").small().muted())
                        .child(
                            h_stack()
                                .gap_2()
                                .child(
                                    button(
                                        "theme-system",
                                        if matches!(
                                            self.state.config.apariencia.tema,
                                            crate::domain::config::TemaApp::Sistema
                                        ) {
                                            "Sistema ✓"
                                        } else {
                                            "Sistema"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.apariencia.tema =
                                                crate::domain::config::TemaApp::Sistema;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "theme-light",
                                        if matches!(
                                            self.state.config.apariencia.tema,
                                            crate::domain::config::TemaApp::Claro
                                        ) {
                                            "Claro ✓"
                                        } else {
                                            "Claro"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.apariencia.tema =
                                                crate::domain::config::TemaApp::Claro;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "theme-dark",
                                        if matches!(
                                            self.state.config.apariencia.tema,
                                            crate::domain::config::TemaApp::Oscuro
                                        ) {
                                            "Oscuro ✓"
                                        } else {
                                            "Oscuro"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.apariencia.tema =
                                                crate::domain::config::TemaApp::Oscuro;
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Densidad").small().muted())
                        .child(
                            h_stack()
                                .gap_2()
                                .child(
                                    button(
                                        "density-compact",
                                        if matches!(
                                            self.state.config.apariencia.densidad,
                                            crate::domain::config::DensidadApp::Compacta
                                        ) {
                                            "Compacta ✓"
                                        } else {
                                            "Compacta"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.apariencia.densidad =
                                                crate::domain::config::DensidadApp::Compacta;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "density-normal",
                                        if matches!(
                                            self.state.config.apariencia.densidad,
                                            crate::domain::config::DensidadApp::Normal
                                        ) {
                                            "Normal ✓"
                                        } else {
                                            "Normal"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.apariencia.densidad =
                                                crate::domain::config::DensidadApp::Normal;
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Color de acento").small().muted())
                        .child(
                            text(&self.state.config.apariencia.color_acento)
                                .small()
                                .muted(),
                        ),
                )
                .into_any_element(),
            SettingsSection::Atajos => v_stack()
                .gap_3()
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Atajo abrir ZIP").small().muted())
                        .child(text(&self.state.config.atajos.abrir_zip).small().muted()),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Atajo exportar").small().muted())
                        .child(text(&self.state.config.atajos.exportar).small().muted()),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Abrir configuración").small().muted())
                        .child(
                            text(&self.state.config.atajos.abrir_configuracion)
                                .small()
                                .muted(),
                        ),
                )
                .into_any_element(),
            SettingsSection::Avanzado => v_stack()
                .gap_3()
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Hilos de procesamiento").small().muted())
                        .child(
                            h_stack()
                                .gap_2()
                                .child(
                                    button(
                                        "threads-auto",
                                        if self.state.config.avanzado.hilos_de_procesamiento
                                            == "Auto"
                                        {
                                            "Auto ✓"
                                        } else {
                                            "Auto"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.avanzado.hilos_de_procesamiento =
                                                "Auto".to_owned();
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "threads-1",
                                        if self.state.config.avanzado.hilos_de_procesamiento == "1"
                                        {
                                            "1 ✓"
                                        } else {
                                            "1"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.avanzado.hilos_de_procesamiento =
                                                "1".to_owned();
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "threads-2",
                                        if self.state.config.avanzado.hilos_de_procesamiento == "2"
                                        {
                                            "2 ✓"
                                        } else {
                                            "2"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.avanzado.hilos_de_procesamiento =
                                                "2".to_owned();
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "threads-4",
                                        if self.state.config.avanzado.hilos_de_procesamiento == "4"
                                        {
                                            "4 ✓"
                                        } else {
                                            "4"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.state.config.avanzado.hilos_de_procesamiento =
                                                "4".to_owned();
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Errores detallados").small().muted())
                        .child(
                            button(
                                "mostrar-errores-detallados",
                                if self.state.config.avanzado.mostrar_errores_detallados {
                                    "Sí"
                                } else {
                                    "No"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.avanzado.mostrar_errores_detallados =
                                    !this.state.config.avanzado.mostrar_errores_detallados;
                                cx.notify();
                            })),
                        ),
                )
                .child(
                    h_stack()
                        .gap_3()
                        .items_center()
                        .child(text("Registro de depuración").small().muted())
                        .child(
                            button(
                                "registro-depuracion",
                                if self.state.config.avanzado.registro_de_depuracion {
                                    "Sí"
                                } else {
                                    "No"
                                },
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.config.avanzado.registro_de_depuracion =
                                    !this.state.config.avanzado.registro_de_depuracion;
                                cx.notify();
                            })),
                        ),
                )
                .into_any_element(),
        };

        let form = v_stack()
            .w_full()
            .gap_3()
            .child(
                h_stack()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .child(text("Configuración").small())
                    .child(
                        button("close-settings", "Cerrar")
                            .on_click(cx.listener(|this, _, _, cx| this.close_settings(cx))),
                    ),
            )
            .child(nav)
            .child(
                div()
                    .w_full()
                    .max_h(px(420.))
                    .overflow_hidden()
                    .pr_1()
                    .child(section_content),
            );

        let form = if invalid.is_empty() {
            form
        } else {
            form.child(div().child(text(invalid[0].mensaje.clone()).small().destructive()))
        };

        div()
            .absolute()
            .inset_0()
            .bg(cx.theme().bg())
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(900.))
                    .max_w(px(900.))
                    .max_h(px(620.))
                    .overflow_hidden()
                    .bg(cx.theme().surface())
                    .border_1()
                    .border_color(cx.theme().border())
                    .rounded(px(12.))
                    .p_4()
                    .child(form),
            )
    }

    fn render_preview(&mut self) -> impl IntoElement {
        let Some(resultados) = &self.state.resultados else {
            return small("Preparando vista previa…").muted().into_any_element();
        };
        match self.state.vista {
            Vista::Cfdi => DataTable::new(
                &CFDI_COLUMNS,
                Arc::clone(&resultados.tablas.cfdi),
                self.table_scroll_handles[0].clone(),
                self.table_h_scroll_handles[0].clone(),
                "cfdi-rows",
            ),
            Vista::Resumen => DataTable::new(
                &RESUMEN_COLUMNS,
                Arc::clone(&resultados.tablas.resumen),
                self.table_scroll_handles[1].clone(),
                self.table_h_scroll_handles[1].clone(),
                "summary-rows",
            ),
            Vista::Clasificacion => DataTable::new(
                &CLASIFICACION_COLUMNS,
                Arc::clone(&resultados.tablas.clasificaciones),
                self.table_scroll_handles[2].clone(),
                self.table_h_scroll_handles[2].clone(),
                "classification-rows",
            ),
            Vista::Errores => DataTable::new(
                &ERROR_COLUMNS,
                Arc::clone(&resultados.tablas.errores),
                self.table_scroll_handles[3].clone(),
                self.table_h_scroll_handles[3].clone(),
                "error-rows",
            ),
        }
        .into_any_element()
    }
}

enum WorkerMessage {
    Progress(usize, usize),
    Complete(ResultadoProceso, TablasPreparadas),
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        theme::follow_system_appearance(window.appearance(), cx);
        let status = self.state.status_text();
        let exported = self.state.ruta_exportada.clone();
        let is_processing = self.state.estado == EstadoApp::Procesando;
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
        let state = self.state.clone();
        if let Some((message, success)) = self.pending_toast.take() {
            window.defer(cx, move |window, cx| {
                let toast = cx.toast(message);
                if success {
                    toast.success().show(window, cx);
                } else {
                    toast.destructive().show(window, cx);
                }
            });
        }

        let progress_element = if is_processing {
            progress_bar(progress_fraction)
                .height(px(2.))
                .into_any_element()
        } else {
            div().h(px(2.)).into_any_element()
        };

        v_stack()
            .id("app-root")
            .track_focus(&root_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let command =
                    event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
                if command {
                    this.on_shortcut(event, window, cx);
                }
            }))
            .on_click(cx.listener(|this, _, window, cx| {
                let rfc_focus = this.rfc_input.read(cx).focus_handle(cx);
                if !rfc_focus.is_focused(window) {
                    window.focus(&this.root_focus, cx);
                }
            }))
            .size_full()
            .font(gpui::Font {
                features: theme::tabular_numbers(),
                ..gpui::font("system-ui")
            })
            .text_color(cx.theme().fg())
            .bg(cx.theme().bg())
            .child(Toolbar::new(
                self.state
                    .ruta_zip
                    .as_deref()
                    .and_then(Path::file_name)
                    .map(|n| n.to_string_lossy().to_string()),
                is_exportable,
                cx.listener(|this, _, _, cx| this.open_zip_dialog(cx)),
                cx.listener(|this, _, _, cx| this.choose_export_path(cx)),
            ))
            .child(self.render_options(cx))
            .child(progress_element)
            .when(is_processing, |element| {
                element.child(div().px_6().py_1().child(small(progress_label).muted()))
            })
            .when(state.resultados.is_some(), |element| {
                element
                    .child(Metrics::new(state.clone()))
                    .child(div().px_6().py_1().child(self.tabs.clone()))
            })
            .child(
                v_stack()
                    .flex_1()
                    .w_full()
                    .min_w_0()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .px_6()
                    .py_2()
                    .when(state.resultados.is_some(), |element| {
                        element.items_start().justify_start().px_6().py_2()
                    })
                    .child(self.render_content(cx)),
            )
            .child(StatusBar::new(
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
            .when(self.state.settings_open, |element| {
                element.child(self.render_settings_modal(cx))
            })
            .child(drop_listener)
    }
}

impl AppView {
    fn render_options(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = self.state.ruta_zip.is_some()
            && matches!(
                self.state.estado,
                EstadoApp::ZipCargado | EstadoApp::Resultados
            );
        let is_processing = self.state.estado == EstadoApp::Procesando;
        let validation = validar_rfc(&self.state.rfc_propio);
        let has_rfc = !self.state.rfc_propio.trim().is_empty();

        let zip_label = self
            .state
            .ruta_zip
            .as_deref()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().to_string());

        h_stack()
            .w_full()
            .items_center()
            .justify_between()
            .gap_4()
            .px_6()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border())
            .bg(cx.theme().surface())
            .child(
                h_stack()
                    .gap_3()
                    .items_center()
                    .child(text("RFC propio (opcional):").small().muted())
                    .child(
                        div()
                            .w(px(200.))
                            .child(text_field(&self.rfc_input, cx).placeholder("Ej. AAA010101AAA")),
                    )
                    .child(if let Some(err) = validation {
                        badge(err).destructive().control_size(ControlSize::Small)
                    } else if has_rfc {
                        badge("RFC válido")
                            .secondary()
                            .control_size(ControlSize::Small)
                    } else {
                        badge("Opcional: guía Ingreso/Gasto")
                            .outline()
                            .control_size(ControlSize::Small)
                    }),
            )
            .child(
                h_stack()
                    .gap_3()
                    .items_center()
                    .when_some(zip_label, |stack, name| {
                        stack.child(small(format!("Listo: {}", name)).muted())
                    })
                    .child(
                        button(
                            "process-button",
                            if is_processing {
                                "Procesando…"
                            } else {
                                "Procesar ZIP"
                            },
                        )
                        .disabled(!enabled || is_processing)
                        .on_click(cx.listener(|this, _, _, cx| this.process_zip(cx))),
                    )
                    .child(
                        button("settings-button", "⚙")
                            .control_size(ControlSize::Small)
                            .on_click(cx.listener(|this, _, _, cx| this.open_settings(cx))),
                    ),
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

fn format_filename_preview(template: &str) -> String {
    let now = Local::now();
    template
        .replace("{YYYYMMDD}", &now.format("%Y%m%d").to_string())
        .replace("{HHmm}", &now.format("%H%M").to_string())
        .replace("{YYYY}", &now.format("%Y").to_string())
        .replace("{MM}", &now.format("%m").to_string())
        .replace("{DD}", &now.format("%d").to_string())
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

pub fn format_integer(value: usize) -> String {
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
        let resultado = ResultadoProceso {
            filas: Vec::new(),
            resumen: Vec::new(),
            clasificaciones: Vec::new(),
            errores: Vec::new(),
            total_xml_leidos: 1,
            total_cfdi_ok: 1,
        };
        let tablas = TablasPreparadas::desde(&resultado);
        state.finish_processing(resultado, tablas);
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
