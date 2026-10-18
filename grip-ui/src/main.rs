mod async_rt;
mod connection;
mod drivers;
mod history;
mod jdbc;
mod model;
mod mysql_format;
mod settings;
mod sql;
mod store;

use connection::WorkerResponse;
use dioxus::prelude::*;
use history::HistoryEntry;
use model::{DataSource, DbVendor, SchemaNode};
use settings::UiSettings;
use std::collections::HashMap;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new().with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("Grip")
                    .with_inner_size(dioxus::desktop::LogicalSize::new(1280.0, 820.0))
                    .with_theme(Some(dioxus::desktop::tao::window::Theme::Dark)),
            ),
        )
        .launch(App);
}

#[derive(Clone, PartialEq)]
enum Tab {
    Console,
    Table { schema: String, table: String },
}

#[derive(Clone, Default)]
struct GridData {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Clone, PartialEq)]
struct Toast {
    ok: bool,
    msg: String,
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
            "Grip — connect a data source, then hit ▶".to_string(),
        )]
    });
    let mut busy = use_signal(|| false);
    let mut show_ds = use_signal(|| false);
    let mut show_settings = use_signal(|| false);
    let mut ui = use_signal(settings::load);
    let mut toast = use_signal(|| None::<Toast>);
    let mut lib_filter = use_signal(String::new);
    let mut last_stmt = use_signal(String::new);

    let mut push_log = move |ok: bool, msg: String| {
        log.write().push((ok, msg));
    };

    let flash = move |ok: bool, msg: String, mut toast: Signal<Option<Toast>>| {
        toast.set(Some(Toast {
            ok,
            msg: msg.clone(),
        }));
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2400)).await;
            toast.set(None);
        });
    };

    let run_blocking = move |req: connection::WorkerRequest,
                             mut grid: Signal<GridData>,
                             mut connected: Signal<HashMap<u64, Vec<SchemaNode>>>,
                             mut log: Signal<Vec<(bool, String)>>,
                             mut busy: Signal<bool>,
                             mut expanded: Signal<HashMap<u64, bool>>,
                             mut toast: Signal<Option<Toast>>| {
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
                    let line = format!(
                        "{} ({} ms)",
                        if ok {
                            format!("✓ {message}")
                        } else {
                            format!("✗ {message}")
                        },
                        elapsed_ms
                    );
                    log.write().push((ok, line.clone()));
                    toast.set(Some(Toast {
                        ok,
                        msg: if ok {
                            "Connection OK".into()
                        } else {
                            "Test failed".into()
                        },
                    }));
                    spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(2400)).await;
                        toast.set(None);
                    });
                }
                Ok(WorkerResponse::Connected {
                    source_id,
                    ok,
                    message,
                    schemas,
                    ..
                }) => {
                    log.write().push((ok, message.clone()));
                    if ok {
                        connected.write().insert(source_id, schemas);
                        expanded.write().insert(source_id, true);
                    }
                    toast.set(Some(Toast {
                        ok,
                        msg: if ok {
                            "Library synced".into()
                        } else {
                            message
                        },
                    }));
                    spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(2400)).await;
                        toast.set(None);
                    });
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
                        message: message.clone(),
                    });
                    toast.set(Some(Toast {
                        ok,
                        msg: if ok { message } else { format!("Query failed") },
                    }));
                    spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
                        toast.set(None);
                    });
                }
                Err(e) => {
                    log.write().push((false, e.to_string()));
                }
            }
        });
    };

    let css = include_str!("../assets/main.css");
    let prefs = ui.read().clone();
    let app_class = format!(
        "app glow-{}{}{}",
        prefs.glow.min(2),
        if prefs.reduce_motion {
            " reduce-motion"
        } else {
            ""
        },
        if prefs.compact { " compact" } else { "" }
    );

    let now_title = match tab.read().clone() {
        Tab::Console => "Console".to_string(),
        Tab::Table { schema, table } => format!("{schema}.{table}"),
    };
    let now_sub = {
        let srcs = sources.read();
        let i = *active.read();
        srcs.get(i)
            .map(|d| format!("{} · {}", d.vendor.label(), d.tree_label()))
            .unwrap_or_else(|| "No source".into())
    };

    rsx! {
        style { {css} }
        div { class: "{app_class}",
            div { class: if *busy.read() { "busy-bar on" } else { "busy-bar" } }

            // ——— Sidebar ———
            aside { class: "sidebar",
                div { class: "brand",
                    div { class: "brand-orb", aria_hidden: "true" }
                    div { class: "brand-text",
                        div { class: "brand-mark", "Grip" }
                        div { class: "brand-sub", "Your data" }
                    }
                }
                div { class: "side-actions",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| show_ds.set(true),
                        "+ Source"
                    }
                    button {
                        class: "btn btn-ghost",
                        disabled: *busy.read(),
                        onclick: move |_| {
                            let srcs = sources.read().clone();
                            let i = *active.read();
                            if let Some(ds) = srcs.get(i).cloned() {
                                run_blocking(
                                    connection::WorkerRequest::Connect(ds),
                                    grid, connected, log, busy, expanded, toast,
                                );
                            }
                        },
                        "Connect"
                    }
                }
                div { class: "nav-label",
                    span { "Library" }
                    button {
                        class: "icon-btn",
                        title: "Settings",
                        onclick: move |_| show_settings.set(true),
                        "⚙"
                    }
                }
                div { class: "search-wrap",
                    input {
                        class: "search",
                        placeholder: "Search tables…",
                        value: "{lib_filter}",
                        oninput: move |e| lib_filter.set(e.value()),
                    }
                }
                div { class: "tree",
                    {
                        let srcs = sources.read().clone();
                        let conn = connected.read().clone();
                        let exp = expanded.read().clone();
                        let act = *active.read();
                        let filter = lib_filter.read().to_lowercase();
                        let any_connected = !conn.is_empty();
                        rsx! {
                            if srcs.is_empty() {
                                div { class: "empty-hero",
                                    h2 { "Start listening" }
                                    p { "Add a data source to fill your library." }
                                }
                            }
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
                                            span { class: if is_open { "chev open" } else { "chev" },
                                                { if is_open { "▾" } else { "▸" } }
                                            }
                                            span { "{label}" }
                                            span { class: "chip", style: "margin-left:auto;", "{vendor}" }
                                        }
                                        if is_open {
                                            if let Some(schemas) = conn.get(&id) {
                                                for sch in schemas {
                                                    {
                                                        let sname = sch.name.clone();
                                                        let tables: Vec<String> = sch
                                                            .children
                                                            .iter()
                                                            .filter(|t| {
                                                                filter.is_empty()
                                                                    || t.to_lowercase().contains(&filter)
                                                                    || sname.to_lowercase().contains(&filter)
                                                            })
                                                            .cloned()
                                                            .collect();
                                                        if tables.is_empty() && !filter.is_empty() {
                                                            rsx! {}
                                                        } else {
                                                            rsx! {
                                                                div { class: "schema-row indent-1",
                                                                    span { "⬡ {sname}" }
                                                                }
                                                                for tname in tables {
                                                                    {
                                                                        let sname2 = sname.clone();
                                                                        let tname2 = tname.clone();
                                                                        let ds_clone = ds.clone();
                                                                        let active_tbl = matches!(
                                                                            tab.read().clone(),
                                                                            Tab::Table { schema: ref s, table: ref t }
                                                                            if s == &sname2 && t == &tname2
                                                                        );
                                                                        rsx! {
                                                                            button {
                                                                                class: if active_tbl { "table-row indent-2 active" } else { "table-row indent-2" },
                                                                                onclick: move |_| {
                                                                                    tab.set(Tab::Table {
                                                                                        schema: sname2.clone(),
                                                                                        table: tname2.clone(),
                                                                                    });
                                                                                    where_clause.set(String::new());
                                                                                    let q = build_select(
                                                                                        ds_clone.vendor,
                                                                                        &sname2,
                                                                                        &tname2,
                                                                                        "",
                                                                                        200,
                                                                                    );
                                                                                    last_stmt.set(q.clone());
                                                                                    run_blocking(
                                                                                        connection::WorkerRequest::Query {
                                                                                            ds: ds_clone.clone(),
                                                                                            sql: q,
                                                                                            tab_id: 1,
                                                                                        },
                                                                                        grid, connected, log, busy, expanded, toast,
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
                                                }
                                            } else if is_on {
                                                div { class: "empty indent-1", "No schemas" }
                                            } else {
                                                div { class: "empty indent-1", "Not connected — hit Connect" }
                                            }
                                        }
                                    }
                                }
                            }
                            if !any_connected && !srcs.is_empty() {
                                div { class: "empty", style: "padding:16px 8px;",
                                    "Pick a source, then Connect to sync library."
                                }
                            }
                        }
                    }
                }
            }

            // ——— Main stage ———
            section { class: "main",
                div { class: "topbar",
                    div { class: "tabs",
                        button {
                            class: if matches!(*tab.read(), Tab::Console) { "tab active" } else { "tab" },
                            onclick: move |_| tab.set(Tab::Console),
                            "Console"
                        }
                        if let Tab::Table { schema, table } = tab.read().clone() {
                            button {
                                class: "tab active",
                                "{schema}.{table}"
                            }
                        }
                    }
                    button {
                        class: "icon-btn",
                        title: "Settings",
                        onclick: move |_| show_settings.set(true),
                        "⚙"
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
                            span { class: "chip accent", "{now_sub}" }
                            div { style: "margin-left:auto; display:flex; gap:8px;",
                                if matches!(*tab.read(), Tab::Console) {
                                    button {
                                        class: "btn btn-primary btn-sm",
                                        disabled: *busy.read(),
                                        onclick: move |_| {
                                            let srcs = sources.read().clone();
                                            let i = *active.read();
                                            let Some(ds) = srcs.get(i).cloned() else { return };
                                            let stmt = sql::execute_target(
                                                &sql.read(),
                                                sql.read().len(),
                                                None,
                                            );
                                            last_stmt.set(stmt.clone());
                                            push_log(true, format!("→ {stmt}"));
                                            run_blocking(
                                                connection::WorkerRequest::Query {
                                                    ds,
                                                    sql: stmt,
                                                    tab_id: 0,
                                                },
                                                grid, connected, log, busy, expanded, toast,
                                            );
                                        },
                                        "▶ Run"
                                    }
                                } else {
                                    button {
                                        class: "btn btn-ghost btn-sm",
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
                                                last_stmt.set(q.clone());
                                                run_blocking(
                                                    connection::WorkerRequest::Query {
                                                        ds,
                                                        sql: q,
                                                        tab_id: 1,
                                                    },
                                                    grid, connected, log, busy, expanded, toast,
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
                                placeholder: "Write SQL — then hit ▶ Run",
                                oninput: move |e| sql.set(e.value()),
                                onkeydown: move |e| {
                                    let mods = e.modifiers();
                                    if (mods.meta() || mods.ctrl()) && e.data().key() == Key::Enter {
                                        let srcs = sources.read().clone();
                                        let i = *active.read();
                                        let Some(ds) = srcs.get(i).cloned() else { return };
                                        let stmt = sql::execute_target(
                                            &sql.read(),
                                            sql.read().len(),
                                            None,
                                        );
                                        last_stmt.set(stmt.clone());
                                        push_log(true, format!("→ {stmt}"));
                                        run_blocking(
                                            connection::WorkerRequest::Query {
                                                ds,
                                                sql: stmt,
                                                tab_id: 0,
                                            },
                                            grid, connected, log, busy, expanded, toast,
                                        );
                                    }
                                },
                                spellcheck: "false",
                            }
                            if ui.read().show_shortcuts {
                                div { class: "hint-bar",
                                    span {
                                        span { class: "kbd", "⌘" }
                                        span { class: "kbd", "⏎" }
                                        " Run statement"
                                    }
                                    span {
                                        span { class: "kbd", "⚙" }
                                        " Settings"
                                    }
                                }
                            }
                        } else {
                            div { class: "filter-bar",
                                span { class: "chip", "WHERE" }
                                input {
                                    value: "{where_clause}",
                                    placeholder: "id = 1  ·  Enter to reload",
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
                                                last_stmt.set(q.clone());
                                                run_blocking(
                                                    connection::WorkerRequest::Query {
                                                        ds,
                                                        sql: q,
                                                        tab_id: 1,
                                                    },
                                                    grid, connected, log, busy, expanded, toast,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "empty", style: "padding:14px;",
                                "Browse & filter live. In-place edit — next drop."
                            }
                        }
                    }

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
                                    rsx! {
                                        div { class: "empty-hero",
                                            h2 { "Nothing playing" }
                                            p { "Run a query or open a table from your library." }
                                            button {
                                                class: "btn btn-primary",
                                                style: "margin-top:8px;",
                                                onclick: move |_| tab.set(Tab::Console),
                                                "Open console"
                                            }
                                        }
                                    }
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
                            for (ok, line) in lines.iter().rev().take(24) {
                                div {
                                    class: if *ok { "log-line log-ok" } else { "log-line log-err" },
                                    "{line}"
                                }
                            }
                        }
                    }
                }

                // Spotify-style now bar
                div { class: "nowbar",
                    div { class: "now-art", aria_hidden: "true" }
                    div { class: "now-meta",
                        div { class: "now-title", "{now_title}" }
                        div { class: "now-sub",
                            {
                                let stmt = last_stmt.read().clone();
                                if stmt.is_empty() {
                                    now_sub.clone()
                                } else {
                                    let short: String = stmt.chars().take(64).collect();
                                    if stmt.len() > 64 {
                                        format!("{short}…")
                                    } else {
                                        short
                                    }
                                }
                            }
                        }
                    }
                    div { class: "now-actions",
                        button {
                            class: "play-btn",
                            disabled: *busy.read(),
                            title: "Run",
                            onclick: move |_| {
                                let srcs = sources.read().clone();
                                let i = *active.read();
                                let Some(ds) = srcs.get(i).cloned() else { return };
                                let stmt = match tab.read().clone() {
                                    Tab::Console => {
                                        sql::execute_target(&sql.read(), sql.read().len(), None)
                                    }
                                    Tab::Table { schema, table } => build_select(
                                        ds.vendor,
                                        &schema,
                                        &table,
                                        &where_clause.read(),
                                        200,
                                    ),
                                };
                                last_stmt.set(stmt.clone());
                                push_log(true, format!("→ {stmt}"));
                                let tab_id = if matches!(*tab.read(), Tab::Console) {
                                    0
                                } else {
                                    1
                                };
                                run_blocking(
                                    connection::WorkerRequest::Query {
                                        ds,
                                        sql: stmt,
                                        tab_id,
                                    },
                                    grid, connected, log, busy, expanded, toast,
                                );
                            },
                            "▶"
                        }
                    }
                }
            }
        }

        if let Some(t) = toast.read().clone() {
            div {
                class: if t.ok { "toast" } else { "toast err" },
                "{t.msg}"
            }
        }

        if *show_ds.read() {
            DataSourceModal {
                sources,
                active,
                on_close: move |_| show_ds.set(false),
                on_test: move |_| {
                    let srcs = sources.read().clone();
                    let i = *active.read();
                    if let Some(ds) = srcs.get(i).cloned() {
                        run_blocking(
                            connection::WorkerRequest::Test(ds),
                            grid, connected, log, busy, expanded, toast,
                        );
                    }
                },
                on_save: move |_| {
                    let _ = store::save(&sources.read());
                    show_ds.set(false);
                    push_log(true, "Saved data sources".into());
                    flash(true, "Saved".into(), toast);
                },
            }
        }

        if *show_settings.read() {
            SettingsModal {
                ui,
                on_close: move |_| show_settings.set(false),
            }
        }
    }
}

#[component]
fn SettingsModal(mut ui: Signal<UiSettings>, on_close: EventHandler<()>) -> Element {
    rsx! {
        div { class: "modal-backdrop",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-head", "Settings" }
                div { class: "modal-body",
                    div { class: "toggle-row",
                        span { "Reduce motion" }
                        button {
                            class: if ui.read().reduce_motion { "toggle on" } else { "toggle" },
                            onclick: move |_| {
                                let mut s = ui.write();
                                s.reduce_motion = !s.reduce_motion;
                            },
                            aria_label: "Reduce motion",
                        }
                    }
                    div { class: "toggle-row",
                        span { "Compact density" }
                        button {
                            class: if ui.read().compact { "toggle on" } else { "toggle" },
                            onclick: move |_| {
                                let mut s = ui.write();
                                s.compact = !s.compact;
                            },
                            aria_label: "Compact density",
                        }
                    }
                    div { class: "toggle-row",
                        span { "Show shortcut hints" }
                        button {
                            class: if ui.read().show_shortcuts { "toggle on" } else { "toggle" },
                            onclick: move |_| {
                                let mut s = ui.write();
                                s.show_shortcuts = !s.show_shortcuts;
                            },
                            aria_label: "Show shortcuts",
                        }
                    }
                    div { class: "field", style: "margin-top:8px;",
                        label { "Green glow" }
                        select {
                            onchange: move |e| {
                                let g: u8 = match e.value().as_str() {
                                    "0" => 0,
                                    "2" => 2,
                                    _ => 1,
                                };
                                ui.write().glow = g;
                            },
                            option {
                                value: "0",
                                selected: ui.read().glow == 0,
                                "Soft"
                            }
                            option {
                                value: "1",
                                selected: ui.read().glow == 1,
                                "Normal"
                            }
                            option {
                                value: "2",
                                selected: ui.read().glow == 2,
                                "Max"
                            }
                        }
                    }
                    p {
                        style: "font-size:12px; color:var(--ink-soft); margin:8px 0 0;",
                        "Prefs save to ~/.grip/ui_settings.json"
                    }
                }
                div { class: "modal-foot",
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| on_close.call(()),
                        "Close"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let _ = settings::save(&ui.read());
                            on_close.call(());
                        },
                        "Save"
                    }
                }
            }
        }
    }
}

#[component]
fn DataSourceModal(
    mut sources: Signal<Vec<DataSource>>,
    mut active: Signal<usize>,
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
                                if let Some(ds) = sources.write().get_mut(i) {
                                    ds.vendor = v;
                                    ds.apply_vendor_defaults();
                                }
                            },
                            for v in DbVendor::ALL {
                                option {
                                    selected: sources.read().get(i).map(|d| d.vendor == *v).unwrap_or(false),
                                    value: "{v.label()}",
                                    "{v.label()}"
                                }
                            }
                        }
                    }
                    div { class: "field",
                        label { "Name" }
                        input {
                            value: sources.read().get(i).map(|d| d.name.clone()).unwrap_or_default(),
                            oninput: move |e| {
                                if let Some(ds) = sources.write().get_mut(i) {
                                    ds.name = e.value();
                                }
                            }
                        }
                    }
                    div { class: "field",
                        label { "Host" }
                        input {
                            value: sources.read().get(i).map(|d| d.host.clone()).unwrap_or_default(),
                            oninput: move |e| {
                                if let Some(ds) = sources.write().get_mut(i) {
                                    ds.host = e.value();
                                    ds.sync_jdbc_url();
                                }
                            }
                        }
                    }
                    div { class: "field",
                        label { "Port" }
                        input {
                            value: sources.read().get(i).map(|d| d.port.to_string()).unwrap_or_default(),
                            oninput: move |e| {
                                if let Some(ds) = sources.write().get_mut(i) {
                                    ds.port = e.value().parse().unwrap_or(ds.port);
                                    ds.sync_jdbc_url();
                                }
                            }
                        }
                    }
                    div { class: "field",
                        label { "Database" }
                        input {
                            value: sources.read().get(i).map(|d| d.database.clone()).unwrap_or_default(),
                            oninput: move |e| {
                                if let Some(ds) = sources.write().get_mut(i) {
                                    ds.database = e.value();
                                    ds.sync_jdbc_url();
                                }
                            }
                        }
                    }
                    div { class: "field",
                        label { "User" }
                        input {
                            value: sources.read().get(i).map(|d| d.user.clone()).unwrap_or_default(),
                            oninput: move |e| {
                                if let Some(ds) = sources.write().get_mut(i) {
                                    ds.user = e.value();
                                }
                            }
                        }
                    }
                    div { class: "field",
                        label { "Password" }
                        input {
                            r#type: "password",
                            value: sources.read().get(i).map(|d| d.password.clone()).unwrap_or_default(),
                            oninput: move |e| {
                                if let Some(ds) = sources.write().get_mut(i) {
                                    ds.password = e.value();
                                }
                            }
                        }
                    }
                    div { class: "field",
                        label { "URL" }
                        input {
                            value: sources.read().get(i).map(|d| d.jdbc_url.clone()).unwrap_or_default(),
                            readonly: true,
                        }
                    }
                }
                div { class: "modal-foot",
                    button { class: "btn btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn btn-ghost", onclick: move |_| on_test.call(()), "Test" }
                    button {
                        class: "btn btn-soft",
                        onclick: move |_| {
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
