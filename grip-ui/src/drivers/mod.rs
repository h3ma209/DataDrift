//! Native Rust drivers (no JVM). Mirrors DataGrip vendor coverage subset.

mod cassandra;
mod clickhouse;
mod mongo;
mod redis;

use crate::jdbc::ParsedJdbc;
use crate::model::{DataSource, SchemaNode, VendorWire};

pub fn test(ds: &DataSource, p: &ParsedJdbc) -> Result<String, String> {
    match p.vendor.wire() {
        VendorWire::ClickHouse => clickhouse::test(ds, p),
        VendorWire::Cassandra => cassandra::test(ds, p),
        VendorWire::Mongo => mongo::test(ds, p),
        VendorWire::Redis => redis::test(ds, p),
        _ => Err("not an async-native vendor".into()),
    }
}

pub fn introspect(ds: &DataSource, p: &ParsedJdbc) -> Result<(Vec<SchemaNode>, usize), String> {
    match p.vendor.wire() {
        VendorWire::ClickHouse => clickhouse::introspect(ds, p),
        VendorWire::Cassandra => cassandra::introspect(ds, p),
        VendorWire::Mongo => mongo::introspect(ds, p),
        VendorWire::Redis => redis::introspect(ds, p),
        _ => Err("not an async-native vendor".into()),
    }
}

pub fn query(
    ds: &DataSource,
    p: &ParsedJdbc,
    sql: &str,
    tab_id: u64,
) -> crate::connection::WorkerResponse {
    match p.vendor.wire() {
        VendorWire::ClickHouse => clickhouse::query(ds, p, sql, tab_id),
        VendorWire::Cassandra => cassandra::query(ds, p, sql, tab_id),
        VendorWire::Mongo => mongo::query(ds, p, sql, tab_id),
        VendorWire::Redis => redis::query(ds, p, sql, tab_id),
        _ => crate::connection::query_result_pub(
            tab_id,
            false,
            "unsupported vendor",
            vec![],
            vec![],
        ),
    }
}
