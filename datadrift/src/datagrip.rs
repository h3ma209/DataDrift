//! Embedded DataGrip architecture-audit reports (REA + static/dynamic validation).
//!
//! Source assets: repo-root `reports/*.md`. Helpers: `scripts/validate-dynamic.sh`.
//! Large class indexes: `tmp/jar-index*.txt` (not embedded; see [`paths`]).

/// Repo-relative paths for non-embedded audit helpers (resolve from workspace root).
pub mod paths {
    /// Full `Contents/lib/` JAR class index (~18 MB). Regenerated locally; gitignored.
    pub const JAR_INDEX: &str = "tmp/jar-index.txt";
    /// DatabaseTools module JAR class index (~1.7 MB).
    pub const JAR_INDEX_DATABASE_MODULES: &str = "tmp/jar-index-database-modules.txt";
    /// Phase-2 dynamic validation helper script.
    pub const VALIDATE_DYNAMIC_SH: &str = "scripts/validate-dynamic.sh";
    /// Optional Phase 2b javap / Vineflower scratch under `tmp/phase2b/`.
    pub const PHASE2B_DIR: &str = "tmp/phase2b";
    /// Markdown report directory (repo root).
    pub const REPORTS_DIR: &str = "reports";
}

/// Kind of audit document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportKind {
    Overview,
    Inventory,
    /// Architecture subsystem deep-dive (IDs 1–4).
    Subsystem,
    Synthesis,
    DynamicValidation,
    /// Product walkthrough / UX map (not REA bytecode).
    Guide,
}

/// One markdown report: metadata + embedded body.
#[derive(Debug, Clone, Copy)]
pub struct Report {
    /// Stable slug, usually the file stem (`00-overview`, `02-jdbc-connections`, …).
    pub id: &'static str,
    /// Filename under [`paths::REPORTS_DIR`].
    pub file: &'static str,
    /// Human title (from report H1 / catalog).
    pub title: &'static str,
    /// Subsystem number when applicable (`"1"`…`"4"`), else `None`.
    pub subsystem: Option<&'static str>,
    pub kind: ReportKind,
    /// Full markdown (`include_str!`).
    pub markdown: &'static str,
}

impl Report {
    /// Byte length of embedded markdown.
    pub fn len(&self) -> usize {
        self.markdown.len()
    }

    pub fn is_empty(&self) -> bool {
        self.markdown.is_empty()
    }
}

/// All audit reports, ordered for reading (overview → inventory → subsystems → synthesis → dynamic).
pub fn reports() -> &'static [Report] {
    REPORTS
}

/// Lookup by [`Report::id`] (file stem).
pub fn report_by_id(id: &str) -> Option<&'static Report> {
    REPORTS.iter().find(|r| r.id == id)
}

/// Reports tagged with a subsystem id (`"1"`…`"4"`).
pub fn reports_for_subsystem(subsystem: &str) -> impl Iterator<Item = &'static Report> + '_ {
    REPORTS
        .iter()
        .filter(move |r| r.subsystem == Some(subsystem))
}

