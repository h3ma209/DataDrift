# DataGrip driver URL templates → Grip native map

**Source:** `/Applications/DataGrip.app` `DatabaseTools` JARs (`databaseDrivers/*-drivers.xml`), build 2026.1.3.  
**Grip policy:** URL strings kept for UX parity; **runtime is pure Rust** (no JDBC class load).

| DataGrip driver id | Default JDBC / URL template (simplified) | Default port | Grip vendor | Rust stack |
|--------------------|------------------------------------------|--------------|-------------|------------|
| postgresql | `jdbc:postgresql://host:5432/db` | 5432 | PostgreSQL | `postgres` |
| mysql | `jdbc:mysql://host:3306/db` | 3306 | MySQL | `mysql` |
| maria | `jdbc:mariadb://host:3306/db` | 3306 | MariaDB | `mysql` (wire-compatible) |
| sqlite | `jdbc:sqlite:file` | — | SQLite | `rusqlite` |
| clickhouse | `jdbc:clickhouse://host:8123/db` | 8123 | ClickHouse | HTTP `reqwest` |
| cassandra | `jdbc:cassandra://host:9042/keyspace` | 9042 | Cassandra | `scylla` CQL |
| mongo | `mongodb://host:27017/db` / `mongodb+srv://…` | 27017 | MongoDB | `mongodb` |
| redis | `jdbc:redis://host:6379/0` | 6379 | Redis | `redis` RESP |
| redshift | `jdbc:redshift://host:5439/db` (also `jdbc:postgresql://…`) | 5439 | Redshift | `postgres` (wire) |
| duckdb | `jdbc:duckdb:{file.db}` | — | *(planned)* | — |

## Extracted XML (verbatim stubs)

Raw files under `tmp/dg-drivers/` (not embedded):

- `clickhouse-drivers.xml` — class `ru.yandex.clickhouse.ClickHouseDriver`, port **8123**
- `cassandra-drivers.xml` — keyspace path segment, port **9042**
- `redis-drivers.xml` — `jdbc:redis://` and `jdbc:redis:cluster://`, db index default **0**
- `mongo-drivers.xml` — `mongodb+srv://` + classic
- `maria-drivers.xml` — MariaDB URL family
- `duckdb-drivers.xml` — file-based `jdbc:duckdb:{file}`

## Console dialects (Grip)

| Vendor | Console language |
|--------|------------------|
| SQL family (PG/MySQL/Maria/SQLite/CH/Redshift) | SQL; `EXPLAIN` via toolbar |
| Cassandra | CQL |
| MongoDB | `db.coll.find({})` lite |
| Redis | Redis CLI (`GET`, `HGETALL`, `SCAN`, …) |

## Related

- [15-native-drivers.md](./15-native-drivers.md)
- [02-jdbc-connections.md](./02-jdbc-connections.md) — how DataGrip isolates JDBC child JVM (Grip does not)
