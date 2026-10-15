use crate::model::{DataSource, DbVendor};

/// Mirrors DataGrip-style URL assembly (simplified; no driver-specific params yet).
pub fn build_url(ds: &DataSource) -> String {
    match ds.vendor {
        DbVendor::PostgreSQL => format!(
            "jdbc:postgresql://{}:{}/{}",
            ds.host, ds.port, ds.database
        ),
        DbVendor::MySQL => format!(
            "jdbc:mysql://{}:{}/{}",
            ds.host, ds.port, ds.database
        ),
        DbVendor::Sqlite => {
            if ds.database.is_empty() || ds.database == ":memory:" {
                "jdbc:sqlite::memory:".into()
            } else {
                format!("jdbc:sqlite:{}", ds.database)
            }
        }
    }
}

pub fn parse_and_validate(url: &str) -> Result<ParsedJdbc, String> {
    let lower = url.to_lowercase();
    if !lower.starts_with("jdbc:") {
        return Err("JDBC URL must start with jdbc:".into());
    }
    let rest = &url[5..];
    let (scheme, path) = rest
        .split_once(':')
        .ok_or("Missing scheme after jdbc:")?;
    match scheme {
        "postgresql" | "postgres" => parse_pg_mysql(url, DbVendor::PostgreSQL, path),
        "mysql" => parse_pg_mysql(url, DbVendor::MySQL, path),
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
    pub raw: String,
}

fn parse_pg_mysql(full: &str, vendor: DbVendor, path: &str) -> Result<ParsedJdbc, String> {
    let without_scheme = path.trim_start_matches("//");
    let (authority, db) = without_scheme
        .split_once('/')
        .ok_or("Expected //host:port/database")?;
    let (host, port) = if authority.contains(':') {
        let (h, p) = authority
            .rsplit_once(':')
            .ok_or("Invalid host:port")?;
        let port: u16 = p.parse().map_err(|_| "Invalid port")?;
        (h.to_string(), port)
    } else {
        (authority.to_string(), vendor.default_port())
    };
    Ok(ParsedJdbc {
        vendor,
        host,
        port,
        database: db.split('?').next().unwrap_or(db).to_string(),
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
            _ => {
                if ds.host.trim().is_empty() {
                    v.push("Host is required".into());
                }
                if ds.port == 0 {
                    v.push("Port is required".into());
                }
                if ds.database.trim().is_empty() {
                    v.push("Database name is required".into());
                }
            }
        }
    }
    v
}
