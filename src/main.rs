#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};

use cfdi_zip_processor::ui::theme;

fn main() -> anyhow::Result<()> {
    Application::with_platform(gpui_platform::current_platform(false))
        .with_assets(gpuikit::assets())
        .run(|cx: &mut App| {
            gpuikit::init(cx);
            theme::follow_system_appearance(cx.window_appearance(), cx);
            let bounds = Bounds::centered(
                None,
                size(px(theme::WINDOW_WIDTH), px(theme::WINDOW_HEIGHT)),
                cx,
            );
            let result = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(
                        px(theme::WINDOW_MIN_WIDTH),
                        px(theme::WINDOW_MIN_HEIGHT),
                    )),
                    titlebar: Some(TitlebarOptions {
                        title: Some("CFDI ZIP Processor".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    window.set_window_title("CFDI ZIP Processor");
                    cx.new(|cx| cfdi_zip_processor::ui::AppView::new(cx, window))
                },
            );

            match result {
                Ok(_) => cx.activate(true),
                Err(error) => {
                    eprintln!("Failed to open the application window: {error:#}");
                    std::process::exit(1);
                }
            }
        });

    Ok(())
}
