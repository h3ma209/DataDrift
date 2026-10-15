use crate::grid::{self, GridChrome, GridViewState};
use crate::model::DbVendor;
use crate::theme::JetBrainsDark;
use egui::{RichText, Ui, Vec2};

#[derive(Clone)]
pub struct GridData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone)]
pub enum WorkspaceTab {
    Console {
        sql: String,
        result: GridData,
        grid: GridViewState,
    },
    TableData {
        id: u64,
        ds_id: u64,
        ds_label: String,
        schema: String,
        table: String,
        where_clause: String,
        order_clause: String,
        result: GridData,
        grid: GridViewState,
        vendor: DbVendor,
    },
}

impl WorkspaceTab {
    pub fn title(&self) -> String {
        match self {
            Self::Console { .. } => "console".into(),
            Self::TableData { table, ds_label, .. } => format!("{table} [{ds_label}]"),
        }
    }

    pub fn tab_id(&self) -> u64 {
        match self {
            Self::Console { .. } => 0,
            Self::TableData { id, .. } => *id,
        }
    }
}

pub struct Workspace {
    pub tabs: Vec<WorkspaceTab>,
    pub active: usize,
    next_table_tab_id: u64,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            tabs: vec![WorkspaceTab::Console {
                sql: "SELECT 1;\n".into(),
                result: GridData {
                    columns: vec![],
                    rows: vec![],
                },
                grid: GridViewState::default(),
            }],
            active: 0,
            next_table_tab_id: 1,
        }
    }
}

impl Workspace {
    pub fn active_tab_mut(&mut self) -> &mut WorkspaceTab {
        &mut self.tabs[self.active]
    }

    pub fn open_table(
        &mut self,
        ds_id: u64,
        ds_label: String,
        schema: String,
        table: String,
        vendor: DbVendor,
    ) -> u64 {
        if let Some((i, _)) = self.tabs.iter().enumerate().find_map(|(i, t)| match t {
            WorkspaceTab::TableData {
                ds_id: id,
                schema: s,
                table: tbl,
                id: tid,
                ..
            } if *id == ds_id && *s == schema && *tbl == table => Some((i, *tid)),
            _ => None,
        }) {
            self.active = i;
            return self.tabs[i].tab_id();
        }
        let id = self.next_table_tab_id;
        self.next_table_tab_id += 1;
        self.tabs.push(WorkspaceTab::TableData {
            id,
            ds_id,
            ds_label,
            schema,
            table,
            where_clause: String::new(),
            order_clause: String::new(),
            result: GridData {
                columns: vec![],
                rows: vec![],
            },
            grid: GridViewState::default(),
            vendor,
        });
        self.active = self.tabs.len() - 1;
        id
    }

    pub fn apply_query_result(&mut self, tab_id: u64, columns: Vec<String>, rows: Vec<Vec<String>>) {
        for tab in &mut self.tabs {
            if tab.tab_id() != tab_id {
                continue;
            }
            match tab {
                WorkspaceTab::Console { result, grid, .. } => {
                    result.columns = columns;
                    result.rows = rows;
                    grid.on_loaded(result);
                }
                WorkspaceTab::TableData { result, grid, .. } => {
                    result.columns = columns;
                    result.rows = rows;
                    grid.on_loaded(result);
                }
            }
            return;
        }
    }

    pub fn close_active(&mut self) {
        if self.active == 0 || self.tabs.is_empty() {
            return;
        }
        self.tabs.remove(self.active);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len().saturating_sub(1);
        }
    }
}

pub struct WorkspaceAction {
    pub refresh_table: bool,
    pub execute_sql: Option<String>,
    pub close_tab: bool,
    pub schema_pick: Option<String>,
    pub export_csv: bool,
    pub explain_sql: Option<String>,
    /// DML batch from table editor Submit.
    pub submit_sql: Option<String>,
    pub copy_text: Option<String>,
    /// Dropped SQL from explorer DnD onto console.
    pub drop_sql: Option<String>,
}

