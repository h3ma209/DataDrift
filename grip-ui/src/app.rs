use crate::connection::{ConnectionWorker, WorkerRequest, WorkerResponse};
use crate::explorer::{self, ExplorerState};
use crate::jdbc::validation_errors;
use crate::model::{DataSource, DbVendor};
use crate::store;
use crate::theme::JetBrainsDark;
use crate::workspace::{self, Workspace, WorkspaceTab};
use egui::{Color32, RichText, ScrollArea};

pub struct GripApp {
    sources: Vec<DataSource>,
    active_source: usize,
    show_datasource_dialog: bool,
    dialog_tab: DialogTab,
    explorer: ExplorerState,
    workspace: Workspace,
    console: Vec<(bool, String)>,
    busy: bool,
    worker: ConnectionWorker,
    status_cursor: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DialogTab {
    General,
    Url,
    Driver,
}

impl GripApp {
    pub fn new() -> Self {
        let mut sources = store::load();
        if sources.is_empty() {
            sources.push(DataSource::new(1));
        }
        for s in &mut sources {
            s.sync_jdbc_url();
        }
        Self {
            sources,
            active_source: 0,
            show_datasource_dialog: false,
            dialog_tab: DialogTab::General,
            explorer: ExplorerState::default(),
            workspace: Workspace::default(),
            console: vec![(
                true,
                "Grip — connect in toolbar, browse tables in Database Explorer.".into(),
            )],
            busy: false,
            worker: ConnectionWorker::spawn(),
            status_cursor: "1:1".into(),
        }
    }

    fn active_ds(&self) -> &DataSource {
        &self.sources[self.active_source]
    }

    fn ds_by_id(&self, id: u64) -> Option<&DataSource> {
        self.sources.iter().find(|s| s.id == id)
    }

    fn log(&mut self, ok: bool, msg: impl Into<String>) {
        self.console.push((ok, msg.into()));
    }

    fn run_query(&mut self, ds: DataSource, sql: String, tab_id: u64) {
        self.busy = true;
        self.worker.send(WorkerRequest::Query { ds, sql, tab_id });
    }

    fn poll_worker(&mut self) {
        while let Some(resp) = self.worker.try_recv() {
            self.busy = false;
            match resp {
                WorkerResponse::TestResult {
                    source_id,
                    ok,
                    message,
                    elapsed_ms,
                } => {
                    if ok {
                        self.log(
                            true,
                            format!("Test OK ({elapsed_ms} ms): {message}"),
                        );
                        if !self.explorer.is_connected(source_id) {
                            if let Some(ds) = self.sources.iter().find(|s| s.id == source_id) {
                                self.busy = true;
                                self.worker.send(WorkerRequest::Connect(ds.clone()));
                                self.log(true, "Connecting…");
                            }
                        }
                    } else {
                        self.log(false, format!("Test failed: {message}"));
                    }
                }
                WorkerResponse::Connected {
                    source_id,
                    ok,
                    message,
                    schemas,
                    schema_total,
                } => {
                    if ok {
                        self.explorer
                            .set_metadata(source_id, schemas, schema_total);
                        if let Some(ds) = self.ds_by_id(source_id) {
                            self.explorer
                                .expanded_tables_folder
                                .insert((source_id, ds.database.clone()));
                        }
                        self.log(true, message);
                    } else {
                        self.explorer.disconnect(source_id);
                        self.log(false, message);
                    }
                }
                WorkerResponse::QueryResult {
                    tab_id,
                    ok,
                    message,
                    columns,
                    rows,
                } => {
                    if ok {
                        self.workspace
                            .apply_query_result(tab_id, columns, rows);
                        self.log(true, format!("Execute: {message}"));
                    } else {
                        self.log(false, format!("Execute failed: {message}"));
                    }
                }
            }
        }
    }

    fn status_breadcrumb(&self) -> String {
        if let Some((ds_id, schema, table)) = &self.explorer.selected_table {
            let ds = self
                .ds_by_id(*ds_id)
                .map(|d| d.tree_label())
                .unwrap_or_else(|| "?".into());
            return format!("Database > {ds} > {schema} > tables > {table}");
        }
        match &self.workspace.tabs[self.workspace.active] {
            WorkspaceTab::TableData {
                ds_label,
                schema,
                table,
                ..
            } => format!("Database > {ds_label} > {schema} > tables > {table}"),
            WorkspaceTab::Console { .. } => {
                format!("Database > {} > console", self.active_ds().tree_label())
            }
        }
    }

    fn update_status_cursor(&mut self) {
        if let WorkspaceTab::TableData { selected, .. } = &self.workspace.tabs[self.workspace.active]
        {
            if let Some((r, c)) = selected {
                self.status_cursor = format!("{}:{}", r + 1, c + 1);
                return;
            }
        }
        self.status_cursor = "1:1".into();
    }

