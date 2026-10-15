//! Audit report browser over `grip::datagrip::reports()`.

use egui::{Color32, RichText, ScrollArea, Sense};
use grip::datagrip::{reports, Report, ReportKind};

use crate::theme::JetBrainsDark;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KindFilter {
    All,
    Overview,
    Inventory,
    Subsystem,
    Synthesis,
    DynamicValidation,
    Guide,
}

impl KindFilter {
    const ALL: &[KindFilter] = &[
        KindFilter::All,
        KindFilter::Overview,
        KindFilter::Inventory,
        KindFilter::Subsystem,
        KindFilter::Synthesis,
        KindFilter::DynamicValidation,
        KindFilter::Guide,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All kinds",
            Self::Overview => "Overview",
            Self::Inventory => "Inventory",
            Self::Subsystem => "Subsystem",
            Self::Synthesis => "Synthesis",
            Self::DynamicValidation => "Dynamic",
            Self::Guide => "Guide",
        }
    }

    fn matches(self, kind: ReportKind) -> bool {
        match self {
            Self::All => true,
            Self::Overview => kind == ReportKind::Overview,
            Self::Inventory => kind == ReportKind::Inventory,
            Self::Subsystem => kind == ReportKind::Subsystem,
            Self::Synthesis => kind == ReportKind::Synthesis,
            Self::DynamicValidation => kind == ReportKind::DynamicValidation,
            Self::Guide => kind == ReportKind::Guide,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SubsystemFilter {
    All,
    S1,
    S2,
    S3,
    S4,
    NoneOnly,
}

impl SubsystemFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All subsystems",
            Self::S1 => "1 — Lifecycle",
            Self::S2 => "2 — JDBC",
            Self::S3 => "3 — Query/Introspect",
            Self::S4 => "4 — UI",
            Self::NoneOnly => "No subsystem",
        }
    }

    fn matches(self, sub: Option<&str>) -> bool {
        match self {
            Self::All => true,
            Self::S1 => sub == Some("1"),
            Self::S2 => sub == Some("2"),
            Self::S3 => sub == Some("3"),
            Self::S4 => sub == Some("4"),
            Self::NoneOnly => sub.is_none(),
        }
    }

    const ALL: &[SubsystemFilter] = &[
        SubsystemFilter::All,
        SubsystemFilter::S1,
        SubsystemFilter::S2,
        SubsystemFilter::S3,
        SubsystemFilter::S4,
        SubsystemFilter::NoneOnly,
    ];
}

pub struct ReportsBrowser {
    search: String,
    kind: KindFilter,
    subsystem: SubsystemFilter,
    selected_id: Option<&'static str>,
    /// Filtered id list rebuilt each frame from catalog.
    filtered_ids: Vec<&'static str>,
}

impl Default for ReportsBrowser {
    fn default() -> Self {
        let mut s = Self {
            search: String::new(),
            kind: KindFilter::All,
            subsystem: SubsystemFilter::All,
            selected_id: reports().first().map(|r| r.id),
            filtered_ids: Vec::new(),
        };
        s.rebuild_filter();
        s
    }
}

impl ReportsBrowser {
    fn rebuild_filter(&mut self) {
        let q = self.search.to_lowercase();
        self.filtered_ids = reports()
            .iter()
            .filter(|r| self.kind.matches(r.kind))
            .filter(|r| self.subsystem.matches(r.subsystem))
            .filter(|r| {
                if q.is_empty() {
                    return true;
                }
                r.id.to_lowercase().contains(&q)
                    || r.title.to_lowercase().contains(&q)
                    || r.file.to_lowercase().contains(&q)
                    || r.markdown.to_lowercase().contains(&q)
            })
            .map(|r| r.id)
            .collect();

        match self.selected_id {
            Some(id) if self.filtered_ids.contains(&id) => {}
            _ => self.selected_id = self.filtered_ids.first().copied(),
        }
    }

