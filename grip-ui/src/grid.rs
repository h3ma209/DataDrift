//! DataGrip-style result / table data grid (guide §7 / DasGrid).
//! In-place edit, insert/delete rows, column resize + reorder, sort, cell menus.

use crate::model::DbVendor;
use crate::text_ar;
use crate::theme::JetBrainsDark;
use crate::workspace::GridData;
use egui::{Color32, CursorIcon, Id, PointerButton, RichText, ScrollArea, Sense, Ui, Vec2};
use std::collections::HashSet;

pub const CELL_PAD_X: f32 = 12.0;
pub const CELL_PAD_Y: f32 = 6.0;
pub const ROW_NUM_W: f32 = 48.0;
pub const MIN_COL_W: f32 = 80.0;
pub const MAX_COL_W: f32 = 420.0;
pub const CHAR_W: f32 = 7.8;
pub const ROW_H: f32 = 28.0;

#[derive(Clone, Default)]
pub struct GridViewState {
    pub selected: Option<(usize, usize)>,
    pub editing: Option<(usize, usize)>,
    pub edit_buf: String,
    /// Logical column order (indices into `columns`).
    pub col_order: Vec<usize>,
    pub col_widths: Vec<f32>,
    /// (logical_col, ascending)
    pub sort: Option<(usize, bool)>,
    /// Row indices marked for delete (soft).
    pub marked_delete: HashSet<usize>,
    /// Rows appended locally (INSERT on submit).
    pub new_rows: HashSet<usize>,
    /// Snapshot after last successful load — dirty detect.
    pub baseline: Option<GridData>,
    pub drag_col: Option<usize>,
}

impl GridViewState {
    pub fn sync_columns(&mut self, data: &GridData) {
        let n = data.columns.len();
        if self.col_order.len() != n || self.col_order.iter().any(|&i| i >= n) {
            self.col_order = (0..n).collect();
        }
        if self.col_widths.len() != n {
            self.col_widths = default_widths(data);
        }
    }

    pub fn on_loaded(&mut self, data: &GridData) {
        self.baseline = Some(data.clone());
        self.editing = None;
        self.edit_buf.clear();
        self.marked_delete.clear();
        self.new_rows.clear();
        self.selected = None;
        self.col_order = (0..data.columns.len()).collect();
        self.col_widths = default_widths(data);
        self.sort = None;
        self.drag_col = None;
    }

    pub fn is_dirty(&self, data: &GridData) -> bool {
        if !self.marked_delete.is_empty() || !self.new_rows.is_empty() {
            return true;
        }
        match &self.baseline {
            Some(b) => b.rows != data.rows || b.columns != data.columns,
            None => false,
        }
    }

    pub fn dirty_cell(&self, data: &GridData, ri: usize, ci: usize) -> bool {
        if self.new_rows.contains(&ri) || self.marked_delete.contains(&ri) {
            return true;
        }
        let Some(base) = &self.baseline else {
            return false;
        };
        match (base.rows.get(ri), data.rows.get(ri)) {
            (Some(br), Some(dr)) => br.get(ci) != dr.get(ci),
            _ => false,
        }
    }
}

#[derive(Default)]
pub struct GridAction {
    pub filter_where: Option<String>,
    pub copy_cell: Option<String>,
    pub submit: bool,
    pub revert: bool,
    pub export_csv: bool,
}

pub struct GridChrome<'a> {
    pub editable: bool,
    #[allow(dead_code)]
    pub vendor: DbVendor,
    pub schema: &'a str,
    pub table: &'a str,
}