impl Default for WorkspaceAction {
    fn default() -> Self {
        Self {
            refresh_table: false,
            execute_sql: None,
            close_tab: false,
            schema_pick: None,
            export_csv: false,
            explain_sql: None,
            submit_sql: None,
            copy_text: None,
            drop_sql: None,
        }
    }
}

pub struct ConsoleChrome<'a> {
    pub ds_label: &'a str,
    pub schemas: &'a [String],
    pub current_schema: &'a mut String,
    pub tx_auto: bool,
    pub supports_explain: bool,
}

pub fn ui(
    workspace: &mut Workspace,
    ui: &mut Ui,
    busy: bool,
    chrome: Option<ConsoleChrome<'_>>,
) -> WorkspaceAction {
    let mut action = WorkspaceAction::default();
    ui.vertical(|ui| {
        tab_bar(ui, workspace, &mut action);
        ui.separator();
        match &mut workspace.tabs[workspace.active] {
            WorkspaceTab::Console { sql, result, grid } => {
                console_view(ui, sql, result, grid, busy, chrome, &mut action);
            }
            WorkspaceTab::TableData {
                schema,
                table,
                where_clause,
                order_clause,
                result,
                grid,
                vendor,
                ..
            } => {
                let schema = schema.clone();
                let table = table.clone();
                let vendor = *vendor;
                table_view(
                    ui,
                    &schema,
                    &table,
                    vendor,
                    where_clause,
                    order_clause,
                    result,
                    grid,
                    busy,
                    &mut action,
                );
            }
        }
    });
    action
}

fn tab_bar(ui: &mut Ui, workspace: &mut Workspace, action: &mut WorkspaceAction) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for i in 0..workspace.tabs.len() {
            let title = workspace.tabs[i].title();
            let active = i == workspace.active;
            let fill = if active {
                JetBrainsDark::TAB_ACTIVE
            } else {
                JetBrainsDark::TAB_INACTIVE
            };
            let tab = ui
                .add(
                    egui::Button::new(RichText::new(&title).size(12.0).color(JetBrainsDark::TEXT))
                        .fill(fill)
                        .stroke(egui::Stroke::new(1.0, JetBrainsDark::BORDER))
                        .rounding(egui::Rounding {
                            nw: 4.0,
                            ne: 4.0,
                            sw: 0.0,
                            se: 0.0,
                        }),
                )
                .clicked();
            if tab {
                workspace.active = i;
            }
            if active && workspace.tabs.len() > 1 && i > 0 {
                if ui
                    .small_button("×")
                    .on_hover_text("Close tab")
                    .clicked()
                {
                    action.close_tab = true;
                }
            }
        }
    });
}

