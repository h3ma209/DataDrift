use crate::model::DbVendor;
use crate::text_ar;
use crate::theme::JetBrainsDark;
use egui::{Color32, RichText, ScrollArea, Ui, Vec2};

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
        selected: Option<(usize, usize)>,
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
            selected: None,
        });
        self.active = self.tabs.len() - 1;
        id
    }

    pub fn apply_query_result(&mut self, tab_id: u64, columns: Vec<String>, rows: Vec<Vec<String>>) {
        for tab in &mut self.tabs {
            let match_id = tab.tab_id();
            if match_id != tab_id {
                continue;
            }
            match tab {
                WorkspaceTab::Console { result, .. } | WorkspaceTab::TableData { result, .. } => {
                    result.columns = columns;
                    result.rows = rows;
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
    /// SQL to execute (statement under caret / selection), if any.
    pub execute_sql: Option<String>,
    pub close_tab: bool,
    pub schema_pick: Option<String>,
    /// Guide §7 — export result grid as CSV to clipboard.
    pub export_csv: bool,
    /// Guide §7.1 — EXPLAIN plan for current statement.
    pub explain_sql: Option<String>,
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
        }
    }
}

/// Editor header context (guide §6.4).
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
            WorkspaceTab::Console { sql, result } => {
                console_view(ui, sql, result, busy, chrome, &mut action);
            }
            WorkspaceTab::TableData {
                schema,
                table,
                where_clause,
                order_clause,
                result,
                selected,
                ..
            } => {
                let schema = schema.clone();
                let table = table.clone();
                table_view(
                    ui,
                    &schema,
                    &table,
                    where_clause,
                    order_clause,
                    result,
                    selected,
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
    result: &GridData,
    busy: bool,
    mut chrome: Option<ConsoleChrome<'_>>,
    action: &mut WorkspaceAction,
) {
    // Guide §6.4 — data source / schema / Tx widgets
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
        if !result.columns.is_empty()
            && ui
                .button("Export CSV")
                .on_hover_text("Copy result grid as CSV (guide §7 / §10)")
                .clicked()
        {
            action.export_csv = true;
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

    let edit = egui::TextEdit::multiline(sql)
        .font(egui::TextStyle::Monospace)
        .desired_width(f32::INFINITY)
        .desired_rows(10)
        .text_color(JetBrainsDark::TEXT)
        .lock_focus(false);
    let resp = ui.add(edit);

    // Ctrl/Cmd+Enter → statement at end of selection or caret approx (end of buffer if unknown)
    let mod_enter = ui.input(|i| {
        (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Enter)
    });
    if mod_enter && !busy && resp.has_focus() {
        let caret = sql.len();
        action.execute_sql = Some(crate::sql::execute_target(sql, caret, None));
    }

    ui.separator();
    ui.label(
        RichText::new("Result")
            .small()
            .strong()
            .color(JetBrainsDark::TEXT_DIM),
    );
    data_grid(ui, result, &mut None, true);
}

fn table_view(
    ui: &mut Ui,
    schema: &str,
    table: &str,
    where_clause: &mut String,
    order_clause: &mut String,
    result: &GridData,
    selected: &mut Option<(usize, usize)>,
    busy: bool,
    action: &mut WorkspaceAction,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if ui
            .add_enabled(!busy, egui::Button::new("↻").min_size(Vec2::new(26.0, 22.0)))
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
                .desired_width(180.0)
                .hint_text("condition"),
        );
        ui.label(RichText::new("ORDER BY").small().color(JetBrainsDark::TEXT_DIM));
        let order_resp = ui.add(
            egui::TextEdit::singleline(order_clause)
                .desired_width(120.0)
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
    ui.separator();
    data_grid(ui, result, selected, false);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} rows", result.rows.len()))
                .small()
                .color(JetBrainsDark::TEXT_DIM),
        );
        if !result.columns.is_empty()
            && ui
                .small_button("Export CSV")
                .on_hover_text("Copy as CSV")
                .clicked()
        {
            action.export_csv = true;
        }
    });
}

/// TSV/CSV for clipboard export (DataGrip Export → Clipboard).
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

const CELL_PAD_X: f32 = 10.0;
const CELL_PAD_Y: f32 = 4.0;
const ROW_NUM_W: f32 = 44.0;
const MIN_COL_W: f32 = 72.0;
const MAX_COL_W: f32 = 280.0;
const CHAR_W: f32 = 7.5;

