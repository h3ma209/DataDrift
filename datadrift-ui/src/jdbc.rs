use crate::model::{DataSource, DbVendor};

/// DataGrip-style URL assembly (Grip documents JDBC URLs; runtime uses native drivers).
pub fn build_url(ds: &DataSource) -> String {
    match ds.vendor {
        DbVendor::PostgreSQL => format!(
            "jdbc:postgresql://{}:{}/{}",
            ds.host, ds.port, ds.database
        ),
        DbVendor::MySQL => format!("jdbc:mysql://{}:{}/{}", ds.host, ds.port, ds.database),
        DbVendor::MariaDB => format!("jdbc:mariadb://{}:{}/{}", ds.host, ds.port, ds.database),
        DbVendor::Sqlite => {
            if ds.database.is_empty() || ds.database == ":memory:" {
                "jdbc:sqlite::memory:".into()
            } else {
                format!("jdbc:sqlite:{}", ds.database)
            }
        }
        DbVendor::ClickHouse => format!(
            "jdbc:clickhouse://{}:{}/{}",
            ds.host, ds.port, ds.database
        ),
        DbVendor::Cassandra => format!(
            "jdbc:cassandra://{}:{}/{}",
            ds.host, ds.port, ds.database
        ),
        DbVendor::MongoDB => {
            if ds.user.is_empty() {
                format!("mongodb://{}:{}/{}", ds.host, ds.port, ds.database)
            } else {
                format!(
                    "mongodb://{}@{}:{}/{}",
                    ds.user, ds.host, ds.port, ds.database
                )
            }
        }
        DbVendor::Redis => format!("jdbc:redis://{}:{}/{}", ds.host, ds.port, ds.database),
        DbVendor::Redshift => format!(
            "jdbc:redshift://{}:{}/{}",
            ds.host, ds.port, ds.database
        ),
    }
}

pub fn parse_and_validate(url: &str) -> Result<ParsedJdbc, String> {
    let lower = url.to_lowercase();
    if lower.starts_with("mongodb://") || lower.starts_with("mongodb+srv://") {
        return parse_mongo(url);
    }
    if lower.starts_with("redis://") {
        return parse_redis_native(url);
    }
    if !lower.starts_with("jdbc:") {
        return Err("URL must start with jdbc:, mongodb:, or redis:".into());
    }
    let rest = &url[5..];
    let (scheme, path) = rest
        .split_once(':')
        .ok_or("Missing scheme after jdbc:")?;
    match scheme {
        "postgresql" | "postgres" => parse_host_db(url, DbVendor::PostgreSQL, path),
        "mysql" => parse_host_db(url, DbVendor::MySQL, path),
        "mariadb" => parse_host_db(url, DbVendor::MariaDB, path),
        "clickhouse" | "ch" => parse_host_db(url, DbVendor::ClickHouse, path),
        "cassandra" | "scylla" => parse_host_db(url, DbVendor::Cassandra, path),
        "redis" | "redis:cluster" => parse_host_db(url, DbVendor::Redis, path),
        "redshift" => parse_host_db(url, DbVendor::Redshift, path),
        "sqlite" => {
            let db = path.trim_start_matches("//");
            Ok(ParsedJdbc {
                vendor: DbVendor::Sqlite,
                host: String::new(),
                port: 0,
                database: db.to_string(),
                raw: url.to_string(),
            })
        }
        other => Err(format!("Unsupported JDBC subprotocol: {other}")),
    }
}

#[derive(Clone, Debug)]
pub struct ParsedJdbc {
    pub vendor: DbVendor,
    pub host: String,
    pub port: u16,
    pub database: String,
    #[allow(dead_code)]
    pub raw: String,
}

