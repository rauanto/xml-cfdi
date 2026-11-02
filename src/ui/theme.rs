//! Dimensiones específicas de la ventana y tabla; los colores y controles vienen de gpuikit.

use std::sync::Arc;

use gpui::{App, FontFeatures, WindowAppearance, hsla};
use gpuikit::theme::{ActiveTheme, GlobalTheme, Theme, ThemeVariant};

use crate::domain::config::TemaApp;

pub const WINDOW_WIDTH: f32 = 1200.0;
pub const WINDOW_HEIGHT: f32 = 760.0;
pub const WINDOW_MIN_WIDTH: f32 = 900.0;
pub const WINDOW_MIN_HEIGHT: f32 = 560.0;
pub const ROW_HEIGHT: f32 = 32.0;
pub const HEADER_HEIGHT: f32 = 32.0;
pub const DROP_ZONE_WIDTH: f32 = 560.0;
pub const DROP_ZONE_HEIGHT: f32 = 192.0;
pub const WIDTH_UUID: f32 = 240.0;
pub const WIDTH_DATE: f32 = 150.0;
pub const WIDTH_RFC: f32 = 140.0;
pub const WIDTH_NAME: f32 = 220.0;
pub const WIDTH_TYPE: f32 = 150.0;
pub const WIDTH_MONEY: f32 = 120.0;
pub const WIDTH_CODE: f32 = 120.0;
pub const WIDTH_DESCRIPTION: f32 = 260.0;
pub const WIDTH_FILE: f32 = 240.0;
pub const WIDTH_METHOD: f32 = 120.0;
pub const WIDTH_CURRENCY: f32 = 96.0;
pub const WIDTH_EXCHANGE: f32 = 120.0;
pub const WIDTH_COUNT: f32 = 144.0;
pub const WIDTH_CATEGORY: f32 = 160.0;
pub const WIDTH_CRITERION: f32 = 300.0;
pub const WIDTH_ERROR: f32 = 360.0;

pub fn tabular_numbers() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".to_owned(), 1)]))
}

pub fn follow_system_appearance(appearance: WindowAppearance, cx: &mut App) {
    apply_configured_appearance(&TemaApp::Sistema, appearance, cx);
}

pub fn apply_configured_appearance(
    configured: &TemaApp,
    appearance: WindowAppearance,
    cx: &mut App,
) {
    let is_dark = match configured {
        TemaApp::Sistema => matches!(
            appearance,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ),
        TemaApp::Oscuro => true,
        TemaApp::Claro => false,
    };
    let theme = if is_dark {
        Theme::gruvbox_dark()
    } else {
        simple_light_theme()
    };
    if cx.theme().name != theme.name {
        cx.set_global(GlobalTheme(Arc::new(theme)));
    }
}

fn simple_light_theme() -> Theme {
    let mut theme = Theme::new(
        "Simple Light",
        ThemeVariant::Light,
        hsla(0.0, 0.0, 0.0, 1.0),
        hsla(0.0, 0.0, 1.0, 1.0),
        hsla(0.0, 0.0, 1.0, 1.0),
        hsla(0.0, 0.0, 0.82, 1.0),
        hsla(0.0, 0.0, 0.0, 1.0),
    );
    theme.fg_muted_color = Some(hsla(0.0, 0.0, 0.32, 1.0));
    theme.fg_disabled_color = Some(hsla(0.0, 0.0, 0.58, 1.0));
    theme.surface_secondary_color = Some(hsla(0.0, 0.0, 0.96, 1.0));
    theme.surface_tertiary_color = Some(hsla(0.0, 0.0, 0.92, 1.0));
    theme.border_secondary_color = Some(hsla(0.0, 0.0, 0.68, 1.0));
    theme.border_subtle_color = Some(hsla(0.0, 0.0, 0.9, 1.0));
    theme.outline_color = Some(hsla(0.0, 0.0, 0.0, 1.0));
    theme.selection_color = Some(hsla(0.0, 0.0, 0.0, 0.15));
    theme.button_bg_color = Some(hsla(0.0, 0.0, 1.0, 1.0));
    theme.button_bg_hover_color = Some(hsla(0.0, 0.0, 0.96, 1.0));
    theme.button_bg_active_color = Some(hsla(0.0, 0.0, 0.9, 1.0));
    theme.button_border_color = Some(hsla(0.0, 0.0, 0.78, 1.0));
    theme.input_bg_color = Some(hsla(0.0, 0.0, 1.0, 1.0));
    theme.input_border_color = Some(hsla(0.0, 0.0, 0.78, 1.0));
    theme.input_border_hover_color = Some(hsla(0.0, 0.0, 0.55, 1.0));
    theme.input_border_focused_color = Some(hsla(0.0, 0.0, 0.0, 1.0));
    theme.input_text_color = Some(hsla(0.0, 0.0, 0.0, 1.0));
    theme.input_placeholder_color = Some(hsla(0.0, 0.0, 0.48, 1.0));
    theme.input_cursor_color = Some(hsla(0.0, 0.0, 0.0, 1.0));
    theme
}