pub fn show(
    ui: &mut Ui,
    data: &mut GridData,
    state: &mut GridViewState,
    chrome: Option<GridChrome<'_>>,
    busy: bool,
) -> GridAction {
    let mut action = GridAction::default();
    state.sync_columns(data);
    let editable = chrome.as_ref().map(|c| c.editable).unwrap_or(false);

    // Toolbar
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if editable {
            if ui
                .add_enabled(!busy, egui::Button::new("+ Row"))
                .on_hover_text("Insert empty row (Submit to INSERT)")
                .clicked()
            {
                let n = data.columns.len();
                data.rows.push(vec![String::new(); n]);
                let idx = data.rows.len() - 1;
                state.new_rows.insert(idx);
                state.selected = Some((idx, 0));
                state.editing = Some((idx, 0));
                state.edit_buf.clear();
            }
            if ui
                .add_enabled(
                    !busy && state.selected.is_some(),
                    egui::Button::new("− Row"),
                )
                .on_hover_text("Mark row for DELETE on Submit")
                .clicked()
            {
                if let Some((ri, _)) = state.selected {
                    if state.new_rows.remove(&ri) {
                        data.rows.remove(ri);
                        // remap indices
                        remap_after_remove(state, ri);
                    } else {
                        state.marked_delete.insert(ri);
                    }
                }
            }
            let dirty = state.is_dirty(data);
            if ui
                .add_enabled(!busy && dirty, egui::Button::new("✓ Submit"))
                .on_hover_text("Apply INSERT/UPDATE/DELETE (guide §7)")
                .clicked()
            {
                action.submit = true;
            }
            if ui
                .add_enabled(dirty, egui::Button::new("Revert"))
                .on_hover_text("Discard local edits")
                .clicked()
            {
                action.revert = true;
            }
            ui.separator();
        }
        if !data.columns.is_empty()
            && ui
                .button("Export CSV")
                .on_hover_text("Copy grid as CSV")
                .clicked()
        {
            action.export_csv = true;
        }
        let dirty_n = if state.is_dirty(data) { " • dirty" } else { "" };
        let title = chrome
            .as_ref()
            .map(|c| format!("{}.{}", c.schema, c.table))
            .unwrap_or_default();
        ui.label(
            RichText::new(format!(
                "{title}  {} cols × {} rows{dirty_n}",
                data.columns.len(),
                data.rows.len()
            ))
            .small()
            .color(if state.is_dirty(data) {
                Color32::from_rgb(230, 180, 80)
            } else {
                JetBrainsDark::TEXT_DIM
            }),
        );
    });
    ui.add_space(4.0);

    if data.columns.is_empty() {
        ui.label(
            RichText::new("No results")
                .small()
                .color(JetBrainsDark::TEXT_DIM),
        );
        return action;
    }

    // Sort (only when clean — keeps baseline indices aligned)
    if let Some((sci, asc)) = state.sort.take() {
        if sci < data.columns.len()
            && state.new_rows.is_empty()
            && state.marked_delete.is_empty()
            && !state.is_dirty(data)
        {
            let mut idxs: Vec<usize> = (0..data.rows.len()).collect();
            idxs.sort_by(|&a, &b| {
                let va = data.rows[a].get(sci).map(|s| s.as_str()).unwrap_or("");
                let vb = data.rows[b].get(sci).map(|s| s.as_str()).unwrap_or("");
                let c = va.cmp(vb);
                if asc {
                    c
                } else {
                    c.reverse()
                }
            });
            let old_rows = std::mem::take(&mut data.rows);
            for &i in &idxs {
                data.rows.push(old_rows[i].clone());
            }
            if let Some(base) = &mut state.baseline {
                if base.rows.len() == idxs.len() {
                    let old_b = std::mem::take(&mut base.rows);
                    for &i in &idxs {
                        base.rows.push(old_b[i].clone());
                    }
                }
            }
        }
    }

    let col_order = state.col_order.clone();
    let n_cols = col_order.len();
    let mut header_drag_target: Option<usize> = None;
    let mut resize_col: Option<(usize, f32)> = None;
    let mut start_edit: Option<(usize, usize)> = None;
    let mut commit_edit = false;
    let mut cancel_edit = false;
    let mut reorder: Option<(usize, usize)> = None;

    let max_h = ui.available_height().max(160.0);
    ScrollArea::both()
        .auto_shrink([false; 2])
        .max_height(max_h)
        .id_salt("grip_data_grid")
        .show(ui, |ui| {
            // Header
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                header_cell(ui, ROW_NUM_W, "#", false, None);
                for (pos, &ci) in col_order.iter().enumerate() {
                    let name = data.columns.get(ci).map(|s| s.as_str()).unwrap_or("?");
                    let w = state.col_widths.get(ci).copied().unwrap_or(MIN_COL_W);
                    let head = if name.eq_ignore_ascii_case("id") {
                        format!("🔑 {name}")
                    } else {
                        name.to_string()
                    };
                    let id = Id::new(("col_hdr", ci));
                    let resp = header_cell(ui, w, &head, true, Some(id));

                    // Click → sort
                    if resp.clicked() && state.editing.is_none() {
                        state.sort = Some((ci, true));
                        if ui.input(|i| i.modifiers.shift) {
                            state.sort = Some((ci, false));
                        }
                    }

                    // Context on header
                    resp.clone().context_menu(|ui| {
                        if ui.button("Sort ascending").clicked() {
                            state.sort = Some((ci, true));
                            ui.close_menu();
                        }
                        if ui.button("Sort descending").clicked() {
                            state.sort = Some((ci, false));
                            ui.close_menu();
                        }
                        if ui.button("Fit width").clicked() {
                            let fit = fit_width(data, ci);
                            if let Some(slot) = state.col_widths.get_mut(ci) {
                                *slot = fit;
                            }
                            ui.close_menu();
                        }
                    });

                    // Drag reorder
                    if resp.dragged_by(PointerButton::Primary) {
                        state.drag_col = Some(pos);
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    }
                    if resp.hovered() && state.drag_col.is_some() && state.drag_col != Some(pos) {
                        header_drag_target = Some(pos);
                    }
                    if resp.drag_stopped() {
                        if let (Some(from), Some(to)) = (state.drag_col, header_drag_target) {
                            if from != to {
                                reorder = Some((from, to));
                            }
                        }
                        state.drag_col = None;
                    }

                    // Resize handle on right edge
                    let r = resp.rect;
                    let handle = egui::Rect::from_min_size(
                        egui::pos2(r.right() - 4.0, r.top()),
                        Vec2::new(8.0, r.height()),
                    );
                    let hresp = ui.interact(handle, Id::new(("resize", ci)), Sense::drag());
                    if hresp.hovered() || hresp.dragged() {
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeColumn);
                    }
                    if hresp.dragged() {
                        resize_col = Some((ci, hresp.drag_delta().x));
                    }
                }
            });

            // Body rows
            for ri in 0..data.rows.len() {
                let deleted = state.marked_delete.contains(&ri);
                let is_new = state.new_rows.contains(&ri);
                let row_bg = if deleted {
                    Color32::from_rgb(60, 35, 35)
                } else if is_new {
                    Color32::from_rgb(35, 55, 40)
                } else if ri % 2 == 0 {
                    JetBrainsDark::EDITOR
                } else {
                    JetBrainsDark::ROW_ALT
                };

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let num_label = if deleted {
                        format!("−{}", ri + 1)
                    } else if is_new {
                        format!("+{}", ri + 1)
                    } else {
                        (ri + 1).to_string()
                    };
                    body_cell(
                        ui,
                        ROW_NUM_W,
                        row_bg,
                        RichText::new(num_label)
                            .size(11.0)
                            .color(JetBrainsDark::TEXT_DIM),
                        false,
                    );

                    for &ci in &col_order {
                        let w = state.col_widths.get(ci).copied().unwrap_or(MIN_COL_W);
                        let cell_val = data
                            .rows
                            .get(ri)
                            .and_then(|r| r.get(ci))
                            .cloned()
                            .unwrap_or_default();
                        let is_sel = state.selected == Some((ri, ci));
                        let is_edit = state.editing == Some((ri, ci));
                        let dirty = state.dirty_cell(data, ri, ci);

                        if is_edit {
                            let te = ui.add_sized(
                                [w, ROW_H],
                                egui::TextEdit::singleline(&mut state.edit_buf)
                                    .font(egui::TextStyle::Monospace)
                                    .text_color(Color32::WHITE)
                                    .frame(true),
                            );
                            te.request_focus();
                            if te.lost_focus() {
                                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                    cancel_edit = true;
                                } else {
                                    commit_edit = true;
                                }
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                commit_edit = true;
                            }
                        } else {
                            let bg = if is_sel {
                                JetBrainsDark::ACCENT
                            } else if dirty {
                                Color32::from_rgb(55, 50, 30)
                            } else {
                                row_bg
                            };
                            let color = if is_sel {
                                Color32::WHITE
                            } else if cell_val == "NULL" || cell_val.is_empty() {
                                JetBrainsDark::TEXT_DIM
                            } else {
                                JetBrainsDark::TEXT
                            };
                            let display = text_ar::for_display(&truncate_display(&cell_val, 64));
                            let resp = body_cell(
                                ui,
                                w,
                                bg,
                                RichText::new(display).size(12.5).color(color),
                                true,
                            );
                            if !cell_val.is_empty() {
                                resp.clone().on_hover_text(text_ar::for_display(&cell_val));
                            }
                            if resp.clicked() {
                                state.selected = Some((ri, ci));
                            }
                            if resp.double_clicked() && editable && !deleted {
                                start_edit = Some((ri, ci));
                            }
                            // F2 when selected
                            if is_sel
                                && editable
                                && !deleted
                                && ui.input(|i| i.key_pressed(egui::Key::F2))
                            {
                                start_edit = Some((ri, ci));
                            }

                            let col_name = data.columns.get(ci).cloned().unwrap_or_default();
                            resp.context_menu(|ui| {
                                if ui.button("Copy").clicked() {
                                    action.copy_cell = Some(cell_val.clone());
                                    ui.close_menu();
                                }
                                if editable && ui.button("Edit").clicked() {
                                    start_edit = Some((ri, ci));
                                    ui.close_menu();
                                }
                                if editable && ui.button("Set NULL").clicked() {
                                    if let Some(row) = data.rows.get_mut(ri) {
                                        if let Some(c) = row.get_mut(ci) {
                                            *c = "NULL".into();
                                        }
                                    }
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui
                                    .button(format!("Filter… WHERE `{col_name}` = …"))
                                    .clicked()
                                {
                                    let lit = sql_literal(&cell_val);
                                    action.filter_where =
                                        Some(format!("`{col_name}` = {lit}"));
                                    ui.close_menu();
                                }
                                if ui.button("Clear filter").clicked() {
                                    action.filter_where = Some(String::new());
                                    ui.close_menu();
                                }
                            });
                        }
                    }
                });
            }
        });

    if let Some((from, to)) = reorder {
        let col = state.col_order.remove(from);
        let insert_at = if from < to { to - 1 } else { to };
        let insert_at = insert_at.min(n_cols.saturating_sub(1));
        state.col_order.insert(insert_at, col);
    }
    if let Some((ci, dx)) = resize_col {
        if let Some(w) = state.col_widths.get_mut(ci) {
            *w = (*w + dx).clamp(MIN_COL_W, MAX_COL_W);
        }
    }
    if let Some((ri, ci)) = start_edit {
        state.editing = Some((ri, ci));
        state.selected = Some((ri, ci));
        state.edit_buf = data
            .rows
            .get(ri)
            .and_then(|r| r.get(ci))
            .cloned()
            .unwrap_or_default();
        if state.edit_buf == "NULL" {
            state.edit_buf.clear();
        }
    }
    if commit_edit {
        if let Some((ri, ci)) = state.editing {
            if let Some(row) = data.rows.get_mut(ri) {
                while row.len() <= ci {
                    row.push(String::new());
                }
                row[ci] = state.edit_buf.clone();
            }
        }
        state.editing = None;
    }
    if cancel_edit {
        state.editing = None;
        state.edit_buf.clear();
    }

    action
}

