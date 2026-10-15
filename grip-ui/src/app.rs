use crate::connection::{ConnectionWorker, WorkerRequest, WorkerResponse};
use crate::explorer::{self, ExplorerState};
use crate::jdbc::validation_errors;
use crate::model::{DataSource, DbVendor};
use crate::history::{self, HistoryEntry};
use crate::reports::ReportsBrowser;
use crate::store;
use crate::theme::JetBrainsDark;
use crate::workspace::{self, Workspace, WorkspaceTab};
use egui::{Color32, RichText, ScrollArea};

pub struct GripApp {
    mode: AppMode,
    reports: ReportsBrowser,
    sources: Vec<DataSource>,
    active_source: usize,
    show_datasource_dialog: bool,
    dialog_tab: DialogTab,
    explorer: ExplorerState,
    workspace: Workspace,
    console: Vec<(bool, String)>,
    /// Last executed SQL (Services Output).
    last_sql: String,
    current_schema: String,
    services_tab: ServicesTab,
    busy: bool,
    worker: ConnectionWorker,
    status_cursor: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ServicesTab {
    Output,
    Result,
    History,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppMode {
    Shell,
    Audit,
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
            mode: AppMode::Shell,
            reports: ReportsBrowser::default(),
            sources,
            active_source: 0,
            show_datasource_dialog: false,
            dialog_tab: DialogTab::General,
            explorer: ExplorerState::default(),
            workspace: Workspace::default(),
            console: vec![(
                true,
                "Grip — DB shell follows DataGrip workflow (see Audit → 14-datagrip-reference-guide)."
                    .into(),
            )],
            last_sql: String::new(),
            current_schema: String::new(),
            services_tab: ServicesTab::Output,
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

    fn run_query(&mut self, mut ds: DataSource, sql: String, tab_id: u64) {
        // Apply console schema as default database when set (MySQL USE equivalent).
        if !self.current_schema.is_empty()
            && matches!(
                ds.vendor,
                DbVendor::MySQL
                    | DbVendor::MariaDB
                    | DbVendor::PostgreSQL
                    | DbVendor::Redshift
                    | DbVendor::ClickHouse
                    | DbVendor::Cassandra
                    | DbVendor::MongoDB
                    | DbVendor::Redis
            )
        {
            // Redis: schema tree is `dbN` — strip prefix for index
            if ds.vendor == DbVendor::Redis {
                let idx = self
                    .current_schema
                    .strip_prefix("db")
                    .unwrap_or(&self.current_schema);
                ds.database = idx.to_string();
            } else {
                ds.database = self.current_schema.clone();
            }
        }
        self.last_sql = sql.clone();
        self.busy = true;
        self.log(true, format!("→ {sql}"));
        self.worker.send(WorkerRequest::Query { ds, sql, tab_id });
        self.services_tab = ServicesTab::Output;
    }

    fn schema_names_for_active(&self) -> Vec<String> {
        let id = self.active_ds().id;
        self.explorer
            .schemas
            .get(&id)
            .map(|v| v.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default()
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
                        self.services_tab = ServicesTab::Result;
                        history::push(HistoryEntry {
                            at: chrono_now(),
                            vendor: self.active_ds().vendor.label().into(),
                            source: self.active_ds().tree_label(),
                            sql: self.last_sql.clone(),
                            ok: true,
                            message: message.clone(),
                        });
                    } else {
                        self.log(false, format!("Execute failed: {message}"));
                        self.services_tab = ServicesTab::Output;
                        history::push(HistoryEntry {
                            at: chrono_now(),
                            vendor: self.active_ds().vendor.label().into(),
                            source: self.active_ds().tree_label(),
                            sql: self.last_sql.clone(),
                            ok: false,
                            message: message.clone(),
                        });
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
                    ui.label(RichText::new("Grip").strong().color(JetBrainsDark::ACCENT));
                    ui.separator();
                    ui.selectable_value(&mut self.mode, AppMode::Audit, "Audit reports");
                    ui.selectable_value(&mut self.mode, AppMode::Shell, "DB shell");
                    if self.mode == AppMode::Shell {
                        ui.separator();
                        ui.label(
                            RichText::new(self.active_ds().tree_label())
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
                    } else {
                        ui.separator();
                        ui.label(
                            RichText::new("embedded via grip::datagrip::reports()")
                                .small()
                                .color(JetBrainsDark::TEXT_DIM),
                        );
                    }
                });
            });

        match self.mode {
            AppMode::Audit => self.ui_audit(ctx),
            AppMode::Shell => self.ui_shell(ctx),
        }
    }
}

