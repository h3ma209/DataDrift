//! ClickHouse via HTTP interface (8123). Pure Rust — no JDBC.

use crate::async_rt;
use crate::connection::{query_result_pub, WorkerResponse};
use crate::jdbc::ParsedJdbc;
use crate::model::{DataSource, SchemaNode};

fn base_url(p: &ParsedJdbc) -> String {
    format!("http://{}:{}/", p.host, p.port)
}

async fn http_query(
    ds: &DataSource,
    p: &ParsedJdbc,
    sql: &str,
    format: &str,
) -> Result<String, String> {
    let db = if p.database.is_empty() {
        "default"
    } else {
        p.database.as_str()
    };
    let mut req = reqwest::Client::new()
        .post(base_url(p))
        .query(&[("database", db), ("default_format", format)])
        .body(sql.to_string())
        .timeout(std::time::Duration::from_secs(30));
    if !ds.user.is_empty() {
        req = req.basic_auth(&ds.user, Some(ds.password.as_str()));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(text);
    }
    Ok(text)
}

pub fn test(ds: &DataSource, p: &ParsedJdbc) -> Result<String, String> {
    async_rt::block_on(async {
        let text = http_query(ds, p, "SELECT version()", "TabSeparated").await?;
        Ok(format!("ClickHouse {}", text.trim()))
    })
}

pub fn introspect(ds: &DataSource, p: &ParsedJdbc) -> Result<(Vec<SchemaNode>, usize), String> {
    async_rt::block_on(async {
        let sql = "SELECT database, name FROM system.tables \
                   WHERE database NOT IN ('system','information_schema','INFORMATION_SCHEMA') \
                   ORDER BY database, name LIMIT 500";
        let text = http_query(ds, p, sql, "TabSeparated").await?;
        let mut pairs = Vec::new();
        for line in text.lines().filter(|l| !l.is_empty()) {
            let mut parts = line.split('\t');
            if let (Some(db), Some(name)) = (parts.next(), parts.next()) {
                pairs.push((db.to_string(), name.to_string()));
            }
        }
        let schemas = group(pairs);
        let total = schemas.len();
        Ok((schemas, total))
    })
}

pub fn query(ds: &DataSource, p: &ParsedJdbc, sql: &str, tab_id: u64) -> WorkerResponse {
    async_rt::block_on(async {
        match http_query(ds, p, sql, "TabSeparatedWithNames").await {
            Ok(text) => match parse_tsv(&text) {
                Ok((cols, rows)) => {
                    query_result_pub(tab_id, true, format!("{} row(s)", rows.len()), cols, rows)
                }
                Err(e) => query_result_pub(tab_id, false, e, vec![], vec![]),
            },
            Err(e) => query_result_pub(tab_id, false, e, vec![], vec![]),
        }
    })
}

fn parse_tsv(text: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut lines = text.lines().filter(|l| !l.is_empty());
    let header = match lines.next() {
        Some(h) => h,
        None => return Ok((vec![], vec![])),
    };
    let cols: Vec<String> = header.split('\t').map(|s| s.to_string()).collect();
    let mut rows = Vec::new();
    for line in lines.take(500) {
        rows.push(line.split('\t').map(|s| s.to_string()).collect());
    }
    Ok((cols, rows))
}

fn group(pairs: Vec<(String, String)>) -> Vec<SchemaNode> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (schema, table) in pairs {
        if schema.is_empty() || table.is_empty() {
            continue;
        }
        map.entry(schema).or_default().push(table);
    }
    map.into_iter()
        .map(|(name, children)| SchemaNode { name, children })
        .collect()
}