fn console_view(
    ui: &mut Ui,
    sql: &mut String,
    result: &mut GridData,
    grid: &mut GridViewState,
    busy: bool,
    mut chrome: Option<ConsoleChrome<'_>>,
    action: &mut WorkspaceAction,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if ui
            .add_enabled(!busy, egui::Button::new("▶ Execute"))
            .on_hover_text("Execute statement under caret (⌘⏎ / Ctrl+Enter)")
            .clicked()
        {
            action.execute_sql = Some(crate::sql::execute_target(sql, sql.len(), None));
        }
        if chrome.as_ref().map(|c| c.supports_explain).unwrap_or(false)
            && ui
                .add_enabled(!busy, egui::Button::new("Explain"))
                .on_hover_text("EXPLAIN plan (guide §7.1)")
                .clicked()
        {
            let stmt = crate::sql::execute_target(sql, sql.len(), None);
            action.explain_sql = Some(stmt);
        }
        if let Some(ref mut ch) = chrome {
            ui.separator();
            ui.label(RichText::new("Data source").small().color(JetBrainsDark::TEXT_DIM));
            ui.label(RichText::new(ch.ds_label).strong().color(JetBrainsDark::ACCENT));
            ui.label(RichText::new("Schema").small().color(JetBrainsDark::TEXT_DIM));
            let schema_label = if ch.current_schema.is_empty() {
                "(default)".to_string()
            } else {
                ch.current_schema.clone()
            };
            egui::ComboBox::from_id_salt("console_schema")
                .selected_text(schema_label)
                .width(140.0)
                .show_ui(ui, |ui| {
                    for s in ch.schemas {
                        if ui
                            .selectable_label(ch.current_schema == s, s.as_str())
                            .clicked()
                        {
                            action.schema_pick = Some(s.clone());
                        }
                    }
                });
            ui.separator();
            ui.label(
                RichText::new(if ch.tx_auto { "Tx: Auto" } else { "Tx: Manual" })
                    .small()
                    .color(JetBrainsDark::TEXT_DIM),
            );
        }
    });

    // DnD drop zone for explorer table → SELECT
    let (_drop_resp, dropped) = ui.dnd_drop_zone::<String, _>(
        egui::Frame::none()
            .stroke(egui::Stroke::new(1.0, JetBrainsDark::BORDER))
            .inner_margin(egui::Margin::same(4.0)),
        |ui| {
            let edit = egui::TextEdit::multiline(sql)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .desired_rows(10)
                .text_color(JetBrainsDark::TEXT)
                .lock_focus(false);
            ui.add(edit)
        },
    );
    if let Some(payload) = dropped {
        action.drop_sql = Some((*payload).clone());
    }

    let mod_enter = ui.input(|i| {
        (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Enter)
    });
    if mod_enter && !busy {
        let caret = sql.len();
        action.execute_sql = Some(crate::sql::execute_target(sql, caret, None));
    }

    ui.add_space(6.0);
    ui.label(
        RichText::new("Result")
            .small()
            .strong()
            .color(JetBrainsDark::TEXT_DIM),
    );
    let ga = grid::show(ui, result, grid, None, busy);
    if ga.export_csv {
        action.export_csv = true;
    }
    if let Some(t) = ga.copy_cell {
        action.copy_text = Some(t);
    }
    if let Some(w) = ga.filter_where {
        // Append filter as comment / WHERE hint into console
        if w.is_empty() {
            // ignore
        } else {
            sql.push_str(&format!("\n-- filter: {w}\n"));
        }
    }
}

fn table_view(
    ui: &mut Ui,
    schema: &str,
    table: &str,
    vendor: DbVendor,
    where_clause: &mut String,
    order_clause: &mut String,
    result: &mut GridData,
    grid: &mut GridViewState,
    busy: bool,
    action: &mut WorkspaceAction,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if ui
            .add_enabled(!busy, egui::Button::new("↻").min_size(Vec2::new(28.0, 24.0)))
            .on_hover_text("Reload data")
            .clicked()
        {
            action.refresh_table = true;
        }
        ui.label(RichText::new("Tx: Auto").small().color(JetBrainsDark::TEXT_DIM));
        ui.separator();
        ui.label(RichText::new("WHERE").small().color(JetBrainsDark::TEXT_DIM));
        let where_resp = ui.add(
            egui::TextEdit::singleline(where_clause)
                .desired_width(200.0)
                .hint_text("condition — or right-click cell → Filter"),
        );
        ui.label(RichText::new("ORDER BY").small().color(JetBrainsDark::TEXT_DIM));
        let order_resp = ui.add(
            egui::TextEdit::singleline(order_clause)
                .desired_width(140.0)
                .hint_text("column"),
        );
        if (where_resp.lost_focus() || order_resp.lost_focus())
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
            && !busy
        {
            action.refresh_table = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{schema}.{table}"))
                    .small()
                    .color(JetBrainsDark::TEXT_DIM),
            );
        });
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);

    let editable = matches!(
        vendor,
        DbVendor::MySQL
            | DbVendor::MariaDB
            | DbVendor::PostgreSQL
            | DbVendor::Sqlite
            | DbVendor::Redshift
            | DbVendor::ClickHouse
    );
    let chrome = GridChrome {
        editable,
        vendor,
        schema,
        table,
    };
    let ga = grid::show(ui, result, grid, Some(chrome), busy);

    if ga.export_csv {
        action.export_csv = true;
    }
    if let Some(t) = ga.copy_cell {
        action.copy_text = Some(t);
    }
    if let Some(w) = ga.filter_where {
        *where_clause = w;
        action.refresh_table = true;
    }
    if ga.revert {
        if let Some(base) = &grid.baseline {
            *result = base.clone();
            grid.marked_delete.clear();
            grid.new_rows.clear();
            grid.editing = None;
        }
    }
    if ga.submit {
        match grid::build_submit_sql(vendor, schema, table, result, grid) {
            Ok(sql) => action.submit_sql = Some(sql),
            Err(e) => action.copy_text = Some(format!("/* submit: {e} */")),
        }
    }
}