pub fn build_submit_sql(
    vendor: DbVendor,
    schema: &str,
    table: &str,
    data: &GridData,
    state: &GridViewState,
) -> Result<String, String> {
    let base = state
        .baseline
        .as_ref()
        .ok_or_else(|| "No baseline — reload table first".to_string())?;
    let pk = pick_pk(&data.columns);
    let from = qualify(vendor, schema, table);
    let mut stmts = Vec::new();

    // DELETEs
    for &ri in &state.marked_delete {
        if state.new_rows.contains(&ri) {
            continue;
        }
        let row = base.rows.get(ri).ok_or("delete row missing")?;
        let where_sql = where_pk(vendor, &data.columns, row, pk)?;
        stmts.push(format!("DELETE FROM {from} WHERE {where_sql};"));
    }

    // UPDATEs
    for (ri, row) in data.rows.iter().enumerate() {
        if state.marked_delete.contains(&ri) || state.new_rows.contains(&ri) {
            continue;
        }
        let Some(old) = base.rows.get(ri) else {
            continue;
        };
        if old == row {
            continue;
        }
        let mut sets = Vec::new();
        for (ci, col) in data.columns.iter().enumerate() {
            let nv = row.get(ci).map(|s| s.as_str()).unwrap_or("");
            let ov = old.get(ci).map(|s| s.as_str()).unwrap_or("");
            if nv != ov {
                sets.push(format!(
                    "{} = {}",
                    ident(vendor, col),
                    sql_literal_vendor(vendor, nv)
                ));
            }
        }
        if sets.is_empty() {
            continue;
        }
        let where_sql = where_pk(vendor, &data.columns, old, pk)?;
        stmts.push(format!(
            "UPDATE {from} SET {} WHERE {where_sql};",
            sets.join(", ")
        ));
    }

    // INSERTs
    for &ri in &state.new_rows {
        if state.marked_delete.contains(&ri) {
            continue;
        }
        let row = data.rows.get(ri).ok_or("insert row missing")?;
        let cols: Vec<_> = data
            .columns
            .iter()
            .map(|c| ident(vendor, c))
            .collect();
        let vals: Vec<_> = (0..data.columns.len())
            .map(|ci| {
                let v = row.get(ci).map(|s| s.as_str()).unwrap_or("");
                sql_literal_vendor(vendor, v)
            })
            .collect();
        stmts.push(format!(
            "INSERT INTO {from} ({}) VALUES ({});",
            cols.join(", "),
            vals.join(", ")
        ));
    }

    if stmts.is_empty() {
        return Err("Nothing to submit".into());
    }
    Ok(stmts.join("\n"))
}