const REPORTS: &[Report] = &[
    Report {
        id: "00-overview",
        file: "00-overview.md",
        title: "Overview — DataGrip static layout",
        subsystem: None,
        kind: ReportKind::Overview,
        markdown: include_str!("../../reports/00-overview.md"),
    },
    Report {
        id: "01-app-inventory",
        file: "01-app-inventory.md",
        title: "App bundle inventory — DataGrip.app",
        subsystem: None,
        kind: ReportKind::Inventory,
        markdown: include_str!("../../reports/01-app-inventory.md"),
    },
    Report {
        id: "02-native-launcher",
        file: "02-native-launcher.md",
        title: "Native launcher — Contents/MacOS/datagrip",
        subsystem: None,
        kind: ReportKind::Inventory,
        markdown: include_str!("../../reports/02-native-launcher.md"),
    },
    Report {
        id: "03-info-plist",
        file: "03-info-plist.md",
        title: "Info.plist — bundle metadata & TCC",
        subsystem: None,
        kind: ReportKind::Inventory,
        markdown: include_str!("../../reports/03-info-plist.md"),
    },
    Report {
        id: "04-datagrip-impl-jar",
        file: "04-datagrip-impl-jar.md",
        title: "Plugin JAR — datagrip-impl.jar",
        subsystem: None,
        kind: ReportKind::Inventory,
        markdown: include_str!("../../reports/04-datagrip-impl-jar.md"),
    },
    Report {
        id: "05-database-plugin-jar",
        file: "05-database-plugin-jar.md",
        title: "Plugin JAR — database-plugin.jar",
        subsystem: None,
        kind: ReportKind::Inventory,
        markdown: include_str!("../../reports/05-database-plugin-jar.md"),
    },
    Report {
        id: "06-jdbc-connection-contents-lib",
        file: "06-jdbc-connection-contents-lib.md",
        title: "JDBC / connection subsystem — Contents/lib/ only",
        subsystem: None,
        kind: ReportKind::Inventory,
        markdown: include_str!("../../reports/06-jdbc-connection-contents-lib.md"),
    },
    Report {
        id: "07-core-architecture-lifecycle",
        file: "07-core-architecture-lifecycle.md",
        title: "Subsystem 1 — Core Architecture & Lifecycle",
        subsystem: Some("1"),
        kind: ReportKind::Subsystem,
        markdown: include_str!("../../reports/07-core-architecture-lifecycle.md"),
    },
    Report {
        id: "02-jdbc-connections",
        file: "02-jdbc-connections.md",
        title: "Subsystem 2 — JDBC & database connections",
        subsystem: Some("2"),
        kind: ReportKind::Subsystem,
        markdown: include_str!("../../reports/02-jdbc-connections.md"),
    },
    Report {
        id: "08-query-parsing-introspection",
        file: "08-query-parsing-introspection.md",
        title: "Subsystem 3 — Query Parsing & Schema Introspection",
        subsystem: Some("3"),
        kind: ReportKind::Subsystem,
        markdown: include_str!("../../reports/08-query-parsing-introspection.md"),
    },
    Report {
        id: "09-ui-component-mapping",
        file: "09-ui-component-mapping.md",
        title: "Subsystem 4 — User Interface & Component Mapping",
        subsystem: Some("4"),
        kind: ReportKind::Subsystem,
        markdown: include_str!("../../reports/09-ui-component-mapping.md"),
    },
    Report {
        id: "10-architecture-synthesis",
        file: "10-architecture-synthesis.md",
        title: "Architecture synthesis — cross-subsystem",
        subsystem: None,
        kind: ReportKind::Synthesis,
        markdown: include_str!("../../reports/10-architecture-synthesis.md"),
    },
    Report {
        id: "11-dynamic-validation",
        file: "11-dynamic-validation.md",
        title: "Phase 2a — Dynamic / runtime validation",
        subsystem: None,
        kind: ReportKind::DynamicValidation,
        markdown: include_str!("../../reports/11-dynamic-validation.md"),
    },
    Report {
        id: "12-dynamic-validation-2b",
        file: "12-dynamic-validation-2b.md",
        title: "Phase 2b — Remaining documented-only items",
        subsystem: None,
        kind: ReportKind::DynamicValidation,
        markdown: include_str!("../../reports/12-dynamic-validation-2b.md"),
    },
    Report {
        id: "13-ui-validation",
        file: "13-ui-validation.md",
        title: "Phase 2c — UI dynamic validation runbook",
        subsystem: None,
        kind: ReportKind::DynamicValidation,
        markdown: include_str!("../../reports/13-ui-validation.md"),
    },
    Report {
        id: "14-datagrip-reference-guide",
        file: "14-datagrip-reference-guide.md",
        title: "DataGrip: Complete Reference Guide",
        subsystem: None,
        kind: ReportKind::Guide,
        markdown: include_str!("../../reports/14-datagrip-reference-guide.md"),
    },
    Report {
        id: "15-native-drivers",
        file: "15-native-drivers.md",
        title: "DataDrift native drivers (pure Rust)",
        subsystem: None,
        kind: ReportKind::Guide,
        markdown: include_str!("../../reports/15-native-drivers.md"),
    },
    Report {
        id: "16-datagrip-driver-url-templates",
        file: "16-datagrip-driver-url-templates.md",
        title: "DataGrip driver URL templates → Grip map",
        subsystem: None,
        kind: ReportKind::Guide,
        markdown: include_str!("../../reports/16-datagrip-driver-url-templates.md"),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_eighteen_reports() {
        assert_eq!(reports().len(), 18);
        assert!(report_by_id("13-ui-validation").is_some());
        assert_eq!(
            report_by_id("13-ui-validation").unwrap().kind,
            ReportKind::DynamicValidation
        );
        assert_eq!(
            report_by_id("14-datagrip-reference-guide").unwrap().kind,
            ReportKind::Guide
        );
    }

    #[test]
    fn every_report_has_nonempty_markdown_and_heading() {
        for r in reports() {
            assert!(!r.is_empty(), "{} empty", r.id);
            assert!(
                r.markdown.contains('#'),
                "{} missing markdown heading",
                r.id
            );
        }
    }

    #[test]
    fn lookup_by_id() {
        let r = report_by_id("07-core-architecture-lifecycle").expect("subsystem 1");
        assert_eq!(r.subsystem, Some("1"));
        assert_eq!(r.kind, ReportKind::Subsystem);
    }

    #[test]
    fn subsystems_1_through_4() {
        for id in ["1", "2", "3", "4"] {
            let n = reports_for_subsystem(id).count();
            assert_eq!(n, 1, "subsystem {id}");
        }
    }

    #[test]
    fn jdbc_and_native_launcher_are_distinct_02_files() {
        assert!(report_by_id("02-native-launcher").is_some());
        assert!(report_by_id("02-jdbc-connections").is_some());
        assert_ne!(
            report_by_id("02-native-launcher").unwrap().file,
            report_by_id("02-jdbc-connections").unwrap().file
        );
    }

    #[test]
    fn helper_paths_are_documented() {
        assert_eq!(paths::VALIDATE_DYNAMIC_SH, "scripts/validate-dynamic.sh");
        assert_eq!(paths::REPORTS_DIR, "reports");
    }
}
