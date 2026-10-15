mod app;
mod connection;
mod explorer;
mod jdbc;
mod model;
mod mysql_format;
mod store;
mod theme;
mod workspace;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_title("DataGrip — Grip"),
        ..Default::default()
    };
    eframe::run_native(
        "grip-ui",
        native_options,
        Box::new(|cc| {
            app::datagrip_visuals(&cc.egui_ctx);
            Ok(Box::new(app::GripApp::new()))
        }),
    )
}