fn remap_after_remove(state: &mut GridViewState, removed: usize) {
    let remap = |set: &mut HashSet<usize>| {
        let old = std::mem::take(set);
        for i in old {
            if i < removed {
                set.insert(i);
            } else if i > removed {
                set.insert(i - 1);
            }
        }
    };
    remap(&mut state.new_rows);
    remap(&mut state.marked_delete);
    if let Some((ri, ci)) = state.selected {
        if ri == removed {
            state.selected = None;
        } else if ri > removed {
            state.selected = Some((ri - 1, ci));
        }
    }
}

fn pick_pk(cols: &[String]) -> usize {
    cols.iter()
        .position(|c| {
            let l = c.to_lowercase();
            l == "id" || l == "_id" || l.ends_with("_id") && l != "uuid"
        })
        .unwrap_or(0)
}

fn where_pk(
    vendor: DbVendor,
    cols: &[String],
    row: &[String],
    pk: usize,
) -> Result<String, String> {
    let col = cols.get(pk).ok_or("no pk column")?;
    let val = row.get(pk).map(|s| s.as_str()).unwrap_or("NULL");
    Ok(format!(
        "{} = {}",
        ident(vendor, col),
        sql_literal_vendor(vendor, val)
    ))
}

fn qualify(vendor: DbVendor, schema: &str, table: &str) -> String {
    match vendor {
        DbVendor::MySQL | DbVendor::MariaDB | DbVendor::ClickHouse => {
            format!("`{schema}`.`{table}`")
        }
        DbVendor::PostgreSQL | DbVendor::Redshift => format!("\"{schema}\".\"{table}\""),
        DbVendor::Sqlite => format!("\"{table}\""),
        DbVendor::Cassandra => format!("{schema}.{table}"),
        _ => format!("{schema}.{table}"),
    }
}

