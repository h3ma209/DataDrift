mod async_rt;
mod connection;
mod drivers;
mod history;
mod jdbc;
mod model;
mod mysql_format;
mod sql;
mod store;

use connection::WorkerResponse;
use dioxus::prelude::*;
use history::HistoryEntry;
use model::{DataSource, DbVendor, SchemaNode};
use std::collections::HashMap;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new().with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("Grip")
                    .with_inner_size(dioxus::desktop::LogicalSize::new(1280.0, 820.0)),
            ),
        )
        .launch(App);
}

#[derive(Clone, PartialEq)]
enum Tab {
    Console,
    Table {
        schema: String,
        table: String,
    },
}

#[derive(Clone, Default)]
struct GridData {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[component]
fn App() -> Element {
    let mut sources = use_signal(|| {
        let mut s = store::load();
        if s.is_empty() {
            s.push(DataSource::new(1));
        }
        for ds in &mut s {
            ds.sync_jdbc_url();
        }
        s
    });
    let mut active = use_signal(|| 0usize);
    let mut connected = use_signal(HashMap::<u64, Vec<SchemaNode>>::new);
    let mut expanded = use_signal(HashMap::<u64, bool>::new);
    let mut tab = use_signal(|| Tab::Console);
    let mut sql = use_signal(|| "SELECT 1;\n".to_string());
    let mut where_clause = use_signal(String::new);
    let mut grid = use_signal(GridData::default);
    let mut log = use_signal(|| {
        vec![(
            true,
            "Grip — connect a data source, then run SQL.".to_string(),
        )]
    });
    let mut busy = use_signal(|| false);
    let mut show_ds = use_signal(|| false);

    let mut push_log = move |ok: bool, msg: String| {
        log.write().push((ok, msg));
    };

    let run_blocking = move |req: connection::WorkerRequest, mut grid: Signal<GridData>, mut connected: Signal<HashMap<u64, Vec<SchemaNode>>>, mut log: Signal<Vec<(bool, String)>>, mut busy: Signal<bool>, mut expanded: Signal<HashMap<u64, bool>>| {
        busy.set(true);
        spawn(async move {
            let resp = tokio::task::spawn_blocking(move || match req {
                connection::WorkerRequest::Test(ds) => {
                    let started = std::time::Instant::now();
                    match connection::test_connection(&ds) {
                        Ok(m) => WorkerResponse::TestResult {
                            source_id: ds.id,
                            ok: true,
                            message: m,
                            elapsed_ms: started.elapsed().as_millis(),
                        },
                        Err(e) => WorkerResponse::TestResult {
                            source_id: ds.id,
                            ok: false,
                            message: e,
                            elapsed_ms: started.elapsed().as_millis(),
                        },
                    }
                }
                connection::WorkerRequest::Connect(ds) => {
                    let id = ds.id;
                    match connection::connect_and_introspect(&ds) {
                        Ok((schemas, total)) => WorkerResponse::Connected {
                            source_id: id,
                            ok: true,
                            message: format!("Connected · {total} schema(s)"),
                            schemas,
                            schema_total: total,
                        },
                        Err(e) => WorkerResponse::Connected {
                            source_id: id,
                            ok: false,
                            message: e,
                            schemas: vec![],
                            schema_total: 0,
                        },
                    }
                }
                connection::WorkerRequest::Query { ds, sql, tab_id } => {
                    connection::run_query(&ds, &sql, tab_id)
                }
            })
            .await;

            busy.set(false);
            match resp {
                Ok(WorkerResponse::TestResult {
                    ok,
                    message,
                    elapsed_ms,
                    ..
                }) => {
                    log.write().push((
                        ok,
                        format!(
                            "{} ({} ms)",
                            if ok {
                                format!("✓ {message}")
                            } else {
                                format!("✗ {message}")
                            },
                            elapsed_ms
                        ),
                    ));
                }
                Ok(WorkerResponse::Connected {
                    source_id,
                    ok,
                    message,
                    schemas,
                    ..
                }) => {
                    log.write().push((ok, message));
                    if ok {
                        connected.write().insert(source_id, schemas);
                        expanded.write().insert(source_id, true);
                    }
                }
                Ok(WorkerResponse::QueryResult {
                    ok,
                    message,
                    columns,
                    rows,
                    ..
                }) => {
                    log.write().push((ok, message.clone()));
                    if ok {
                        grid.set(GridData { columns, rows });
                    }
                    history::push(HistoryEntry {
                        at: chrono::Local::now().format("%H:%M:%S").to_string(),
                        vendor: String::new(),
                        source: String::new(),
                        sql: String::new(),
                        ok,
                        message,
                    });
                }
                Err(e) => {
                    log.write().push((false, e.to_string()));
                }
            }
        });
    };

    let css = include_str!("../assets/main.css");

    rsx! {
        style { {css} }
        div { class: "app",
            // ——— Sidebar ———
            aside { class: "sidebar",
                div { class: "brand",
                    div { class: "brand-mark", "Grip" }
                    div { class: "brand-sub", "database workspace" }
                }
                div { class: "side-actions",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| show_ds.set(true),
                        "+ Source"
                    }
                    button {
                        class: "btn btn-soft",
                        disabled: *busy.read(),
                        onclick: move |_| {
                            let srcs = sources.read().clone();
                            let i = *active.read();
                            if let Some(ds) = srcs.get(i).cloned() {
                                run_blocking(
                                    connection::WorkerRequest::Connect(ds),
                                    grid,
                                    connected,
                                    log,
                                    busy,
                                    expanded,
                                );
                            }
                        },
                        "Connect"
                    }
                }
                div { class: "tree",
                    {
                        let srcs = sources.read().clone();
                        let conn = connected.read().clone();
                        let exp = expanded.read().clone();
                        let act = *active.read();
                        rsx! {
                            for (i, ds) in srcs.iter().enumerate() {
                                {
                                    let id = ds.id;
                                    let is_on = conn.contains_key(&id);
                                    let is_open = *exp.get(&id).unwrap_or(&false);
                                    let label = ds.tree_label();
                                    let vendor = ds.vendor.label();
                                    rsx! {
                                        button {
                                            class: if i == act { "ds-row active" } else { "ds-row" },
                                            onclick: move |_| {
                                                active.set(i);
                                                let mut e = expanded.write();
                                                let cur = *e.get(&id).unwrap_or(&false);
                                                e.insert(id, !cur);
                                            },
                                            span { class: if is_on { "ds-dot on" } else { "ds-dot" } }
                                            span { class: "chev", { if is_open { "▾" } else { "▸" } } }
                                            span { "{label}" }
                                            span { class: "chip", style: "margin-left:auto;", "{vendor}" }
                                        }
                                        if is_open {
                                            if let Some(schemas) = conn.get(&id) {
                                                for sch in schemas {
                                                    {
                                                        let sname = sch.name.clone();
                                                        rsx! {
                                                            div { class: "schema-row indent-1",
                                                                span { "⬡ {sname}" }
                                                            }
                                                            for tbl in sch.children.iter() {
                                                                {
                                                                    let sname2 = sname.clone();
                                                                    let tname = tbl.clone();
                                                                    let ds_clone = ds.clone();
                                                                    rsx! {
                                                                        button {
                                                                            class: "table-row indent-2",
                                                                            onclick: move |_| {
                                                                                tab.set(Tab::Table {
                                                                                    schema: sname2.clone(),
                                                                                    table: tname.clone(),
                                                                                });
                                                                                where_clause.set(String::new());
                                                                                let q = build_select(
                                                                                    ds_clone.vendor,
                                                                                    &sname2,
                                                                                    &tname,
                                                                                    "",
                                                                                    200,
                                                                                );
                                                                                run_blocking(
                                                                                    connection::WorkerRequest::Query {
                                                                                        ds: ds_clone.clone(),
                                                                                        sql: q,
                                                                                        tab_id: 1,
                                                                                    },
                                                                                    grid,
                                                                                    connected,
                                                                                    log,
                                                                                    busy,
                                                                                    expanded,
                                                                                );
                                                                            },
                                                                            span { class: "table-ico", "▦" }
                                                                            span { "{tname}" }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            } else if is_on {
                                                div { class: "empty indent-1", "No schemas" }
                                            } else {
                                                div { class: "empty indent-1", "Not connected" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ——— Main ———
            section { class: "main",
                div { class: "topbar",
                    div { class: "tabs",
                        button {
                            class: if matches!(*tab.read(), Tab::Console) { "tab active" } else { "tab" },
                            onclick: move |_| tab.set(Tab::Console),
                            "Console"
                        }
                        if let Tab::Table { schema, table } = tab.read().clone() {
                            button { class: "tab active", "{schema}.{table}" }
                        }
                    }
                    div { class: "status-pill",
                        if *busy.read() {
                            span { class: "spinner" }
                            span { "Running…" }
                        } else {
                            span { "Ready" }
                        }
                    }
                }

                div { class: "work",
                    // Editor / filters
                    div { class: "panel",
                        div { class: "panel-head",
                            span { class: "label",
                                {
                                    match tab.read().clone() {
                                        Tab::Console => "Query".to_string(),
                                        Tab::Table { .. } => "Data".to_string(),
                                    }
                                }
                            }
                            {
                                let srcs = sources.read();
                                let i = *active.read();
                                let name = srcs.get(i).map(|d| d.tree_label()).unwrap_or_default();
                                rsx! { span { class: "chip accent", "{name}" } }
                            }
                            div { style: "margin-left:auto; display:flex; gap:8px;",
                                if matches!(*tab.read(), Tab::Console) {
                                    button {
                                        class: "btn btn-primary",
                                        disabled: *busy.read(),
                                        onclick: move |_| {
                                            let srcs = sources.read().clone();
                                            let i = *active.read();
                                            let Some(ds) = srcs.get(i).cloned() else { return };
                                            let stmt = sql::execute_target(&sql.read(), sql.read().len(), None);
                                            push_log(true, format!("→ {stmt}"));
                                            run_blocking(
                                                connection::WorkerRequest::Query {
                                                    ds,
                                                    sql: stmt,
                                                    tab_id: 0,
                                                },
                                                grid,
                                                connected,
                                                log,
                                                busy,
                                                expanded,
                                            );
                                        },
                                        "Run"
                                    }
                                } else {
                                    button {
                                        class: "btn btn-ghost",
                                        disabled: *busy.read(),
                                        onclick: move |_| {
                                            if let Tab::Table { schema, table } = tab.read().clone() {
                                                let srcs = sources.read().clone();
                                                let i = *active.read();
                                                let Some(ds) = srcs.get(i).cloned() else { return };
                                                let q = build_select(
                                                    ds.vendor,
                                                    &schema,
                                                    &table,
                                                    &where_clause.read(),
                                                    200,
                                                );
                                                run_blocking(
                                                    connection::WorkerRequest::Query {
                                                        ds,
                                                        sql: q,
                                                        tab_id: 1,
                                                    },
                                                    grid,
                                                    connected,
                                                    log,
                                                    busy,
                                                    expanded,
                                                );
                                            }
                                        },
                                        "Reload"
                                    }
                                }
                            }
                        }
                        if matches!(*tab.read(), Tab::Console) {
                            textarea {
                                class: "sql-area",
                                value: "{sql}",
                                oninput: move |e| sql.set(e.value()),
                                spellcheck: "false",
                            }
                        } else {
                            div { class: "filter-bar",
                                span { class: "chip", "WHERE" }
                                input {
                                    value: "{where_clause}",
                                    placeholder: "id = 1",
                                    oninput: move |e| where_clause.set(e.value()),
                                    onkeydown: move |e| {
                                        if e.data().key() == Key::Enter {
                                            if let Tab::Table { schema, table } = tab.read().clone() {
                                                let srcs = sources.read().clone();
                                                let i = *active.read();
                                                let Some(ds) = srcs.get(i).cloned() else { return };
                                                let q = build_select(
                                                    ds.vendor,
                                                    &schema,
                                                    &table,
                                                    &where_clause.read(),
                                                    200,
                                                );
                                                run_blocking(
                                                    connection::WorkerRequest::Query {
                                                        ds,
                                                        sql: q,
                                                        tab_id: 1,
                                                    },
                                                    grid,
                                                    connected,
                                                    log,
                                                    busy,
                                                    expanded,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "empty", style: "padding:12px;",
                                "Edit cells coming next — browse & filter works now."
                            }
                        }
                    }

                    // Results
                    div { class: "panel",
                        div { class: "panel-head",
                            span { class: "label", "Result" }
                            {
                                let g = grid.read();
                                rsx! {
                                    span { class: "chip",
                                        "{g.columns.len()} cols · {g.rows.len()} rows"
                                    }
                                }
                            }
                        }
                        div { class: "grid-wrap",
                            {
                                let g = grid.read().clone();
                                if g.columns.is_empty() {
                                    rsx! { div { class: "empty", "Run a query to see results." } }
                                } else {
                                    rsx! {
                                        table { class: "grid",
                                            thead {
                                                tr {
                                                    th { "#" }
                                                    for c in g.columns.iter() {
                                                        th { "{c}" }
                                                    }
                                                }
                                            }
                                            tbody {
                                                for (ri, row) in g.rows.iter().enumerate() {
                                                    tr {
                                                        td { class: "rownum", "{ri + 1}" }
                                                        for cell in row.iter() {
                                                            {
                                                                let is_null = cell.is_empty() || cell == "NULL";
                                                                rsx! {
                                                                    td { class: if is_null { "null" } else { "" },
                                                                        { if is_null { "NULL" } else { cell.as_str() } }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "services",
                    {
                        let lines = log.read().clone();
                        rsx! {
                            for (ok, line) in lines.iter().rev().take(40) {
                                div {
                                    class: if *ok { "log-line log-ok" } else { "log-line log-err" },
                                    "{line}"
                                }
                            }
                        }
                    }
                }
            }
        }

        if *show_ds.read() {
            DataSourceModal {
                sources: sources,
                active: active,
                on_close: move |_| show_ds.set(false),
                on_test: move |_| {
                    let srcs = sources.read().clone();
                    let i = *active.read();
                    if let Some(ds) = srcs.get(i).cloned() {
                        run_blocking(
                            connection::WorkerRequest::Test(ds),
                            grid,
                            connected,
                            log,
                            busy,
                            expanded,
                        );
                    }
                },
                on_save: move |_| {
                    let _ = store::save(&sources.read());
                    show_ds.set(false);
                    push_log(true, "Saved data sources".into());
                },
            }
        }
    }
}

#[component]
fn DataSourceModal(
    sources: Signal<Vec<DataSource>>,
    active: Signal<usize>,
    on_close: EventHandler<()>,
    on_test: EventHandler<()>,
    on_save: EventHandler<()>,
) -> Element {
    let i = *active.read();
    rsx! {
        div { class: "modal-backdrop",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-head", "Data source" }
                div { class: "modal-body",
                    {
                        let mut list = sources;
                        rsx! {
                            div { class: "field",
                                label { "Vendor" }
                                select {
                                    onchange: move |e| {
                                        let v = match e.value().as_str() {
                                            "MySQL" => DbVendor::MySQL,
                                            "MariaDB" => DbVendor::MariaDB,
                                            "SQLite" => DbVendor::Sqlite,
                                            "ClickHouse" => DbVendor::ClickHouse,
                                            "Cassandra" => DbVendor::Cassandra,
                                            "MongoDB" => DbVendor::MongoDB,
                                            "Redis" => DbVendor::Redis,
                                            "Redshift" => DbVendor::Redshift,
                                            _ => DbVendor::PostgreSQL,
                                        };
                                        if let Some(ds) = list.write().get_mut(i) {
                                            ds.vendor = v;
                                            ds.apply_vendor_defaults();
                                        }
                                    },
                                    for v in DbVendor::ALL {
                                        option {
                                            selected: list.read().get(i).map(|d| d.vendor == *v).unwrap_or(false),
                                            value: "{v.label()}",
                                            "{v.label()}"
                                        }
                                    }
                                }
                            }
                            div { class: "field",
                                label { "Name" }
                                input {
                                    value: list.read().get(i).map(|d| d.name.clone()).unwrap_or_default(),
                                    oninput: move |e| {
                                        if let Some(ds) = list.write().get_mut(i) {
                                            ds.name = e.value();
                                        }
                                    }
                                }
                            }
                            div { class: "field",
                                label { "Host" }
                                input {
                                    value: list.read().get(i).map(|d| d.host.clone()).unwrap_or_default(),
                                    oninput: move |e| {
                                        if let Some(ds) = list.write().get_mut(i) {
                                            ds.host = e.value();
                                            ds.sync_jdbc_url();
                                        }
                                    }
                                }
                            }
                            div { class: "field",
                                label { "Port" }
                                input {
                                    value: list.read().get(i).map(|d| d.port.to_string()).unwrap_or_default(),
                                    oninput: move |e| {
                                        if let Some(ds) = list.write().get_mut(i) {
                                            ds.port = e.value().parse().unwrap_or(ds.port);
                                            ds.sync_jdbc_url();
                                        }
                                    }
                                }
                            }
                            div { class: "field",
                                label { "Database" }
                                input {
                                    value: list.read().get(i).map(|d| d.database.clone()).unwrap_or_default(),
                                    oninput: move |e| {
                                        if let Some(ds) = list.write().get_mut(i) {
                                            ds.database = e.value();
                                            ds.sync_jdbc_url();
                                        }
                                    }
                                }
                            }
                            div { class: "field",
                                label { "User" }
                                input {
                                    value: list.read().get(i).map(|d| d.user.clone()).unwrap_or_default(),
                                    oninput: move |e| {
                                        if let Some(ds) = list.write().get_mut(i) {
                                            ds.user = e.value();
                                        }
                                    }
                                }
                            }
                            div { class: "field",
                                label { "Password" }
                                input {
                                    r#type: "password",
                                    value: list.read().get(i).map(|d| d.password.clone()).unwrap_or_default(),
                                    oninput: move |e| {
                                        if let Some(ds) = list.write().get_mut(i) {
                                            ds.password = e.value();
                                        }
                                    }
                                }
                            }
                            div { class: "field",
                                label { "URL" }
                                input {
                                    value: list.read().get(i).map(|d| d.jdbc_url.clone()).unwrap_or_default(),
                                    readonly: true,
                                }
                            }
                        }
                    }
                }
                div { class: "modal-foot",
                    button { class: "btn btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn btn-ghost", onclick: move |_| on_test.call(()), "Test" }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            // add new
                            let mut s = sources.write();
                            let id = s.iter().map(|d| d.id).max().unwrap_or(0) + 1;
                            s.push(DataSource::new(id));
                            let n = s.len() - 1;
                            drop(s);
                            active.set(n);
                        },
                        "Add another"
                    }
                    button { class: "btn btn-primary", onclick: move |_| on_save.call(()), "Save" }
                }
            }
        }
    }
}

fn build_select(vendor: DbVendor, schema: &str, table: &str, where_c: &str, limit: u32) -> String {
    let from = match vendor {
        DbVendor::MySQL | DbVendor::MariaDB | DbVendor::ClickHouse => {
            format!("`{schema}`.`{table}`")
        }
        DbVendor::PostgreSQL | DbVendor::Redshift => format!("\"{schema}\".\"{table}\""),
        DbVendor::Sqlite => format!("\"{table}\""),
        DbVendor::Cassandra => format!("{schema}.{table}"),
        DbVendor::Redis => return format!("GET {table}"),
        DbVendor::MongoDB => return format!("db.{table}.find({{}});"),
    };
    let mut sql = format!("SELECT * FROM {from}");
    let w = where_c.trim();
    if !w.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(w);
    }
    sql.push_str(&format!(" LIMIT {limit}"));
    sql
}