fn parse_host_db(full: &str, vendor: DbVendor, path: &str) -> Result<ParsedJdbc, String> {
    let without_scheme = path.trim_start_matches("//");
    let (authority, db) = without_scheme
        .split_once('/')
        .map(|(a, d)| (a, d))
        .unwrap_or((without_scheme, ""));
    // strip user:pass@ if present (redis jdbc form)
    let authority = authority.rsplit_once('@').map(|(_, h)| h).unwrap_or(authority);
    let (host, port) = if authority.contains(':') {
        let (h, p) = authority.rsplit_once(':').ok_or("Invalid host:port")?;
        let port: u16 = p.parse().map_err(|_| "Invalid port")?;
        (h.to_string(), port)
    } else {
        (authority.to_string(), vendor.default_port())
    };
    let database = db.split('?').next().unwrap_or(db).to_string();
    let database = if database.is_empty() && matches!(vendor, DbVendor::Redis) {
        "0".into()
    } else {
        database
    };
    Ok(ParsedJdbc {
        vendor,
        host,
        port,
        database,
        raw: full.to_string(),
    })
}

fn parse_mongo(full: &str) -> Result<ParsedJdbc, String> {
    let rest = full
        .strip_prefix("mongodb://")
        .or_else(|| full.strip_prefix("mongodb+srv://"))
        .ok_or("bad mongo url")?;
    let after_auth = rest.rsplit_once('@').map(|(_, h)| h).unwrap_or(rest);
    let (authority, db) = after_auth
        .split_once('/')
        .map(|(a, d)| (a, d.split('?').next().unwrap_or(d)))
        .unwrap_or((after_auth, "admin"));
    let (host, port) = if authority.contains(':') {
        let (h, p) = authority.rsplit_once(':').unwrap();
        (
            h.to_string(),
            p.parse().unwrap_or(DbVendor::MongoDB.default_port()),
        )
    } else {
        (authority.to_string(), DbVendor::MongoDB.default_port())
    };
    Ok(ParsedJdbc {
        vendor: DbVendor::MongoDB,
        host,
        port,
        database: db.to_string(),
        raw: full.to_string(),
    })
}

fn parse_redis_native(full: &str) -> Result<ParsedJdbc, String> {
    // redis://[:password@]host:port/db
    let rest = full.strip_prefix("redis://").ok_or("bad redis url")?;
    let after_auth = rest.rsplit_once('@').map(|(_, h)| h).unwrap_or(rest);
    let (authority, db) = after_auth
        .split_once('/')
        .map(|(a, d)| (a, d.split('?').next().unwrap_or(d)))
        .unwrap_or((after_auth, "0"));
    let (host, port) = if authority.contains(':') {
        let (h, p) = authority.rsplit_once(':').unwrap();
        (
            h.to_string(),
            p.parse().unwrap_or(DbVendor::Redis.default_port()),
        )
    } else {
        (authority.to_string(), DbVendor::Redis.default_port())
    };
    Ok(ParsedJdbc {
        vendor: DbVendor::Redis,
        host,
        port,
        database: if db.is_empty() {
            "0".into()
        } else {
            db.to_string()
        },
        raw: full.to_string(),
    })
}

pub fn validation_errors(ds: &DataSource) -> Vec<String> {
    let mut v = Vec::new();
    if ds.name.trim().is_empty() {
        v.push("Name is required".into());
    }
    if ds.use_url {
        if let Err(e) = parse_and_validate(&ds.jdbc_url) {
            v.push(e);
        }
    } else {
        match ds.vendor {
            DbVendor::Sqlite => {
                if ds.database.trim().is_empty() {
                    v.push("Database file path or :memory: required".into());
                }
            }
            DbVendor::Cassandra | DbVendor::Redis => {
                if ds.host.trim().is_empty() {
                    v.push("Host is required".into());
                }
                if ds.port == 0 {
                    v.push("Port is required".into());
                }
            }
            _ => {
                if ds.host.trim().is_empty() {
                    v.push("Host is required".into());
                }
                if ds.port == 0 {
                    v.push("Port is required".into());
                }
                if ds.database.trim().is_empty() {
                    v.push(format!("{} name is required", ds.vendor.database_label()));
                }
            }
        }
    }
    v
}
