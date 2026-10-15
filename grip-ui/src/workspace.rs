use crate::model::DbVendor;
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
    pub run_console: bool,
    pub close_tab: bool,
}

impl Default for WorkspaceAction {
    fn default() -> Self {
        Self {
            refresh_table: false,
            run_console: false,
            close_tab: false,
        }
    }
}

pub fn ui(workspace: &mut Workspace, ui: &mut Ui, busy: bool) -> WorkspaceAction {
    let mut action = WorkspaceAction::default();
    ui.vertical(|ui| {
        tab_bar(ui, workspace, &mut action);
        ui.separator();
        match &mut workspace.tabs[workspace.active] {
            WorkspaceTab::Console { sql, result } => {
                console_view(ui, sql, result, busy, &mut action);
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
    action: &mut WorkspaceAction,
) {
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!busy, egui::Button::new("▶ Execute"))
            .clicked()
        {
            action.run_console = true;
        }
    });
    ui.add(
        egui::TextEdit::multiline(sql)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .desired_rows(8)
            .text_color(JetBrainsDark::TEXT),
    );
    ui.separator();
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
        ui.add(
            egui::TextEdit::singleline(where_clause)
                .desired_width(180.0)
                .hint_text("condition"),
        );
        ui.label(RichText::new("ORDER BY").small().color(JetBrainsDark::TEXT_DIM));
        ui.add(
            egui::TextEdit::singleline(order_clause)
                .desired_width(120.0)
                .hint_text("column"),
        );
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
    });
}

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
    ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
        egui::Grid::new("data_grid")
            .striped(false)
            .spacing([0.0, 0.0])
            .show(ui, |ui| {
                ui.label(row_header_cell("#"));
                for c in &data.columns {
                    let head = if !console_mode && c == "id" {
                        format!("🔑 {c}")
                    } else {
                        c.clone()
                    };
                    ui.label(header_cell(&head));
                }
                ui.end_row();

                for (ri, row) in data.rows.iter().enumerate() {
                    ui.label(row_num_cell(ri + 1));
                    for (ci, cell) in row.iter().enumerate() {
                        let is_sel = selected
                            .map(|(r, c)| r == ri && c == ci)
                            .unwrap_or(false);
                        let resp = ui.add(
                            egui::Label::new(body_cell(cell, is_sel))
                                .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            *selected = Some((ri, ci));
                        }
                    }
                    ui.end_row();
                }
            });
    });
}

fn header_cell(text: &str) -> RichText {
    RichText::new(text)
        .strong()
        .size(12.0)
        .color(JetBrainsDark::TEXT)
        .background_color(JetBrainsDark::HEADER)
}

fn row_header_cell(text: &str) -> RichText {
    RichText::new(text)
        .size(11.0)
        .color(JetBrainsDark::TEXT_DIM)
        .background_color(JetBrainsDark::HEADER)
}

fn row_num_cell(n: usize) -> RichText {
    RichText::new(n.to_string())
        .size(11.0)
        .color(JetBrainsDark::TEXT_DIM)
        .background_color(JetBrainsDark::ROW_ALT)
}

fn body_cell(text: &str, selected: bool) -> RichText {
    let bg = if selected {
        JetBrainsDark::ACCENT
    } else {
        Color32::TRANSPARENT
    };
    RichText::new(text)
        .size(12.0)
        .color(if selected {
            Color32::WHITE
        } else {
            JetBrainsDark::TEXT
        })
        .background_color(bg)
}

pub fn build_table_sql(vendor: DbVendor, schema: &str, table: &str, limit: u32) -> String {
    match vendor {
        DbVendor::MySQL => format!("SELECT * FROM `{schema}`.`{table}` LIMIT {limit}"),
        DbVendor::PostgreSQL => format!("SELECT * FROM \"{schema}\".\"{table}\" LIMIT {limit}"),
        DbVendor::Sqlite => format!("SELECT * FROM \"{table}\" LIMIT {limit}"),
    }
}