    fn open_table_tab(&mut self, schema: String, table: String) {
        let ds = self.active_ds().clone();
        let tab_id = self.workspace.open_table(
            ds.id,
            ds.tree_label(),
            schema.clone(),
            table.clone(),
        );
        let sql = workspace::build_table_sql(ds.vendor, &schema, &table, 200);
        self.explorer.selected_table = Some((ds.id, schema, table));
        self.run_query(ds, sql, tab_id);
        self.log(true, "Loading table data…");
    }
}

impl eframe::App for GripApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_worker();
        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::none()
                    .fill(JetBrainsDark::PANEL)
                    .stroke(egui::Stroke::new(1.0, JetBrainsDark::BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("DG").strong().color(JetBrainsDark::ACCENT));
                    ui.label(
                        RichText::new("hakki-backend")
                            .size(13.0)
                            .color(JetBrainsDark::TEXT),
                    );
                    ui.separator();
                    if ui.button("+ Data Source").clicked() {
                        let id = self.sources.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                        self.sources.push(DataSource::new(id));
                        self.active_source = self.sources.len() - 1;
                        self.show_datasource_dialog = true;
                    }
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("Test Connection"))
                        .clicked()
                    {
                        let ds = self.active_ds().clone();
                        self.busy = true;
                        self.worker.send(WorkerRequest::Test(ds));
                    }
                    if ui.add_enabled(!self.busy, egui::Button::new("Connect")).clicked() {
                        let ds = self.active_ds().clone();
                        self.busy = true;
                        self.worker.send(WorkerRequest::Connect(ds));
                    }
                });
            });

        let mut explorer_actions = explorer::ExplorerActions::default();
        let mut explorer_hit = explorer::ExplorerHit::default();
        egui::SidePanel::left("explorer")
            .default_width(260.0)
            .resizable(true)
            .frame(
                egui::Frame::none()
                    .fill(JetBrainsDark::PANEL)
                    .stroke(egui::Stroke::new(1.0, JetBrainsDark::BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new("Database Explorer")
                            .size(14.0)
                            .color(JetBrainsDark::TEXT),
                    );
                });
                ui.separator();
                explorer_actions = explorer::toolbar(ui, self.busy);
                ui.separator();
                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        explorer_hit = explorer::tree(
                            ui,
                            &mut self.explorer,
                            &self.sources,
                            self.active_source,
                        );
                    });
            });

        if explorer_actions.add_source {
            let id = self.sources.iter().map(|s| s.id).max().unwrap_or(0) + 1;
            self.sources.push(DataSource::new(id));
            self.active_source = self.sources.len() - 1;
            self.show_datasource_dialog = true;
        }
        if explorer_actions.properties {
            self.show_datasource_dialog = true;
        }
        if explorer_actions.refresh && !self.busy {
            let ds = self.active_ds().clone();
            self.busy = true;
            self.worker.send(WorkerRequest::Connect(ds));
        }
        if explorer_actions.disconnect {
            self.explorer.disconnect(self.active_ds().id);
        }
        if explorer_actions.collapse_all {
            self.explorer.collapse_all();
        }
        if explorer_actions.expand_all {
            self.explorer.expand_all(&self.sources);
        }
        if explorer_actions.new_console {
            self.workspace.active = 0;
        }
        if let Some(i) = explorer_hit.select_source {
            self.active_source = i;
        }
        if let Some((schema, table)) = explorer_hit.open_table {
            self.open_table_tab(schema, table);
        }

        let mut ws_action = workspace::WorkspaceAction::default();
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(JetBrainsDark::EDITOR))
            .show(ctx, |ui| {
                ws_action = workspace::ui(&mut self.workspace, ui, self.busy);
            });

        if ws_action.run_console {
            let ds = self.active_ds().clone();
            if validation_errors(&ds).is_empty() {
                let sql = match &self.workspace.tabs[self.workspace.active] {
                    WorkspaceTab::Console { sql, .. } => sql.clone(),
                    _ => String::new(),
                };
                self.run_query(ds, sql, 0);
            }
        }
        if ws_action.refresh_table {
            if let WorkspaceTab::TableData {
                id,
                ds_id,
                schema,
                table,
                ..
            } = &self.workspace.tabs[self.workspace.active]
            {
                if let Some(ds) = self.ds_by_id(*ds_id).cloned() {
                    let sql = workspace::build_table_sql(ds.vendor, schema, table, 200);
                    self.run_query(ds, sql, *id);
                }
            }
        }
        if ws_action.close_tab {
            self.workspace.close_active();
        }

        self.update_status_cursor();

        egui::TopBottomPanel::bottom("status")
            .exact_height(22.0)
            .frame(
                egui::Frame::none()
                    .fill(JetBrainsDark::PANEL)
                    .stroke(egui::Stroke::new(1.0, JetBrainsDark::BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(self.status_breadcrumb())
                            .small()
                            .color(JetBrainsDark::TEXT_DIM),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("Non-commercial use")
                                .small()
                                .color(JetBrainsDark::SUCCESS),
                        );
                        ui.label(
                            RichText::new(&self.status_cursor)
                                .small()
                                .color(JetBrainsDark::TEXT_DIM),
                        );
                    });
                });
            });

        egui::TopBottomPanel::bottom("services")
            .default_height(100.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label(RichText::new("Services").small().strong());
                ScrollArea::vertical().show(ui, |ui| {
                    for (ok, line) in &self.console {
                        let color = if *ok {
                            Color32::from_rgb(140, 190, 140)
                        } else {
                            Color32::from_rgb(230, 120, 120)
                        };
                        ui.label(RichText::new(line).color(color).small());
                    }
                });
            });

        if self.show_datasource_dialog {
            self.data_source_dialog(ctx);
        }
    }
}

