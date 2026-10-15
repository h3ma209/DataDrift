use crate::model::{DataSource, DbVendor, SchemaNode};
use crate::theme::JetBrainsDark;
use egui::{Color32, RichText, Ui};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct ExplorerState {
    pub expanded_ds: HashSet<u64>,
    pub expanded_schema: HashSet<(u64, String)>,
    pub expanded_tables_folder: HashSet<(u64, String)>,
    pub connected: HashSet<u64>,
    pub schemas: HashMap<u64, Vec<SchemaNode>>,
    pub schema_totals: HashMap<u64, usize>,
    pub selected_table: Option<(u64, String, String)>,
}

impl ExplorerState {
    pub fn set_metadata(&mut self, ds_id: u64, schemas: Vec<SchemaNode>, schema_total: usize) {
        self.connected.insert(ds_id);
        self.schemas.insert(ds_id, schemas);
        self.schema_totals.insert(ds_id, schema_total);
        self.expanded_ds.insert(ds_id);
    }

    pub fn disconnect(&mut self, ds_id: u64) {
        self.connected.remove(&ds_id);
        self.schemas.remove(&ds_id);
        self.schema_totals.remove(&ds_id);
        self.expanded_schema.retain(|(id, _)| *id != ds_id);
        self.expanded_tables_folder.retain(|(id, _)| *id != ds_id);
        self.selected_table = None;
    }

    pub fn collapse_all(&mut self) {
        self.expanded_ds.clear();
        self.expanded_schema.clear();
        self.expanded_tables_folder.clear();
    }

    pub fn expand_all(&mut self, sources: &[DataSource]) {
        for ds in sources {
            if !self.connected.contains(&ds.id) {
                continue;
            }
            self.expanded_ds.insert(ds.id);
            if let Some(list) = self.schemas.get(&ds.id) {
                for s in list {
                    self.expanded_schema.insert((ds.id, s.name.clone()));
                    self.expanded_tables_folder.insert((ds.id, s.name.clone()));
                }
            }
        }
    }

    pub fn is_connected(&self, ds_id: u64) -> bool {
        self.connected.contains(&ds_id)
    }
}

pub fn vendor_color(v: DbVendor) -> Color32 {
    match v {
        DbVendor::MySQL | DbVendor::MariaDB => Color32::from_rgb(0, 120, 215),
        DbVendor::PostgreSQL | DbVendor::Redshift => Color32::from_rgb(59, 115, 175),
        DbVendor::Sqlite => Color32::from_rgb(45, 160, 120),
        DbVendor::ClickHouse => Color32::from_rgb(255, 204, 0),
        DbVendor::Cassandra => Color32::from_rgb(60, 140, 120),
        DbVendor::MongoDB => Color32::from_rgb(0, 150, 80),
        DbVendor::Redis => Color32::from_rgb(200, 60, 50),
    }
}

pub fn vendor_icon(v: DbVendor) -> &'static str {
    match v {
        DbVendor::MySQL | DbVendor::MariaDB => "🐬",
        DbVendor::PostgreSQL | DbVendor::Redshift => "🐘",
        DbVendor::Sqlite => "📄",
        DbVendor::ClickHouse => "▮",
        DbVendor::Cassandra => "◉",
        DbVendor::MongoDB => "🍃",
        DbVendor::Redis => "◆",
    }
}

pub struct ExplorerActions {
    pub add_source: bool,
    pub properties: bool,
    pub refresh: bool,
    pub disconnect: bool,
    pub new_console: bool,
    pub collapse_all: bool,
    pub expand_all: bool,
}

impl Default for ExplorerActions {
    fn default() -> Self {
        Self {
            add_source: false,
            properties: false,
            refresh: false,
            disconnect: false,
            new_console: false,
            collapse_all: false,
            expand_all: false,
        }
    }
}

pub struct ExplorerHit {
    pub select_source: Option<usize>,
    pub open_table: Option<(String, String)>,
    /// Right-click: open console with SELECT for table.
    pub console_select: Option<(String, String)>,
    pub copy_ddl: Option<(String, String)>,
    pub set_schema: Option<String>,
}

