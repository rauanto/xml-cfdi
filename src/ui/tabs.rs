//! Pestañas gpuikit para seleccionar las tablas de vista previa.

use gpui::{AppContext, Context, Entity};
use gpuikit::elements::tabs::{Tabs, tab, tabs};

use super::app_view::{AppView, Vista};

pub fn create(cx: &mut Context<AppView>, error_count: usize, active: Vista) -> Entity<Tabs> {
    let error_label = if error_count > 0 {
        format!("Errores ({error_count})")
    } else {
        "Errores".to_owned()
    };

    cx.new(|_| {
        tabs("preview-tabs")
            .tab(tab("cfdi", "CFDI"))
            .tab(tab("resumen", "Resumen por producto"))
            .tab(tab("clasificacion", "Clasificación sugerida"))
            .tab(tab("errores", error_label))
            .selected(active.id())
    })
}

pub fn from_id(id: &str) -> Option<Vista> {
    match id {
        "cfdi" => Some(Vista::Cfdi),
        "resumen" => Some(Vista::Resumen),
        "clasificacion" => Some(Vista::Clasificacion),
        "errores" => Some(Vista::Errores),
        _ => None,
    }
}