pub fn grid_to_csv(data: &GridData) -> String {
    let mut out = String::new();
    let escape = |s: &str| {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    out.push_str(
        &data
            .columns
            .iter()
            .map(|c| escape(c))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');
    for row in &data.rows {
        out.push_str(
            &row.iter()
                .map(|c| escape(c))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}

pub fn wrap_explain(vendor: DbVendor, sql: &str) -> String {
    let s = sql.trim().trim_end_matches(';');
    match vendor {
        DbVendor::PostgreSQL | DbVendor::Redshift => format!("EXPLAIN {s}"),
        DbVendor::MySQL | DbVendor::MariaDB => format!("EXPLAIN {s}"),
        DbVendor::Sqlite => format!("EXPLAIN QUERY PLAN {s}"),
        DbVendor::ClickHouse => format!("EXPLAIN {s}"),
        _ => s.to_string(),
    }
}

pub fn build_table_sql(vendor: DbVendor, schema: &str, table: &str, limit: u32) -> String {
    build_table_sql_filtered(vendor, schema, table, "", "", limit)
}

pub fn build_table_sql_filtered(
    vendor: DbVendor,
    schema: &str,
    table: &str,
    where_clause: &str,
    order_clause: &str,
    limit: u32,
) -> String {
    let from = match vendor {
        DbVendor::MySQL | DbVendor::MariaDB => format!("`{schema}`.`{table}`"),
        DbVendor::PostgreSQL | DbVendor::Redshift => format!("\"{schema}\".\"{table}\""),
        DbVendor::Sqlite => format!("\"{table}\""),
        DbVendor::ClickHouse => format!("`{schema}`.`{table}`"),
        DbVendor::Cassandra => format!("{schema}.{table}"),
        DbVendor::Redis => {
            let _ = (schema, where_clause, order_clause, limit);
            return format!("GET {table}");
        }
        DbVendor::MongoDB => {
            return format!("db.{table}.find({{}});");
        }
    };
    let mut sql = format!("SELECT * FROM {from}");
    let w = where_clause.trim();
    if !w.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(w);
    }
    let o = order_clause.trim();
    if !o.is_empty() {
        sql.push_str(" ORDER BY ");
        sql.push_str(o);
    }
    sql.push_str(&format!(" LIMIT {limit}"));
    sql
}

/// SQL script templates (guide §5.3 table context menu).
pub fn sql_templates(vendor: DbVendor, schema: &str, table: &str) -> Vec<(&'static str, String)> {
    let q = |s: &str, t: &str| match vendor {
        DbVendor::MySQL | DbVendor::MariaDB | DbVendor::ClickHouse => format!("`{s}`.`{t}`"),
        DbVendor::PostgreSQL | DbVendor::Redshift => format!("\"{s}\".\"{t}\""),
        DbVendor::Sqlite => format!("\"{t}\""),
        _ => format!("{s}.{t}"),
    };
    let ft = q(schema, table);
    vec![
        ("SELECT * LIMIT 200", format!("SELECT * FROM {ft} LIMIT 200;")),
        ("SELECT COUNT(*)", format!("SELECT COUNT(*) FROM {ft};")),
        (
            "INSERT template",
            format!("INSERT INTO {ft} (-- cols\n) VALUES (-- vals\n);"),
        ),
        (
            "UPDATE template",
            format!("UPDATE {ft} SET col = val WHERE pk = ?;"),
        ),
        (
            "DELETE template",
            format!("DELETE FROM {ft} WHERE pk = ?;"),
        ),
    ]
}