impl Default for ExplorerHit {
    fn default() -> Self {
        Self {
            select_source: None,
            open_table: None,
            console_select: None,
            copy_ddl: None,
            set_schema: None,
        }
    }
}

pub fn toolbar(ui: &mut Ui, busy: bool) -> ExplorerActions {
    let mut a = ExplorerActions::default();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        flat_btn(ui, busy, "+", "Add data source", || a.add_source = true);
        flat_btn(ui, busy, "⚙", "Data Source Properties", || a.properties = true);
        flat_btn(ui, busy, "↻", "Refresh", || a.refresh = true);
        flat_btn(ui, busy, "⏏", "Disconnect", || a.disconnect = true);
        flat_btn(ui, busy, "▶", "Console", || a.new_console = true);
        ui.separator();
        if ui.small_button("▾").on_hover_text("Expand all").clicked() {
            a.expand_all = true;
        }
        if ui.small_button("▸").on_hover_text("Collapse all").clicked() {
            a.collapse_all = true;
        }
    });
    a
}

fn flat_btn(ui: &mut Ui, busy: bool, label: &str, tip: &str, mut mark: impl FnMut()) {
    if ui
        .add_enabled(
            !busy,
            egui::Button::new(RichText::new(label).size(13.0))
                .fill(Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE),
        )
        .on_hover_text(tip)
        .clicked()
    {
        mark();
    }
}

pub fn tree(
    ui: &mut Ui,
    state: &mut ExplorerState,
    sources: &[DataSource],
    active_source: usize,
) -> ExplorerHit {
    let mut hit = ExplorerHit::default();
    for (i, ds) in sources.iter().enumerate() {
        let ds_id = ds.id;
        let selected = i == active_source;
        let connected = state.is_connected(ds_id);
        let mut ds_open = state.expanded_ds.contains(&ds_id);

        ui.horizontal(|ui| {
            toggle(ui, &mut ds_open, |open| {
                if open {
                    state.expanded_ds.insert(ds_id);
                } else {
                    state.expanded_ds.remove(&ds_id);
                }
            });
            if connected {
                ui.label(RichText::new("⚡").size(11.0).color(JetBrainsDark::ACCENT));
            }
            ui.label(RichText::new(vendor_icon(ds.vendor)).size(13.0));
            let label = ds.tree_label();
            let fill = if selected {
                Color32::from_rgb(55, 58, 63)
            } else {
                Color32::TRANSPARENT
            };
            let resp = egui::Frame::none()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                .rounding(3.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(&label)
                            .size(13.0)
                            .color(vendor_color(ds.vendor)),
                    );
                })
                .response
                .interact(egui::Sense::click());
            if resp.clicked() {
                hit.select_source = Some(i);
                if connected {
                    state.expanded_ds.insert(ds_id);
                    ds_open = true;
                }
            }
        });

        if !ds_open {
            continue;
        }

        if !connected {
            ui.indent("disc", |ui| {
                ui.label(
                    RichText::new("Not connected — ↻ or Connect")
                        .small()
                        .color(JetBrainsDark::TEXT_DIM),
                );
            });
            continue;
        }

        let schemas = state.schemas.get(&ds_id).cloned().unwrap_or_default();
        let default_db = ds.database.clone();
        let (primary, other): (Vec<_>, Vec<_>) = schemas
            .into_iter()
            .partition(|s| s.name == default_db);

        ui.indent(format!("ds-{ds_id}"), |ui| {
            for schema in primary {
                render_tables_folder(ui, state, ds_id, &schema, &mut hit);
            }
            for schema in other {
                render_schema_node(ui, state, ds_id, &schema, &mut hit);
            }
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(
                    RichText::new("📁 Server Objects")
                        .small()
                        .color(JetBrainsDark::TEXT_DIM),
                );
            });
        });
    }
    hit
}

