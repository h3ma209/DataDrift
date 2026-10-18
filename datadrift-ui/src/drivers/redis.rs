//! Redis via RESP (`redis` crate). DataGrip dialect jar → `jdbc:redis://host:6379/db`.

use crate::async_rt;
use crate::connection::{query_result_pub, WorkerResponse};
use crate::jdbc::ParsedJdbc;
use crate::model::{DataSource, SchemaNode};
fn client(ds: &DataSource, p: &ParsedJdbc) -> Result<redis::Client, String> {
    let db: i64 = p.database.parse().unwrap_or(0);
    let url = if ds.password.is_empty() {
        format!("redis://{}:{}/{}", p.host, p.port, db)
    } else if ds.user.is_empty() {
        format!("redis://:{}@{}:{}/{}", ds.password, p.host, p.port, db)
    } else {
        format!(
            "redis://{}:{}@{}:{}/{}",
            ds.user, ds.password, p.host, p.port, db
        )
    };
    redis::Client::open(url).map_err(|e| e.to_string())
}

pub fn test(ds: &DataSource, p: &ParsedJdbc) -> Result<String, String> {
    async_rt::block_on(async {
        // redis crate is sync; run on blocking pool of current runtime
        let ds = ds.clone();
        let p = p.clone();
        tokio::task::spawn_blocking(move || {
            let c = client(&ds, &p)?;
            let mut con = c.get_connection().map_err(|e| e.to_string())?;
            let pong: String = redis::cmd("PING")
                .query(&mut con)
                .map_err(|e| e.to_string())?;
            let info: String = redis::cmd("INFO")
                .arg("server")
                .query(&mut con)
                .unwrap_or_default();
            let ver = info
                .lines()
                .find(|l| l.starts_with("redis_version:"))
                .map(|l| l.trim_start_matches("redis_version:").to_string())
                .unwrap_or_else(|| pong.clone());
            Ok(format!("Redis {ver}"))
        })
        .await
        .map_err(|e| e.to_string())?
    })
}

pub fn introspect(ds: &DataSource, p: &ParsedJdbc) -> Result<(Vec<SchemaNode>, usize), String> {
    async_rt::block_on(async {
        let ds = ds.clone();
        let p = p.clone();
        tokio::task::spawn_blocking(move || {
            let c = client(&ds, &p)?;
            let mut con = c.get_connection().map_err(|e| e.to_string())?;
            // SCAN first 200 keys — DataGrip Redis tree is key-oriented.
            let mut cursor: u64 = 0;
            let mut keys: Vec<String> = Vec::new();
            loop {
                let (next, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("COUNT")
                    .arg(50)
                    .query(&mut con)
                    .map_err(|e| e.to_string())?;
                keys.extend(batch);
                cursor = next;
                if cursor == 0 || keys.len() >= 200 {
                    break;
                }
            }
            keys.sort();
            keys.truncate(200);
            let db_label = if p.database.is_empty() {
                "0".into()
            } else {
                p.database.clone()
            };
            let schemas = vec![SchemaNode {
                name: format!("db{db_label}"),
                children: keys,
            }];
            Ok((schemas, 1))
        })
        .await
        .map_err(|e| e.to_string())?
    })
}

/// Console: Redis CLI-style — `PING`, `GET key`, `KEYS pattern`, `TYPE key`, `HGETALL key`, …
pub fn query(ds: &DataSource, p: &ParsedJdbc, cmd: &str, tab_id: u64) -> WorkerResponse {
    async_rt::block_on(async {
        let ds = ds.clone();
        let p = p.clone();
        let cmd = cmd.trim().to_string();
        match tokio::task::spawn_blocking(move || run_cmd(&ds, &p, &cmd)).await {
            Ok(Ok((cols, rows, msg))) => query_result_pub(tab_id, true, msg, cols, rows),
            Ok(Err(e)) => query_result_pub(tab_id, false, e, vec![], vec![]),
            Err(e) => query_result_pub(tab_id, false, e.to_string(), vec![], vec![]),
        }
    })
}

fn run_cmd(
    ds: &DataSource,
    p: &ParsedJdbc,
    cmd: &str,
) -> Result<(Vec<String>, Vec<Vec<String>>, String), String> {
    let c = client(ds, p)?;
    let mut con = c.get_connection().map_err(|e| e.to_string())?;
    let parts = shell_split(cmd);
    if parts.is_empty() {
        return Err("empty Redis command".into());
    }
    let mut redis_cmd = redis::cmd(parts[0].as_str());
    for a in &parts[1..] {
        redis_cmd.arg(a.as_str());
    }
    let val: redis::Value = redis_cmd.query(&mut con).map_err(|e| e.to_string())?;
    let rows = flatten_value(&val);
    let cols = if rows.first().map(|r| r.len()).unwrap_or(0) == 2 {
        vec!["field".into(), "value".into()]
    } else {
        vec!["value".into()]
    };
    let n = rows.len();
    Ok((cols, rows, format!("{n} value(s)")))
}

fn flatten_value(v: &redis::Value) -> Vec<Vec<String>> {
    use redis::Value::*;
    match v {
        Nil => vec![vec!["(nil)".into()]],
        Int(i) => vec![vec![i.to_string()]],
        BulkString(b) => vec![vec![String::from_utf8_lossy(b).into_owned()]],
        SimpleString(s) => vec![vec![s.clone()]],
        Array(items) => {
            // pair-wise for HGETALL-like
            if items.len() >= 2 && items.len() % 2 == 0 {
                let mut out = Vec::new();
                let mut i = 0;
                while i + 1 < items.len() {
                    let k = value_one(&items[i]);
                    let val = value_one(&items[i + 1]);
                    out.push(vec![k, val]);
                    i += 2;
                }
                // if looks like list of scalars (odd structure already handled), also OK
                if out.iter().all(|r| r.len() == 2) {
                    return out;
                }
            }
            items.iter().map(|x| vec![value_one(x)]).collect()
        }
        Map(pairs) => pairs
            .iter()
            .map(|(k, v)| vec![value_one(k), value_one(v)])
            .collect(),
        Set(items) => items.iter().map(|x| vec![value_one(x)]).collect(),
        Double(d) => vec![vec![d.to_string()]],
        Boolean(b) => vec![vec![b.to_string()]],
        BigNumber(b) => vec![vec![format!("{b:?}")]],
        Attribute { data, .. } => flatten_value(data),
        Push { data, .. } => data.iter().map(|x| vec![value_one(x)]).collect(),
        ServerError(e) => vec![vec![format!("ERR {e:?}")]],
        Okay => vec![vec!["OK".into()]],
        VerbatimString { text, .. } => vec![vec![text.clone()]],
    }
}

fn value_one(v: &redis::Value) -> String {
    flatten_value(v)
        .into_iter()
        .next()
        .and_then(|r| r.into_iter().next())
        .unwrap_or_default()
}

fn shell_split(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_q: Option<char> = None;
    for ch in s.chars() {
        match in_q {
            Some(q) if ch == q => in_q = None,
            Some(_) => cur.push(ch),
            None if ch == '"' || ch == '\'' => in_q = Some(ch),
            None if ch.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            None => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
