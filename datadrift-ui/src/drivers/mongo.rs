//! MongoDB via official Rust driver.

use crate::async_rt;
use crate::connection::{query_result_pub, WorkerResponse};
use crate::jdbc::ParsedJdbc;
use crate::model::{DataSource, SchemaNode};
use futures_util::TryStreamExt;
use mongodb::bson::{doc, Bson, Document};
use mongodb::options::ClientOptions;
use mongodb::Client;

async fn client(ds: &DataSource, p: &ParsedJdbc) -> Result<Client, String> {
    let uri = if ds.user.is_empty() {
        format!("mongodb://{}:{}", p.host, p.port)
    } else {
        let pass = urlencoding_simple(&ds.password);
        format!(
            "mongodb://{}:{}@{}:{}",
            ds.user, pass, p.host, p.port
        )
    };
    let mut opts = ClientOptions::parse(&uri)
        .await
        .map_err(|e| e.to_string())?;
    opts.server_selection_timeout = Some(std::time::Duration::from_secs(5));
    Client::with_options(opts).map_err(|e| e.to_string())
}

fn urlencoding_simple(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn test(ds: &DataSource, p: &ParsedJdbc) -> Result<String, String> {
    async_rt::block_on(async {
        let c = client(ds, p).await?;
        let db = c.database(if p.database.is_empty() {
            "admin"
        } else {
            &p.database
        });
        let doc = db.run_command(doc! { "buildInfo": 1 }).await.map_err(|e| e.to_string())?;
        let ver = doc
            .get_str("version")
            .unwrap_or("?")
            .to_string();
        Ok(format!("MongoDB {ver}"))
    })
}

pub fn introspect(ds: &DataSource, p: &ParsedJdbc) -> Result<(Vec<SchemaNode>, usize), String> {
    async_rt::block_on(async {
        let c = client(ds, p).await?;
        let names = c.list_database_names().await.map_err(|e| e.to_string())?;
        let mut schemas = Vec::new();
        for name in names.into_iter().take(40) {
            if name == "local" || name == "config" {
                continue;
            }
            let cols = c
                .database(&name)
                .list_collection_names()
                .await
                .unwrap_or_default();
            schemas.push(SchemaNode {
                name,
                children: cols.into_iter().take(200).collect(),
            });
        }
        let total = schemas.len();
        Ok((schemas, total))
    })
}

pub fn query(ds: &DataSource, p: &ParsedJdbc, text: &str, tab_id: u64) -> WorkerResponse {
    // DataDrift console for Mongo: JSON find filter on current DB, or `db.collection.find({})` lite.
    // Also accept `{...}` as filter on database.collection from `database` field path `db.coll`.
    async_rt::block_on(async {
        let c = match client(ds, p).await {
            Ok(c) => c,
            Err(e) => return query_result_pub(tab_id, false, e, vec![], vec![]),
        };
        let (db_name, coll_name, filter) = match parse_mongo_console(text, &p.database) {
            Ok(v) => v,
            Err(e) => return query_result_pub(tab_id, false, e, vec![], vec![]),
        };
        let coll = c.database(&db_name).collection::<Document>(&coll_name);
        let cursor = match coll.find(filter).await {
            Ok(cur) => cur,
            Err(e) => return query_result_pub(tab_id, false, e.to_string(), vec![], vec![]),
        };
        let docs: Vec<Document> = match cursor.try_collect().await {
            Ok(d) => d,
            Err(e) => return query_result_pub(tab_id, false, e.to_string(), vec![], vec![]),
        };
        let docs: Vec<_> = docs.into_iter().take(200).collect();
        let mut colset = std::collections::BTreeSet::new();
        for d in &docs {
            for k in d.keys() {
                colset.insert(k.clone());
            }
        }
        let cols: Vec<String> = colset.into_iter().collect();
        let mut rows = Vec::new();
        for d in &docs {
            let mut line = Vec::new();
            for c in &cols {
                line.push(bson_cell(d.get(c)));
            }
            rows.push(line);
        }
        query_result_pub(
            tab_id,
            true,
            format!("{} doc(s)", rows.len()),
            cols,
            rows,
        )
    })
}

fn parse_mongo_console(text: &str, default_db: &str) -> Result<(String, String, Document), String> {
    let t = text.trim();
    // db.users.find({...})
    if let Some(rest) = t.strip_prefix("db.") {
        let (coll, after) = rest
            .split_once('.')
            .ok_or("expected db.<collection>.find(...)")?;
        let after = after.trim_start();
        if let Some(args) = after.strip_prefix("find(") {
            let args = args.trim_end_matches(';').trim_end();
            let args = args.strip_suffix(')').unwrap_or(args);
            let filter = if args.trim().is_empty() {
                doc! {}
            } else {
                mongodb::bson::from_slice(args.as_bytes())
                    .or_else(|_| serde_json::from_str::<serde_json::Value>(args).map_err(|e| e.to_string()).and_then(|v| {
                        mongodb::bson::to_document(&v).map_err(|e| e.to_string())
                    }))
                    .map_err(|e| format!("filter JSON: {e}"))?
            };
            let db = if default_db.is_empty() {
                "admin".into()
            } else {
                default_db.to_string()
            };
            return Ok((db, coll.to_string(), filter));
        }
    }
    // collection|filter JSON
    if let Some((coll, json)) = t.split_once('|') {
        let filter: Document = serde_json::from_str(json.trim())
            .map_err(|e| e.to_string())
            .and_then(|v: serde_json::Value| {
                mongodb::bson::to_document(&v).map_err(|e| e.to_string())
            })?;
        return Ok((default_db.to_string(), coll.trim().to_string(), filter));
    }
    // bare JSON filter → need collection in database field as db.collection
    if t.starts_with('{') {
        let (db, coll) = default_db
            .split_once('.')
            .ok_or("set Database to db.collection or use db.coll.find({})")?;
        let filter: Document = serde_json::from_str(t)
            .map_err(|e| e.to_string())
            .and_then(|v: serde_json::Value| {
                mongodb::bson::to_document(&v).map_err(|e| e.to_string())
            })?;
        return Ok((db.to_string(), coll.to_string(), filter));
    }
    Err(
        "Mongo console: db.<coll>.find({}), or coll|{json}, or Database=db.coll + {json} filter"
            .into(),
    )
}

fn bson_cell(v: Option<&Bson>) -> String {
    match v {
        None => "NULL".into(),
        Some(Bson::String(s)) => s.clone(),
        Some(Bson::ObjectId(oid)) => oid.to_hex(),
        Some(Bson::Document(d)) => d.to_string(),
        Some(Bson::Array(a)) => format!("{a:?}"),
        Some(other) => other.to_string(),
    }
}
