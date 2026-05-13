use eframe::egui;
use eframe::NativeOptions;
use egui::ViewportBuilder;
use tracing_subscriber::EnvFilter;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("nsr=debug".parse().unwrap()),
        )
        .init();

    let icon = load_icon();

    let mut viewport = ViewportBuilder::default()
        .with_title("NSR-SSH")
        .with_app_id("nsr-ssh")
        .with_decorations(false)
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([800.0, 500.0]);

    if let Some(icon_data) = icon {
        viewport = viewport.with_icon(std::sync::Arc::new(icon_data));
    }

    let options = NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "NSR-SSH",
        options,
        Box::new(|cc| Ok(Box::new(nsr_ui::NsrApp::new(cc)))),
    )
}

fn load_icon() -> Option<egui::IconData> {
    // PNG embutido em compile-time — sem dep extra em runtime
    let bytes = include_bytes!("../../../assets/icons/nsr-ssh-128.png");
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width: w,
        height: h,
    })
}
