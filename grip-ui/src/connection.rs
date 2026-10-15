use crate::jdbc::{parse_and_validate, ParsedJdbc};
use mysql::prelude::Queryable;
use crate::model::{DataSource, SchemaNode, VendorWire};
use postgres::{Client, NoTls};
use rusqlite::Connection as SqliteConn;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub enum WorkerRequest {
    Test(DataSource),
    Connect(DataSource),
    Query {
        ds: DataSource,
        sql: String,
        tab_id: u64,
    },
}

pub enum WorkerResponse {
    TestResult {
        source_id: u64,
        ok: bool,
        message: String,
        elapsed_ms: u128,
    },
    Connected {
        source_id: u64,
        ok: bool,
        message: String,
        schemas: Vec<SchemaNode>,
        schema_total: usize,
    },
    QueryResult {
        tab_id: u64,
        ok: bool,
        message: String,
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

pub struct ConnectionWorker {
    tx: mpsc::Sender<WorkerRequest>,
    rx: mpsc::Receiver<WorkerResponse>,
}

impl ConnectionWorker {
    pub fn spawn() -> Self {
        let (req_tx, req_rx) = mpsc::channel();
        let (resp_tx, resp_rx) = mpsc::channel();
        thread::spawn(move || worker_loop(req_rx, resp_tx));
        Self {
            tx: req_tx,
            rx: resp_rx,
        }
    }

    pub fn send(&self, req: WorkerRequest) {
        let _ = self.tx.send(req);
    }

    pub fn try_recv(&self) -> Option<WorkerResponse> {
        self.rx.try_recv().ok()
    }
}

fn worker_loop(req_rx: mpsc::Receiver<WorkerRequest>, resp_tx: mpsc::Sender<WorkerResponse>) {
    for req in req_rx {
        match req {
            WorkerRequest::Test(ds) => {
                let started = std::time::Instant::now();
                let r = test_connection(&ds);
                let elapsed = started.elapsed().as_millis();
                let sid = ds.id;
                let _ = resp_tx.send(WorkerResponse::TestResult {
                    source_id: sid,
                    ok: r.is_ok(),
                    message: r.unwrap_or_else(|e| e),
                    elapsed_ms: elapsed,
                });
            }
            WorkerRequest::Connect(ds) => {
                let sid = ds.id;
                let r = connect_and_introspect(&ds);
                match r {
                    Ok((schemas, schema_total)) => {
                        let _ = resp_tx.send(WorkerResponse::Connected {
                            source_id: sid,
                            ok: true,
                            message: "Connected".into(),
                            schemas,
                            schema_total,
                        });
                    }
                    Err(e) => {
                        let _ = resp_tx.send(WorkerResponse::Connected {
                            source_id: sid,
                            ok: false,
                            message: e,
                            schemas: vec![],
                            schema_total: 0,
                        });
                    }
                }
            }
            WorkerRequest::Query { ds, sql, tab_id } => {
                let _ = resp_tx.send(run_query(&ds, &sql, tab_id));
            }
        }
    }
}

fn resolve(ds: &DataSource) -> Result<ParsedJdbc, String> {
    if ds.use_url {
        parse_and_validate(&ds.jdbc_url)
    } else {
        Ok(ParsedJdbc {
            vendor: ds.vendor,
            host: ds.host.clone(),
            port: ds.port,
            database: ds.database.clone(),
            raw: crate::jdbc::build_url(ds),
        })
    }
}

fn test_connection(ds: &DataSource) -> Result<String, String> {
    let p = resolve(ds)?;
    match p.vendor.wire() {
        VendorWire::ClickHouse | VendorWire::Cassandra | VendorWire::Mongo | VendorWire::Redis => {
            crate::drivers::test(ds, &p)
        }
        VendorWire::Postgres => {
            let conn_str = format!(
                "host={} port={} dbname={} user={} password={} connect_timeout=5",
                p.host, p.port, p.database, ds.user, ds.password
            );
            let mut client = Client::connect(&conn_str, NoTls).map_err(|e| e.to_string())?;
            let row = client
                .query_one("SELECT version()", &[])
                .map_err(|e| e.to_string())?;
            let ver: String = row.get(0);
            Ok(format!("{ver}"))
        }
        VendorWire::Mysql => {
            let opts = mysql::OptsBuilder::new()
                .ip_or_hostname(Some(p.host.as_str()))
                .tcp_port(p.port)
                .db_name(Some(p.database.as_str()))
                .user(Some(ds.user.as_str()))
                .pass(Some(ds.password.as_str()))
                .tcp_connect_timeout(Some(Duration::from_secs(5)));
            let pool = mysql::Pool::new(opts).map_err(|e| e.to_string())?;
            let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
            let ver: Option<String> = conn
                .query_first("SELECT VERSION()")
                .map_err(|e| e.to_string())?;
            Ok(ver.unwrap_or_else(|| "MySQL/MariaDB".into()))
        }
        VendorWire::Sqlite => {
            let path = if p.database == ":memory:" {
                ":memory:"
            } else {
                p.database.as_str()
            };
            let conn = SqliteConn::open(path).map_err(|e| e.to_string())?;
            let ver: String = conn
                .query_row("SELECT sqlite_version()", [], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            Ok(format!("SQLite {ver}"))
        }
    }
}

fn connect_and_introspect(ds: &DataSource) -> Result<(Vec<SchemaNode>, usize), String> {
    test_connection(ds)?;
    let p = resolve(ds)?;
    match p.vendor.wire() {
        VendorWire::ClickHouse | VendorWire::Cassandra | VendorWire::Mongo | VendorWire::Redis => {
            crate::drivers::introspect(ds, &p)
        }
        VendorWire::Postgres => {
            let conn_str = format!(
                "host={} port={} dbname={} user={} password={}",
                p.host, p.port, p.database, ds.user, ds.password
            );
            let mut client = Client::connect(&conn_str, NoTls).map_err(|e| e.to_string())?;
            let rows = client
                .query(
                    "SELECT table_schema, table_name FROM information_schema.tables \
                     WHERE table_schema NOT IN ('pg_catalog','information_schema') \
                     ORDER BY 1, 2 LIMIT 200",
                    &[],
                )
                .map_err(|e| e.to_string())?;
            let pairs: Vec<(String, String)> = rows
                .iter()
                .map(|r| (r.get::<_, String>(0), r.get::<_, String>(1)))
                .collect();
            let schemas = group_schema_tables(pairs);
            let total: i64 = client
                .query_one(
                    "SELECT COUNT(*) FROM information_schema.schemata \
                     WHERE schema_name NOT IN ('pg_catalog','information_schema')",
                    &[],
                )
                .map_err(|e| e.to_string())?
                .get(0);
            Ok((schemas, total as usize))
        }
        VendorWire::Mysql => {
            let opts = mysql::OptsBuilder::new()
                .ip_or_hostname(Some(p.host.as_str()))
                .tcp_port(p.port)
                .db_name(Some(p.database.as_str()))
                .user(Some(ds.user.as_str()))
                .pass(Some(ds.password.as_str()));
            let pool = mysql::Pool::new(opts).map_err(|e| e.to_string())?;
            let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
            let rows: Vec<(String, String)> = conn
                .query(
                    "SELECT table_schema, table_name FROM information_schema.tables \
                     WHERE table_schema NOT IN ('information_schema','performance_schema','sys') \
                     ORDER BY 1, 2 LIMIT 500",
                )
                .map_err(|e| e.to_string())?;
            let schemas = group_schema_tables(rows);
            let total: Option<u64> = conn
                .query_first(
                    "SELECT COUNT(*) FROM information_schema.schemata \
                     WHERE schema_name NOT IN ('information_schema','performance_schema','sys')",
                )
                .map_err(|e| e.to_string())?;
            Ok((schemas, total.unwrap_or(0) as usize))
        }
        VendorWire::Sqlite => {
            let path = if p.database == ":memory:" {
                ":memory:"
            } else {
                p.database.as_str()
            };
            let conn = SqliteConn::open(path).map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY 1")
                .map_err(|e| e.to_string())?;
            let tables: Vec<String> = stmt
                .query_map([], |r| r.get(0))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok((
                vec![SchemaNode {
                    name: "main".into(),
                    children: tables,
                }],
                1,
            ))
        }
    }
}

fn group_schema_tables(pairs: Vec<(String, String)>) -> Vec<SchemaNode> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (schema, table) in pairs {
        if schema.trim().is_empty() || table.trim().is_empty() {
            continue;
        }
        map.entry(schema).or_default().push(table);
    }
    map.into_iter()
        .map(|(name, children)| SchemaNode { name, children })
        .collect()
}

pub fn query_result_pub(
    tab_id: u64,
    ok: bool,
    message: impl Into<String>,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
) -> WorkerResponse {
    WorkerResponse::QueryResult {
        tab_id,
        ok,
        message: message.into(),
        columns,
        rows,
    }
}

fn query_result(
    tab_id: u64,
    ok: bool,
    message: impl Into<String>,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
) -> WorkerResponse {
    query_result_pub(tab_id, ok, message, columns, rows)
}

fn run_query(ds: &DataSource, sql: &str, tab_id: u64) -> WorkerResponse {
    let sql = sql.trim();
    if sql.is_empty() {
        return query_result(tab_id, false, "Empty statement", vec![], vec![]);
    }
    let p = match resolve(ds) {
        Ok(p) => p,
        Err(e) => return query_result(tab_id, false, e, vec![], vec![]),
    };
    match p.vendor.wire() {
        VendorWire::ClickHouse | VendorWire::Cassandra | VendorWire::Mongo | VendorWire::Redis => {
            return crate::drivers::query(ds, &p, sql, tab_id);
        }
        VendorWire::Postgres => {
            let conn_str = format!(
                "host={} port={} dbname={} user={} password={}",
                p.host, p.port, p.database, ds.user, ds.password
            );
            match Client::connect(&conn_str, NoTls) {
                Ok(mut client) => match client.query(sql, &[]) {
                    Ok(rows) => {
                        if rows.is_empty() {
                            return query_result(tab_id, true, "OK (0 rows)", vec![], vec![]);
                        }
                        let cols: Vec<String> = rows[0]
                            .columns()
                            .iter()
                            .map(|c| c.name().to_string())
                            .collect();
                        let mut out = Vec::new();
                        for row in rows.iter().take(500) {
                            let mut line = Vec::new();
                            for i in 0..row.len() {
                                let s: String = row
                                    .try_get(i)
                                    .unwrap_or_else(|_| "<null>".into());
                                line.push(s);
                            }
                            out.push(line);
                        }
                        query_result(
                            tab_id,
                            true,
                            format!("{} row(s)", out.len()),
                            cols,
                            out,
                        )
                    }
                    Err(e) => query_result(tab_id, false, e.to_string(), vec![], vec![]),
                },
                Err(e) => query_result(tab_id, false, e.to_string(), vec![], vec![]),
            }
        }
        VendorWire::Sqlite => {
            let path = if p.database == ":memory:" {
                ":memory:"
            } else {
                p.database.as_str()
            };
            match SqliteConn::open(path) {
                Ok(conn) => {
                    let mut stmt = match conn.prepare(sql) {
                        Ok(s) => s,
                        Err(e) => return query_result(tab_id, false, e.to_string(), vec![], vec![]),
                    };
                    let col_count = stmt.column_count();
                    let cols: Vec<String> = (0..col_count)
                        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
                        .collect();
                    let mut rows = Vec::new();
                    let mut q = stmt.query([]).ok();
                    if let Some(ref mut q) = q {
                        while let Ok(Some(row)) = q.next() {
                            let mut line = Vec::new();
                            for i in 0..col_count {
                                let v: rusqlite::types::Value =
                                    row.get(i).unwrap_or(rusqlite::types::Value::Null);
                                line.push(sqlite_value_display(v));
                            }
                            rows.push(line);
                            if rows.len() >= 500 {
                                break;
                            }
                        }
                    }
                    query_result(
                        tab_id,
                        true,
                        format!("{} row(s)", rows.len()),
                        cols,
                        rows,
                    )
                }
                Err(e) => query_result(tab_id, false, e.to_string(), vec![], vec![]),
            }
        }
        VendorWire::Mysql => {
            let opts = mysql::OptsBuilder::new()
                .ip_or_hostname(Some(p.host.as_str()))
                .tcp_port(p.port)
                .db_name(Some(p.database.as_str()))
                .user(Some(ds.user.as_str()))
                .pass(Some(ds.password.as_str()))
                .tcp_connect_timeout(Some(Duration::from_secs(5)));
            match mysql::Pool::new(opts) {
                Ok(pool) => match pool.get_conn() {
                    Ok(mut conn) => match conn.query_iter(sql) {
                        Ok(mut qr) => {
                            let Some(mut rs) = qr.iter() else {
                                return query_result(
                                    tab_id,
                                    true,
                                    format!("OK ({} row(s) affected)", qr.affected_rows()),
                                    vec![],
                                    vec![],
                                );
                            };
                            let cols: Vec<String> = rs
                                .columns()
                                .as_ref()
                                .iter()
                                .map(|c| String::from_utf8_lossy(c.name_ref()).into_owned())
                                .collect();
                            let mut rows = Vec::new();
                            for row in &mut rs {
                                let row = match row {
                                    Ok(r) => r,
                                    Err(e) => {
                                        return query_result(
                                            tab_id,
                                            false,
                                            e.to_string(),
                                            vec![],
                                            vec![],
                                        );
                                    }
                                };
                                let mut line = Vec::new();
                                for i in 0..cols.len() {
                                    line.push(crate::mysql_format::row_cell(&row, i));
                                }
                                rows.push(line);
                                if rows.len() >= 500 {
                                    break;
                                }
                            }
                            query_result(
                                tab_id,
                                true,
                                format!("{} row(s)", rows.len()),
                                cols,
                                rows,
                            )
                        }
                        Err(e) => query_result(tab_id, false, e.to_string(), vec![], vec![]),
                    },
                    Err(e) => query_result(tab_id, false, e.to_string(), vec![], vec![]),
                },
                Err(e) => query_result(tab_id, false, e.to_string(), vec![], vec![]),
            }
        }
    }
}

fn sqlite_value_display(v: rusqlite::types::Value) -> String {
    match v {
        rusqlite::types::Value::Null => "NULL".into(),
        rusqlite::types::Value::Integer(n) => n.to_string(),
        rusqlite::types::Value::Real(f) => {
            if f.fract() == 0.0 && f.is_finite() {
                format!("{:.0}", f)
            } else {
                f.to_string()
            }
        }
        rusqlite::types::Value::Text(s) => s,
        rusqlite::types::Value::Blob(b) => match std::str::from_utf8(&b) {
            Ok(s) => s.to_string(),
            Err(_) => format!("<blob {} bytes>", b.len()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DataSource, DbVendor};

    fn docker_mysql_ds() -> DataSource {
        let mut ds = DataSource::new(1);
        ds.name = "MySQL 8.4 @localhost".into();
        ds.vendor = DbVendor::MySQL;
        ds.host = "localhost".into();
        ds.port = 3306;
        ds.database = "mysql".into();
        ds.user = "root".into();
        ds.password = "88888888".into();
        ds.remember_password = true;
        ds.sync_jdbc_url();
        ds
    }

    #[test]
    fn mysql_localhost_docker() {
        let ds = docker_mysql_ds();
        let ver = test_connection(&ds).expect("test connection");
        assert!(ver.contains("8.4"), "unexpected version: {ver}");
        let (schemas, _total) = connect_and_introspect(&ds).expect("connect");
        assert!(
            schemas.iter().any(|s| s.name == "mysql"),
            "expected mysql schema in explorer"
        );
    }
}
