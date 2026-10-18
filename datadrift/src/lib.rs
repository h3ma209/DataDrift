//! DataDrift — DataGrip reverse-engineering knowledge for the DataDrift recreation.
//!
//! Audit markdown lives in repo-root [`reports/`](../../reports/). Access via
//! [`datagrip::reports`]. Large JAR indexes stay under `tmp/` (gitignored);
//! see [`datagrip::paths`].

pub mod datagrip;

pub use datagrip::{report_by_id, reports, Report, ReportKind};