fn ident(vendor: DbVendor, name: &str) -> String {
    match vendor {
        DbVendor::MySQL | DbVendor::MariaDB | DbVendor::ClickHouse => format!("`{name}`"),
        DbVendor::PostgreSQL | DbVendor::Redshift | DbVendor::Sqlite => format!("\"{name}\""),
        _ => name.to_string(),
    }
}

fn sql_literal(v: &str) -> String {
    sql_literal_vendor(DbVendor::MySQL, v)
}

fn sql_literal_vendor(vendor: DbVendor, v: &str) -> String {
    let _ = vendor;
    if v.is_empty() || v.eq_ignore_ascii_case("NULL") {
        return "NULL".into();
    }
    if v.parse::<i64>().is_ok() || v.parse::<f64>().is_ok() {
        return v.to_string();
    }
    format!("'{}'", v.replace('\'', "''"))
}

fn default_widths(data: &GridData) -> Vec<f32> {
    (0..data.columns.len()).map(|ci| fit_width(data, ci)).collect()
}

fn fit_width(data: &GridData, ci: usize) -> f32 {
    let mut w = data
        .columns
        .get(ci)
        .map(|c| c.chars().count() as f32 * CHAR_W + CELL_PAD_X * 2.0 + 16.0)
        .unwrap_or(MIN_COL_W);
    for row in data.rows.iter().take(40) {
        if let Some(cell) = row.get(ci) {
            let cw = cell.chars().count().min(40) as f32 * CHAR_W + CELL_PAD_X * 2.0;
            w = w.max(cw);
        }
    }
    w.clamp(MIN_COL_W, MAX_COL_W)
}