fn render_schema_node(
    ui: &mut Ui,
    state: &mut ExplorerState,
    ds_id: u64,
    schema: &SchemaNode,
    hit: &mut ExplorerHit,
) {
    let key = (ds_id, schema.name.clone());
    let mut sch_open = state.expanded_schema.contains(&key);
    ui.horizontal(|ui| {
        toggle(ui, &mut sch_open, |open| {
            if open {
                state.expanded_schema.insert(key.clone());
            } else {
                state.expanded_schema.remove(&key);
            }
        });
        ui.label(RichText::new("⬡").color(JetBrainsDark::TEXT));
        ui.label(RichText::new(&schema.name).size(13.0));
    });
    if sch_open {
        ui.indent(format!("sch-{ds_id}-{}", schema.name), |ui| {
            render_tables_folder(ui, state, ds_id, schema, hit);
        });
    }
}

fn render_tables_folder(
    ui: &mut Ui,
    state: &mut ExplorerState,
    ds_id: u64,
    schema: &SchemaNode,
    hit: &mut ExplorerHit,
) {
    let folder_key = (ds_id, schema.name.clone());
    let mut open = state.expanded_tables_folder.contains(&folder_key);
    let count = schema.children.len();
    ui.horizontal(|ui| {
        toggle(ui, &mut open, |is_open| {
            if is_open {
                state.expanded_tables_folder.insert(folder_key.clone());
            } else {
                state.expanded_tables_folder.remove(&folder_key);
            }
        });
        ui.label(RichText::new("📁 tables").size(13.0).color(JetBrainsDark::TEXT));
        ui.label(
            RichText::new(count.to_string())
                .small()
                .color(JetBrainsDark::TEXT_DIM),
        );
    });
    if !open {
        return;
    }
    ui.indent(format!("tbl-{ds_id}-{}", schema.name), |ui| {
        for table in &schema.children {
            if table.trim().is_empty() {
                continue;
            }
            let is_sel = state.selected_table.as_ref()
                == Some(&(ds_id, schema.name.clone(), table.clone()));
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("▦")
                        .size(12.0)
                        .color(Color32::from_rgb(130, 170, 130)),
                );
                let fill = if is_sel {
                    Color32::from_rgb(55, 58, 63)
                } else {
                    Color32::TRANSPARENT
                };
                let text_color = if is_sel {
                    Color32::WHITE
                } else {
                    JetBrainsDark::TEXT
                };
                let resp = egui::Frame::none()
                    .fill(fill)
                    .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                    .rounding(2.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(table).size(12.0).color(text_color));
                    })
                    .response
                    .interact(egui::Sense::click());
                if resp.clicked() {
                    state.selected_table = Some((ds_id, schema.name.clone(), table.clone()));
                    hit.open_table = Some((schema.name.clone(), table.clone()));
                }
                resp.context_menu(|ui| {
                    if ui.button("Open Data Editor").clicked() {
                        hit.open_table = Some((schema.name.clone(), table.clone()));
                        ui.close_menu();
                    }
                    if ui.button("Jump to Query Console").clicked() {
                        hit.console_select = Some((schema.name.clone(), table.clone()));
                        ui.close_menu();
                    }
                    if ui.button("SQL Scripts → SELECT … LIMIT 200").clicked() {
                        hit.console_select = Some((schema.name.clone(), table.clone()));
                        ui.close_menu();
                    }
                    if ui.button("Copy DDL (CREATE stub)").clicked() {
                        hit.copy_ddl = Some((schema.name.clone(), table.clone()));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Set as current schema").clicked() {
                        hit.set_schema = Some(schema.name.clone());
                        ui.close_menu();
                    }
                });
            });
        }
    });
}

fn toggle(ui: &mut Ui, open: &mut bool, mut apply: impl FnMut(bool)) {
    let arrow = if *open { "▾" } else { "▸" };
    if ui
        .add(
            egui::Button::new(RichText::new(arrow).size(11.0).color(JetBrainsDark::TEXT_DIM))
                .fill(Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE)
                .min_size(egui::vec2(14.0, 14.0)),
        )
        .clicked()
    {
        *open = !*open;
        apply(*open);
    }
}
