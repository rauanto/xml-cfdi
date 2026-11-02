//! Colores y medidas compartidos por todas las vistas.

use std::sync::Arc;

use gpui::{FontFeatures, Hsla, WindowAppearance, rgb};

pub const WINDOW_WIDTH: f32 = 1200.0;
pub const WINDOW_HEIGHT: f32 = 760.0;
pub const WINDOW_MIN_WIDTH: f32 = 900.0;
pub const WINDOW_MIN_HEIGHT: f32 = 560.0;
pub const TOP_BAR_HEIGHT: f32 = 48.0;
pub const OPTIONS_HEIGHT: f32 = 48.0;
pub const METRICS_HEIGHT: f32 = 36.0;
pub const TABS_HEIGHT: f32 = 36.0;
pub const STATUS_HEIGHT: f32 = 28.0;
pub const ROW_HEIGHT: f32 = 32.0;
pub const HEADER_HEIGHT: f32 = 32.0;
pub const TABLE_NAV_HEIGHT: f32 = 32.0;
pub const PROGRESS_HEIGHT: f32 = 2.0;
pub const BORDER_WIDTH: f32 = 1.0;
pub const CORNER_RADIUS: f32 = 6.0;
pub const SPACING: f32 = 8.0;
pub const SMALL_SPACING: f32 = 4.0;
pub const CONTENT_PADDING: f32 = 24.0;
pub const CONTROL_HEIGHT: f32 = 32.0;
pub const RFC_INPUT_WIDTH: f32 = 224.0;
pub const DROP_ZONE_WIDTH: f32 = 560.0;
pub const DROP_ZONE_HEIGHT: f32 = 192.0;
pub const FONT_SMALL: f32 = 12.0;
pub const FONT_BODY: f32 = 13.0;
pub const FONT_LABEL: f32 = 14.0;
pub const FONT_TITLE: f32 = 18.0;

pub fn tabular_numbers() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".to_owned(), 1)]))
}

pub const WIDTH_UUID: f32 = 240.0;
pub const WIDTH_DATE: f32 = 150.0;
pub const WIDTH_RFC: f32 = 140.0;
pub const WIDTH_NAME: f32 = 220.0;
pub const WIDTH_TYPE: f32 = 120.0;
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

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Hsla,
    pub surface: Hsla,
    pub text: Hsla,
    pub secondary: Hsla,
    pub border: Hsla,
    pub accent: Hsla,
    pub accent_text: Hsla,
    pub error: Hsla,
    pub hover: Hsla,
}

pub fn for_appearance(appearance: WindowAppearance) -> Palette {
    let dark = matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    );

    if dark {
        Palette {
            background: rgb(0x111827).into(),
            surface: rgb(0x1F2937).into(),
            text: rgb(0xF9FAFB).into(),
            secondary: rgb(0x9CA3AF).into(),
            border: rgb(0x374151).into(),
            accent: rgb(0x4F46E5).into(),
            accent_text: rgb(0xFFFFFF).into(),
            error: rgb(0xFCA5A5).into(),
            hover: rgb(0x273449).into(),
        }
    } else {
        Palette {
            background: rgb(0xFAFAFA).into(),
            surface: rgb(0xFFFFFF).into(),
            text: rgb(0x111111).into(),
            secondary: rgb(0x6B7280).into(),
            border: rgb(0xE5E7EB).into(),
            accent: rgb(0x4F46E5).into(),
            accent_text: rgb(0xFFFFFF).into(),
            error: rgb(0xB45353).into(),
            hover: rgb(0xF5F5F5).into(),
        }
    }
}