    fn selected_report(&self) -> Option<&'static Report> {
        let id = self.selected_id?;
        reports().iter().find(|r| r.id == id)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Search").small().color(JetBrainsDark::TEXT_DIM));
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .desired_width(220.0)
                    .hint_text("id, title, body…"),
            );
            egui::ComboBox::from_id_salt("report_kind")
                .selected_text(self.kind.label())
                .width(120.0)
                .show_ui(ui, |ui| {
                    for k in KindFilter::ALL {
                        ui.selectable_value(&mut self.kind, *k, k.label());
                    }
                });
            egui::ComboBox::from_id_salt("report_sub")
                .selected_text(self.subsystem.label())
                .width(160.0)
                .show_ui(ui, |ui| {
                    for s in SubsystemFilter::ALL {
                        ui.selectable_value(&mut self.subsystem, *s, s.label());
                    }
                });
            ui.label(
                RichText::new(format!(
                    "{}/{}",
                    self.filtered_ids.len(),
                    reports().len()
                ))
                .small()
                .color(JetBrainsDark::TEXT_DIM),
            );
        });

        self.rebuild_filter();
        ui.separator();

        let mut pick: Option<&'static str> = None;
        ui.columns(2, |cols| {
            cols[0].set_min_width(260.0);
            ScrollArea::vertical()
                .id_salt("report_list")
                .auto_shrink([false; 2])
                .show(&mut cols[0], |ui| {
                    if self.filtered_ids.is_empty() {
                        ui.label(
                            RichText::new("No reports match.")
                                .color(JetBrainsDark::TEXT_DIM),
                        );
                        return;
                    }
                    for id in &self.filtered_ids {
                        let Some(r) = reports().iter().find(|r| r.id == *id) else {
                            continue;
                        };
                        let selected = Some(*id) == self.selected_id;
                        let fill = if selected {
                            JetBrainsDark::ACCENT_DIM
                        } else {
                            Color32::TRANSPARENT
                        };
                        let resp = egui::Frame::none()
                            .fill(fill)
                            .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(r.id)
                                            .strong()
                                            .size(12.0)
                                            .color(if selected {
                                                Color32::WHITE
                                            } else {
                                                JetBrainsDark::TEXT
                                            }),
                                    );
                                    ui.label(
                                        RichText::new(r.title)
                                            .small()
                                            .color(JetBrainsDark::TEXT_DIM),
                                    );
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(kind_label(r.kind))
                                                .small()
                                                .color(JetBrainsDark::ACCENT),
                                        );
                                        if let Some(s) = r.subsystem {
                                            ui.label(
                                                RichText::new(format!("sub {s}"))
                                                    .small()
                                                    .color(JetBrainsDark::SUCCESS),
                                            );
                                        }
                                        ui.label(
                                            RichText::new(format!("{} B", r.len()))
                                                .small()
                                                .color(JetBrainsDark::TEXT_DIM),
                                        );
                                    });
                                });
                            })
                            .response
                            .interact(Sense::click());
                        if resp.clicked() {
                            pick = Some(*id);
                        }
                    }
                });

            cols[1].set_min_width(400.0);
            if let Some(r) = self.selected_report() {
                ui_report_body(&mut cols[1], r);
            } else {
                cols[1].centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("Select a report")
                            .color(JetBrainsDark::TEXT_DIM),
                    );
                });
            }
        });

        if let Some(id) = pick {
            self.selected_id = Some(id);
        }
    }
}

fn kind_label(k: ReportKind) -> &'static str {
    match k {
        ReportKind::Overview => "overview",
        ReportKind::Inventory => "inventory",
        ReportKind::Subsystem => "subsystem",
        ReportKind::Synthesis => "synthesis",
        ReportKind::DynamicValidation => "dynamic",
        ReportKind::Guide => "guide",
    }
}

fn ui_report_body(ui: &mut egui::Ui, r: &Report) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(r.title).size(15.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(r.file)
                    .small()
                    .monospace()
                    .color(JetBrainsDark::TEXT_DIM),
            );
        });
    });
    ui.separator();
    ScrollArea::both()
        .id_salt("report_body")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            render_markdown(ui, r.markdown);
        });
}