impl GripApp {
    fn data_source_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Data Sources and Drivers")
            .collapsible(false)
            .resizable(true)
            .default_size([520.0, 420.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.dialog_tab, DialogTab::General, "General");
                    ui.selectable_value(&mut self.dialog_tab, DialogTab::Url, "URL");
                    ui.selectable_value(&mut self.dialog_tab, DialogTab::Driver, "Driver");
                });
                ui.separator();

                let tab = self.dialog_tab;
                let idx = self.active_source;
                {
                    let ds = &mut self.sources[idx];
                    match tab {
                        DialogTab::General => {
                            ui.label("Name");
                            ui.text_edit_singleline(&mut ds.name);
                            ui.add_space(4.0);
                            ui.label("Connection type");
                            egui::ComboBox::from_id_salt("vendor")
                                .selected_text(ds.vendor.label())
                                .show_ui(ui, |ui| {
                                    for v in
                                        [DbVendor::PostgreSQL, DbVendor::MySQL, DbVendor::Sqlite]
                                    {
                                        ui.selectable_value(&mut ds.vendor, v, v.label());
                                    }
                                });
                            if ds.vendor != DbVendor::Sqlite {
                                ui.horizontal(|ui| {
                                    ui.label("Host");
                                    ui.text_edit_singleline(&mut ds.host);
                                    ui.label("Port");
                                    ui.add(
                                        egui::DragValue::new(&mut ds.port)
                                            .range(1..=65535)
                                            .speed(1),
                                    );
                                });
                            }
                            ui.label(if ds.vendor == DbVendor::Sqlite {
                                "Database file (or :memory:)"
                            } else {
                                "Database"
                            });
                            ui.text_edit_singleline(&mut ds.database);
                            if ds.vendor != DbVendor::Sqlite {
                                ui.label("User");
                                ui.text_edit_singleline(&mut ds.user);
                                ui.horizontal(|ui| {
                                    ui.label("Password");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut ds.password).password(true),
                                    );
                                });
                                ui.checkbox(
                                    &mut ds.remember_password,
                                    "Save password locally (~/.grip/)",
                                );
                            }
                        }
                        DialogTab::Url => {
                            ui.checkbox(&mut ds.use_url, "Use URL directly");
                            ui.label("JDBC URL");
                            ui.text_edit_singleline(&mut ds.jdbc_url);
                            if !ds.use_url {
                                ds.sync_jdbc_url();
                                ui.label(RichText::new("Generated:").weak());
                                ui.monospace(&ds.jdbc_url);
                            }
                        }
                        DialogTab::Driver => {
                            ui.label("Driver class");
                            ui.monospace(ds.vendor.driver_class());
                        }
                    }
                }

                let errors = validation_errors(&self.sources[idx]);
                for e in &errors {
                    ui.colored_label(Color32::LIGHT_RED, e);
                }

                ui.add_space(8.0);
                let can_test = errors.is_empty() && !self.busy;
                let mut apply = false;
                let mut test = false;
                let mut ok = false;
                let mut cancel = false;
                ui.horizontal(|ui| {
                    apply = ui.button("Apply").clicked();
                    test = ui
                        .add_enabled(can_test, egui::Button::new("Test Connection"))
                        .clicked();
                    ok = ui.button("OK").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
                if apply {
                    self.sources[idx].sync_jdbc_url();
                    if let Err(e) = store::save(&self.sources) {
                        self.log(false, e);
                    }
                }
                if test {
                    self.sources[idx].sync_jdbc_url();
                    let d = self.sources[idx].clone();
                    self.busy = true;
                    self.worker.send(WorkerRequest::Test(d));
                }
                if ok {
                    self.sources[idx].sync_jdbc_url();
                    let _ = store::save(&self.sources);
                    self.show_datasource_dialog = false;
                }
                if cancel {
                    self.show_datasource_dialog = false;
                }
            });
    }
}

pub fn datagrip_visuals(ctx: &egui::Context) {
    crate::theme::apply(ctx);
}