fn data_grid(
    ui: &mut Ui,
    data: &GridData,
    selected: &mut Option<(usize, usize)>,
    console_mode: bool,
) {
    if data.columns.is_empty() {
        ui.label(
            RichText::new("No results")
                .small()
                .color(JetBrainsDark::TEXT_DIM),
        );
        return;
    }

    let col_widths = column_widths(data);

    ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
        egui::Grid::new("data_grid")
            .striped(true)
            .spacing([1.0, 1.0])
            .min_col_width(MIN_COL_W)
            .max_col_width(MAX_COL_W)
            .show(ui, |ui| {
                // Header row
                grid_cell(
                    ui,
                    ROW_NUM_W,
                    JetBrainsDark::HEADER,
                    RichText::new("#")
                        .size(11.0)
                        .strong()
                        .color(JetBrainsDark::TEXT_DIM),
                    false,
                );
                for (ci, c) in data.columns.iter().enumerate() {
                    let head = if !console_mode && c.eq_ignore_ascii_case("id") {
                        format!("🔑 {c}")
                    } else {
                        c.clone()
                    };
                    grid_cell(
                        ui,
                        col_widths[ci],
                        JetBrainsDark::HEADER,
                        RichText::new(text_ar::for_display(&head))
                            .size(12.0)
                            .strong()
                            .color(JetBrainsDark::TEXT),
                        false,
                    );
                }
                ui.end_row();

                for (ri, row) in data.rows.iter().enumerate() {
                    let row_bg = if ri % 2 == 0 {
                        JetBrainsDark::EDITOR
                    } else {
                        JetBrainsDark::ROW_ALT
                    };
                    grid_cell(
                        ui,
                        ROW_NUM_W,
                        row_bg,
                        RichText::new((ri + 1).to_string())
                            .size(11.0)
                            .color(JetBrainsDark::TEXT_DIM),
                        false,
                    );
                    for (ci, cell) in row.iter().enumerate() {
                        let is_sel = selected
                            .map(|(r, c)| r == ri && c == ci)
                            .unwrap_or(false);
                        let bg = if is_sel {
                            JetBrainsDark::ACCENT
                        } else {
                            row_bg
                        };
                        let color = if is_sel {
                            Color32::WHITE
                        } else {
                            JetBrainsDark::TEXT
                        };
                        let display = text_ar::for_display(&truncate_display(cell, 48));
                        let w = col_widths.get(ci).copied().unwrap_or(MIN_COL_W);
                        let resp = grid_cell(
                            ui,
                            w,
                            bg,
                            RichText::new(display).size(12.5).color(color),
                            true,
                        );
                        if !cell.is_empty() && cell != "NULL" {
                            resp.clone().on_hover_text(text_ar::for_display(cell));
                        }
                        if resp.clicked() {
                            *selected = Some((ri, ci));
                        }
                    }
                    ui.end_row();
                }
            });
    });
}

fn grid_cell(
    ui: &mut Ui,
    width: f32,
    fill: Color32,
    text: RichText,
    clickable: bool,
) -> egui::Response {
    let sense = if clickable {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, JetBrainsDark::BORDER))
        .inner_margin(egui::Margin::symmetric(CELL_PAD_X, CELL_PAD_Y))
        .show(ui, |ui| {
            ui.set_min_width(width - CELL_PAD_X * 2.0);
            ui.set_max_width(width - CELL_PAD_X * 2.0);
            ui.add(egui::Label::new(text).truncate());
        })
        .response
        .interact(sense)
}

fn column_widths(data: &GridData) -> Vec<f32> {
    let mut widths: Vec<f32> = data
        .columns
        .iter()
        .map(|c| ((c.chars().count() as f32) * CHAR_W + CELL_PAD_X * 2.0).max(MIN_COL_W))
        .collect();
    for row in data.rows.iter().take(80) {
        for (i, cell) in row.iter().enumerate() {
            if i >= widths.len() {
                break;
            }
            let visible = truncate_display(cell, 48);
            let w = (visible.chars().count() as f32) * CHAR_W + CELL_PAD_X * 2.0;
            widths[i] = widths[i].max(w.min(MAX_COL_W)).max(MIN_COL_W);
        }
    }
    widths
}

fn truncate_display(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}


pub fn build_table_sql(vendor: DbVendor, schema: &str, table: &str, limit: u32) -> String {
    build_table_sql_filtered(vendor, schema, table, "", "", limit)
}

/// DataGrip-style data editor query: `SELECT * FROM … [WHERE …] [ORDER BY …] LIMIT n`.
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
            // Console uses find syntax, not SQL SELECT
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
