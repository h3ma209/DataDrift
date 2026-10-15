# Grip native drivers (pure Rust)

**No JVM / JDBC runtime.** DataGrip JDBC class names are kept only as documentation labels in the Drivers tab.

| Vendor | Default port | Rust stack | Introspection |
|--------|--------------|------------|---------------|
| PostgreSQL | 5432 | `postgres` | `information_schema` |
| MySQL | 3306 | `mysql` | `information_schema` |
| MariaDB | 3306 | `mysql` (wire-compatible) | `information_schema` |
| SQLite | file | `rusqlite` | `sqlite_master` |
| ClickHouse | 8123 (HTTP) | `reqwest` HTTP | `system.tables` |
| Cassandra / Scylla | 9042 | `scylla` CQL | `system_schema.tables` |
| MongoDB | 27017 | `mongodb` | `listDatabases` + collections |
| Redis | 6379 | `redis` RESP | `SCAN` keys |
| Redshift | 5439 | `postgres` (wire) | `information_schema` |

URL template map extracted from DataGrip JARs: [16-datagrip-driver-url-templates.md](./16-datagrip-driver-url-templates.md).

## Console notes

- **SQL vendors:** statement under caret (`⌘/Ctrl+Enter`), `;` delimited; **Explain** toolbar for plan.
- **MongoDB:** `db.<collection>.find({})` or `collection|{json}` or Database=`db.coll` + `{json}` filter.
- **Cassandra:** CQL (`SELECT * FROM ks.table LIMIT 100`).
- **Redis:** CLI-style (`PING`, `GET key`, `HGETALL key`, `KEYS *`).

## Query history & export

- Executions append to `~/.grip/query_history.json` (last 200) — Services → **History** (guide §7.6).
- Result grids: **Export CSV** → clipboard (guide §7 / §10).

## Related

- [14-datagrip-reference-guide.md](./14-datagrip-reference-guide.md) — UX map  
- [02-jdbc-connections.md](./02-jdbc-connections.md) — how DataGrip isolates JDBC (Grip does not ship that path)
