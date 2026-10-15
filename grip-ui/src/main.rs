mod app;
mod async_rt;
mod connection;
mod drivers;
mod explorer;
mod history;
mod jdbc;
mod model;
mod mysql_format;
mod reports;
mod sql;
mod store;
mod text_ar;
mod theme;
mod workspace;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_title("Grip — DataGrip audit"),
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