fn truncate_display(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

fn header_cell(ui: &mut Ui, width: f32, text: &str, interactive: bool, id: Option<Id>) -> egui::Response {
    let sense = if interactive {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let frame = egui::Frame::none()
        .fill(JetBrainsDark::HEADER)
        .stroke(egui::Stroke::new(1.0, JetBrainsDark::BORDER))
        .inner_margin(egui::Margin::symmetric(CELL_PAD_X, CELL_PAD_Y));
    let resp = frame
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(width - CELL_PAD_X * 2.0, ROW_H - CELL_PAD_Y * 2.0));
            ui.set_max_width(width - CELL_PAD_X * 2.0);
            ui.label(
                RichText::new(text_ar::for_display(text))
                    .size(12.0)
                    .strong()
                    .color(JetBrainsDark::TEXT),
            );
        })
        .response;
    if let Some(id) = id {
        ui.interact(resp.rect, id, sense)
    } else {
        resp.interact(sense)
    }
}

fn body_cell(
    ui: &mut Ui,
    width: f32,
    fill: Color32,
    text: RichText,
    clickable: bool,
) -> egui::Response {
    let sense = if clickable {
        Sense::click()
    } else {
        Sense::hover()
    };
    egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, JetBrainsDark::BORDER))
        .inner_margin(egui::Margin::symmetric(CELL_PAD_X, CELL_PAD_Y))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(width - CELL_PAD_X * 2.0, ROW_H - CELL_PAD_Y * 2.0));
            ui.set_max_width(width - CELL_PAD_X * 2.0);
            ui.add(egui::Label::new(text).truncate());
        })
        .response
        .interact(sense)
}
