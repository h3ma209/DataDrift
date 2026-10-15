//! Apache Cassandra / ScyllaDB via `scylla` CQL driver (pure Rust).

use crate::async_rt;
use crate::connection::{query_result_pub, WorkerResponse};
use crate::jdbc::ParsedJdbc;
use crate::model::{DataSource, SchemaNode};
use scylla::SessionBuilder;
use std::time::Duration;

async fn session(
    ds: &DataSource,
    p: &ParsedJdbc,
) -> Result<scylla::Session, String> {
    let addr = format!("{}:{}", p.host, p.port);
    let mut b = SessionBuilder::new()
        .known_node(addr)
        .connection_timeout(Duration::from_secs(8));
    if !ds.user.is_empty() {
        b = b.user(ds.user.as_str(), ds.password.as_str());
    }
    let session = b.build().await.map_err(|e| e.to_string())?;
    if !p.database.is_empty() && p.database != "system" {
        session
            .use_keyspace(p.database.as_str(), false)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(session)
}

pub fn test(ds: &DataSource, p: &ParsedJdbc) -> Result<String, String> {
    async_rt::block_on(async {
        let s = session(ds, p).await?;
        let result = s
            .query_unpaged("SELECT release_version FROM system.local", &[])
            .await
            .map_err(|e| e.to_string())?;
        if let Ok(rows) = result.into_rows_result() {
            for row in rows.rows::<(Option<String>,)>().into_iter().flatten() {
                if let Ok((Some(v),)) = row {
                    return Ok(format!("Cassandra/Scylla {v}"));
                }
            }
        }
        Ok("Cassandra/Scylla connected".into())
    })
}

pub fn introspect(ds: &DataSource, p: &ParsedJdbc) -> Result<(Vec<SchemaNode>, usize), String> {
    async_rt::block_on(async {
        let s = session(ds, p).await?;
        let result = s
            .query_unpaged(
                "SELECT keyspace_name, table_name FROM system_schema.tables",
                &[],
            )
            .await
            .map_err(|e| e.to_string())?;
        let mut pairs = Vec::new();
        if let Ok(rows) = result.into_rows_result() {
            for row in rows
                .rows::<(String, String)>()
                .map_err(|e| e.to_string())?
                .take(500)
            {
                if let Ok((ks, tbl)) = row {
                    if ks.starts_with("system") {
                        continue;
                    }
                    pairs.push((ks, tbl));
                }
            }
        }
        let schemas = group(pairs);
        let total = schemas.len();
        Ok((schemas, total))
    })
}

pub fn query(ds: &DataSource, p: &ParsedJdbc, cql: &str, tab_id: u64) -> WorkerResponse {
    async_rt::block_on(async {
        let s = match session(ds, p).await {
            Ok(s) => s,
            Err(e) => return query_result_pub(tab_id, false, e, vec![], vec![]),
        };
        let result = match s.query_unpaged(cql, &[]).await {
            Ok(r) => r,
            Err(e) => return query_result_pub(tab_id, false, e.to_string(), vec![], vec![]),
        };
        let rows_result = match result.into_rows_result() {
            Ok(r) => r,
            Err(_) => {
                return query_result_pub(tab_id, true, "OK (no row set)", vec![], vec![]);
            }
        };
        let cols: Vec<String> = rows_result
            .column_specs()
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        let mut out = Vec::new();
        match rows_result.rows::<scylla::frame::response::result::Row>() {
            Ok(iter) => {
                for row in iter.take(500) {
                    match row {
                        Ok(r) => {
                            let cells: Vec<String> = r
                                .columns
                                .iter()
                                .map(|opt| match opt {
                                    None => "NULL".into(),
                                    Some(v) => cql_cell(v),
                                })
                                .collect();
                            out.push(cells);
                        }
                        Err(e) => {
                            return query_result_pub(tab_id, false, e.to_string(), vec![], vec![]);
                        }
                    }
                }
            }
            Err(e) => return query_result_pub(tab_id, false, e.to_string(), vec![], vec![]),
        }
        query_result_pub(tab_id, true, format!("{} row(s)", out.len()), cols, out)
    })
}

fn cql_cell(v: &scylla::frame::response::result::CqlValue) -> String {
    use scylla::frame::response::result::CqlValue::*;
    match v {
        Ascii(s) | Text(s) => s.clone(),
        Boolean(b) => b.to_string(),
        Int(i) => i.to_string(),
        BigInt(i) => i.to_string(),
        SmallInt(i) => i.to_string(),
        TinyInt(i) => i.to_string(),
        Float(f) => f.to_string(),
        Double(d) => d.to_string(),
        Uuid(u) => u.to_string(),
        Timeuuid(u) => u.to_string(),
        Timestamp(t) => format!("{t:?}"),
        Date(d) => format!("{d:?}"),
        Time(t) => format!("{t:?}"),
        Inet(ip) => ip.to_string(),
        Blob(b) => format!("0x{}", hex_preview(b)),
        Counter(c) => format!("{c:?}"),
        Decimal(d) => format!("{d:?}"),
        Varint(v) => format!("{v:?}"),
        List(items) | Set(items) => {
            let inner: Vec<String> = items.iter().map(cql_cell).collect();
            format!("[{}]", inner.join(", "))
        }
        Map(pairs) => {
            let inner: Vec<String> = pairs
                .iter()
                .map(|(k, v)| format!("{}:{}", cql_cell(k), cql_cell(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
        UserDefinedType { fields, .. } => {
            let inner: Vec<String> = fields
                .iter()
                .map(|(n, opt)| match opt {
                    None => format!("{n}:NULL"),
                    Some(v) => format!("{n}:{}", cql_cell(v)),
                })
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
        Tuple(items) => {
            let inner: Vec<String> = items
                .iter()
                .map(|opt| match opt {
                    None => "NULL".into(),
                    Some(v) => cql_cell(v),
                })
                .collect();
            format!("({})", inner.join(", "))
        }
        Duration(d) => format!("{d:?}"),
        Empty => String::new(),
    }
}

fn hex_preview(b: &[u8]) -> String {
    const MAX: usize = 32;
    let take = b.len().min(MAX);
    let mut s = String::with_capacity(take * 2);
    for byte in &b[..take] {
        s.push_str(&format!("{byte:02x}"));
    }
    if b.len() > MAX {
        s.push('…');
    }
    s
}

fn group(pairs: Vec<(String, String)>) -> Vec<SchemaNode> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (schema, table) in pairs {
        map.entry(schema).or_default().push(table);
    }
    map.into_iter()
        .map(|(name, children)| SchemaNode { name, children })
        .collect()
}