/// Lightweight markdown → egui (headings, fences, lists, hr; else plain).
fn render_markdown(ui: &mut egui::Ui, md: &str) {
    let mut in_code = false;
    let mut code_buf = String::new();

    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            if in_code {
                egui::Frame::none()
                    .fill(JetBrainsDark::EDITOR)
                    .stroke(egui::Stroke::new(1.0, JetBrainsDark::BORDER))
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(&code_buf)
                                    .monospace()
                                    .size(11.5)
                                    .color(Color32::from_rgb(200, 205, 210)),
                            )
                            .wrap(),
                        );
                    });
                code_buf.clear();
                in_code = false;
            } else {
                in_code = true;
            }
            continue;
        }
        if in_code {
            code_buf.push_str(line);
            code_buf.push('\n');
            continue;
        }

        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            ui.add_space(4.0);
            continue;
        }
        if trimmed == "---" || trimmed == "***" {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("### ") {
            ui.add_space(6.0);
            ui.label(
                RichText::new(strip_inline(rest))
                    .size(13.5)
                    .strong()
                    .color(JetBrainsDark::TEXT),
            );
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            ui.add_space(8.0);
            ui.label(
                RichText::new(strip_inline(rest))
                    .size(15.0)
                    .strong()
                    .color(Color32::from_rgb(220, 222, 226)),
            );
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            ui.add_space(4.0);
            ui.label(
                RichText::new(strip_inline(rest))
                    .size(18.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("- ") {
            ui.horizontal(|ui| {
                ui.label(RichText::new("•").color(JetBrainsDark::ACCENT));
                ui.add(
                    egui::Label::new(
                        RichText::new(strip_inline(rest))
                            .size(12.5)
                            .color(JetBrainsDark::TEXT),
                    )
                    .wrap(),
                );
            });
            continue;
        }

        if trimmed.starts_with('|') {
            ui.add(
                egui::Label::new(
                    RichText::new(trimmed)
                        .monospace()
                        .size(11.0)
                        .color(JetBrainsDark::TEXT_DIM),
                )
                .wrap(),
            );
            continue;
        }

        ui.add(
            egui::Label::new(
                RichText::new(strip_inline(trimmed))
                    .size(12.5)
                    .color(JetBrainsDark::TEXT),
            )
            .wrap(),
        );
    }

    if in_code && !code_buf.is_empty() {
        ui.monospace(&code_buf);
    }
}

/// Strip common inline markers for readable plain fallback.
fn strip_inline(s: &str) -> String {
    let mut out = s.to_string();
    for marker in ["**", "__", "`"] {
        out = out.replace(marker, "");
    }
    // Soften markdown links [text](url) → text
    while let Some(start) = out.find('[') {
        let Some(mid) = out[start..].find("](") else {
            break;
        };
        let mid = start + mid;
        let Some(end_rel) = out[mid + 2..].find(')') else {
            break;
        };
        let end = mid + 2 + end_rel;
        let text = out[start + 1..mid].to_string();
        out.replace_range(start..=end, &text);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_lists_all_reports() {
        let b = ReportsBrowser::default();
        assert_eq!(b.filtered_ids.len(), reports().len());
        assert!(reports().len() >= 16);
        assert!(b.filtered_ids.contains(&"13-ui-validation"));
        assert!(b.filtered_ids.contains(&"14-datagrip-reference-guide"));
        assert_eq!(b.selected_id, Some("00-overview"));
        assert!(b.selected_report().is_some());
    }

    #[test]
    fn kind_filter_subsystem() {
        let mut b = ReportsBrowser::default();
        b.kind = KindFilter::Subsystem;
        b.rebuild_filter();
        assert_eq!(b.filtered_ids.len(), 4);
    }

    #[test]
    fn search_finds_jdbc() {
        let mut b = ReportsBrowser::default();
        b.search = "jdbc".into();
        b.rebuild_filter();
        assert!(!b.filtered_ids.is_empty());
        assert!(b
            .filtered_ids
            .iter()
            .any(|id| id.contains("jdbc") || id.contains("02")));
    }

    #[test]
    fn strip_link() {
        assert_eq!(
            strip_inline("see [foo](./bar.md) ok"),
            "see foo ok"
        );
    }
}
