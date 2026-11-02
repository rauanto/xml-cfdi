use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};

fn main() -> anyhow::Result<()> {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("CFDI ZIP Processor".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title("CFDI ZIP Processor");
                cx.new(|_| cfdi_zip_processor::ui::AppView)
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