impl GripApp {
    fn ui_audit(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(JetBrainsDark::BG)
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                ui.heading(
                    RichText::new("DataGrip audit knowledge")
                        .size(16.0)
                        .color(JetBrainsDark::TEXT),
                );
                ui.label(
                    RichText::new(
                        "List / search / filter REA reports. Markdown headings, fences, lists rendered; rest plain text.",
                    )
                    .small()
                    .color(JetBrainsDark::TEXT_DIM),
                );
                ui.add_space(6.0);
                self.reports.ui(ui);
            });
    }

    fn ui_shell(&mut self, ctx: &egui::Context) {
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
            let db = self.sources[i].database.clone();
            if self.current_schema.is_empty() {
                self.current_schema = db;
            }
        }
        if let Some((schema, table)) = explorer_hit.open_table {
            self.open_table_tab(schema, table);
        }
        if let Some((schema, table)) = explorer_hit.console_select {
            self.current_schema = schema.clone();
            self.workspace.active = 0;
            let q = workspace::build_table_sql(self.active_ds().vendor, &schema, &table, 200);
            if let WorkspaceTab::Console { sql, .. } = &mut self.workspace.tabs[0] {
                *sql = format!("{q};\n");
            }
            self.log(true, format!("Console ← {schema}.{table}"));
        }
        if let Some((schema, table)) = explorer_hit.copy_ddl {
            let ddl = format!(
                "-- Grip stub DDL (full introspect TBD)\nCREATE TABLE `{schema}`.`{table}` (/* … */);\n"
            );
            ctx.copy_text(ddl);
            self.log(true, format!("Copied CREATE stub for {schema}.{table}"));
        }
        if let Some(schema) = explorer_hit.set_schema {
            self.current_schema = schema.clone();
            self.log(true, format!("Current schema → {schema}"));
        }

        // Sync default schema from DS when empty after connect
        if self.current_schema.is_empty() {
            self.current_schema = self.active_ds().database.clone();
        }

        let schemas = self.schema_names_for_active();
        let ds_label = self.active_ds().tree_label();
        let supports_explain = self.active_ds().vendor.supports_sql_explain();
        let mut ws_action = workspace::WorkspaceAction::default();
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(JetBrainsDark::EDITOR))
            .show(ctx, |ui| {
                let chrome = if matches!(
                    self.workspace.tabs.get(self.workspace.active),
                    Some(WorkspaceTab::Console { .. })
                ) {
                    Some(workspace::ConsoleChrome {
                        ds_label: &ds_label,
                        schemas: &schemas,
                        current_schema: &mut self.current_schema,
                        tx_auto: true,
                        supports_explain,
                    })
                } else {
                    None
                };
                ws_action = workspace::ui(&mut self.workspace, ui, self.busy, chrome);
            });

        if let Some(schema) = ws_action.schema_pick {
            self.current_schema = schema;
        }
        if let Some(sql) = ws_action.execute_sql {
            let ds = self.active_ds().clone();
            if validation_errors(&ds).is_empty() {
                self.run_query(ds, sql, 0);
                self.services_tab = ServicesTab::Result;
            } else {
                self.log(false, "Fix data source settings first");
            }
        }
        if let Some(sql) = ws_action.explain_sql {
            let ds = self.active_ds().clone();
            let wrapped = workspace::wrap_explain(ds.vendor, &sql);
            if validation_errors(&ds).is_empty() {
                self.run_query(ds, wrapped, 0);
                self.services_tab = ServicesTab::Result;
            }
        }
        if ws_action.export_csv {
            let grid = match &self.workspace.tabs[self.workspace.active] {
                WorkspaceTab::Console { result, .. } | WorkspaceTab::TableData { result, .. } => {
                    result.clone()
                }
            };
            let csv = workspace::grid_to_csv(&grid);
            ctx.copy_text(csv);
            self.log(true, "Copied CSV to clipboard");
        }
        if ws_action.refresh_table {
            if let WorkspaceTab::TableData {
                id,
                ds_id,
                schema,
                table,
                where_clause,
                order_clause,
                ..
            } = &self.workspace.tabs[self.workspace.active]
            {
                if let Some(ds) = self.ds_by_id(*ds_id).cloned() {
                    let sql = workspace::build_table_sql_filtered(
                        ds.vendor,
                        schema,
                        table,
                        where_clause,
                        order_clause,
                        200,
                    );
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

        // Guide §8 — Services: session tree + Output / Result
        egui::TopBottomPanel::bottom("services")
            .default_height(140.0)
            .resizable(true)
            .frame(
                egui::Frame::none()
                    .fill(JetBrainsDark::PANEL)
                    .stroke(egui::Stroke::new(1.0, JetBrainsDark::BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Services").small().strong());
                    ui.separator();
                    ui.selectable_value(&mut self.services_tab, ServicesTab::Output, "Output");
                    ui.selectable_value(&mut self.services_tab, ServicesTab::Result, "Result");
                    ui.selectable_value(&mut self.services_tab, ServicesTab::History, "History");
                    if self.busy {
                        ui.label(
                            RichText::new("Running…")
                                .small()
                                .color(JetBrainsDark::ACCENT),
                        );
                    }
                });
                ui.separator();
                ui.columns(2, |cols| {
                    cols[0].set_min_width(180.0);
                    ScrollArea::vertical()
                        .id_salt("svc_tree")
                        .show(&mut cols[0], |ui| {
                            ui.label(
                                RichText::new(self.active_ds().tree_label())
                                    .small()
                                    .color(JetBrainsDark::ACCENT),
                            );
                            ui.indent("svc_console", |ui| {
                                ui.label(RichText::new("console").small());
                                if !self.last_sql.is_empty() {
                                    ui.label(
                                        RichText::new(truncate_sql(&self.last_sql, 48))
                                            .small()
                                            .monospace()
                                            .color(JetBrainsDark::TEXT_DIM),
                                    );
                                }
                            });
                        });
                    ScrollArea::vertical()
                        .id_salt("svc_body")
                        .show(&mut cols[1], |ui| {
                            match self.services_tab {
                                ServicesTab::Output => {
                                    for (ok, line) in &self.console {
                                        let color = if *ok {
                                            Color32::from_rgb(140, 190, 140)
                                        } else {
                                            Color32::from_rgb(230, 120, 120)
                                        };
                                        ui.label(RichText::new(line).color(color).small());
                                    }
                                }
                                ServicesTab::Result => {
                                    if let WorkspaceTab::Console { result, .. } =
                                        &self.workspace.tabs[0]
                                    {
                                        if result.columns.is_empty() {
                                            ui.label(
                                                RichText::new("No result grid yet — Execute SQL.")
                                                    .small()
                                                    .color(JetBrainsDark::TEXT_DIM),
                                            );
                                        } else {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} cols × {} rows",
                                                    result.columns.len(),
                                                    result.rows.len()
                                                ))
                                                .small()
                                                .color(JetBrainsDark::TEXT_DIM),
                                            );
                                            for c in &result.columns {
                                                ui.label(
                                                    RichText::new(c)
                                                        .small()
                                                        .monospace()
                                                        .color(JetBrainsDark::TEXT),
                                                );
                                            }
                                        }
                                    } else {
                                        ui.label(
                                            RichText::new("Switch to console tab for Result summary.")
                                                .small()
                                                .color(JetBrainsDark::TEXT_DIM),
                                        );
                                    }
                                }
                                ServicesTab::History => {
                                    let entries = history::load();
                                    if entries.is_empty() {
                                        ui.label(
                                            RichText::new(
                                                "No history yet (~/.grip/query_history.json)",
                                            )
                                            .small()
                                            .color(JetBrainsDark::TEXT_DIM),
                                        );
                                    }
                                    let mut restore: Option<String> = None;
                                    for e in entries.iter().take(50) {
                                        let mark = if e.ok { "✓" } else { "✗" };
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{mark} {} · {}",
                                                    e.at, e.vendor
                                                ))
                                                .small()
                                                .color(JetBrainsDark::TEXT_DIM),
                                            );
                                            if ui.small_button("↩").on_hover_text("Restore to console").clicked()
                                            {
                                                restore = Some(e.sql.clone());
                                            }
                                        });
                                        ui.label(
                                            RichText::new(truncate_sql(&e.sql, 80))
                                                .small()
                                                .monospace()
                                                .color(JetBrainsDark::TEXT),
                                        );
                                        ui.separator();
                                    }
                                    if let Some(sql) = restore {
                                        if let WorkspaceTab::Console { sql: buf, .. } =
                                            &mut self.workspace.tabs[0]
                                        {
                                            *buf = format!("{sql}\n");
                                        }
                                        self.workspace.active = 0;
                                        self.log(true, "Restored SQL from history");
                                    }
                                }
                            }
                        });
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
                            let prev = ds.vendor;
                            egui::ComboBox::from_id_salt("vendor")
                                .selected_text(ds.vendor.label())
                                .width(160.0)
                                .show_ui(ui, |ui| {
                                    for v in DbVendor::ALL {
                                        ui.selectable_value(&mut ds.vendor, *v, v.label());
                                    }
                                });
                            if ds.vendor != prev {
                                ds.apply_vendor_defaults();
                            }
                            if !ds.vendor.uses_file_path() {
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
                            ui.label(ds.vendor.database_label());
                            ui.text_edit_singleline(&mut ds.database);
                            if !ds.vendor.uses_file_path() {
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
                            ui.label("DataGrip JDBC class (reference only)");
                            ui.monospace(ds.vendor.driver_class());
                            ui.add_space(6.0);
                            ui.label("Grip native Rust driver");
                            ui.monospace(ds.vendor.rust_driver());
                            ui.add_space(8.0);
                            ui.label(RichText::new("Bundled vendors").strong());
                            for v in DbVendor::ALL {
                                ui.label(format!(
                                    "• {} — port {} — {}",
                                    v.label(),
                                    v.default_port(),
                                    v.rust_driver()
                                ));
                            }
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

fn truncate_sql(s: &str, max: usize) -> String {
    let one = s.replace('\n', " ");
    if one.chars().count() <= max {
        one
    } else {
        let t: String = one.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}
