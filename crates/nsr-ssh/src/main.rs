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

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("NSR-SSH")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "NSR-SSH",
        options,
        Box::new(|cc| Ok(Box::new(nsr_ui::NsrApp::new(cc)))),
    )
}
