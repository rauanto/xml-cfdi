//! Dimensiones específicas de la ventana y tabla; los colores y controles vienen de gpuikit.

use std::sync::Arc;

use gpui::{App, FontFeatures, WindowAppearance};
use gpuikit::theme::{ActiveTheme, GlobalTheme, Theme, ThemeVariant};

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
    let (variant, theme) = match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => {
            (ThemeVariant::Dark, Theme::gruvbox_dark())
        }
        WindowAppearance::Light | WindowAppearance::VibrantLight => {
            (ThemeVariant::Light, Theme::gruvbox_light())
        }
    };
    if cx.theme().variant != variant {
        cx.set_global(GlobalTheme(Arc::new(theme)));
    }
}
