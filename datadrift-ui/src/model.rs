use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DbVendor {
    PostgreSQL,
    MySQL,
    MariaDB,
    Sqlite,
    ClickHouse,
    Cassandra,
    MongoDB,
    Redis,
    Redshift,
}

impl DbVendor {
    pub const ALL: &[DbVendor] = &[
        Self::PostgreSQL,
        Self::MySQL,
        Self::MariaDB,
        Self::Sqlite,
        Self::ClickHouse,
        Self::Cassandra,
        Self::MongoDB,
        Self::Redis,
        Self::Redshift,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::PostgreSQL => "PostgreSQL",
            Self::MySQL => "MySQL",
            Self::MariaDB => "MariaDB",
            Self::Sqlite => "SQLite",
            Self::ClickHouse => "ClickHouse",
            Self::Cassandra => "Cassandra",
            Self::MongoDB => "MongoDB",
            Self::Redis => "Redis",
            Self::Redshift => "Redshift",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Self::PostgreSQL => 5432,
            Self::MySQL | Self::MariaDB => 3306,
            Self::Sqlite => 0,
            Self::ClickHouse => 8123, // HTTP interface (native TCP 9000 also common)
            Self::Cassandra => 9042,
            Self::MongoDB => 27017,
            Self::Redis => 6379,
            Self::Redshift => 5439,
        }
    }

    pub fn default_database(self) -> &'static str {
        match self {
            Self::PostgreSQL | Self::Redshift => "postgres",
            Self::MySQL | Self::MariaDB => "mysql",
            Self::Sqlite => ":memory:",
            Self::ClickHouse => "default",
            Self::Cassandra => "system",
            Self::MongoDB => "admin",
            Self::Redis => "0",
        }
    }

    pub fn default_user(self) -> &'static str {
        match self {
            Self::PostgreSQL | Self::Redshift => "postgres",
            Self::MySQL | Self::MariaDB => "root",
            Self::Sqlite | Self::MongoDB | Self::Redis => "",
            Self::ClickHouse => "default",
            Self::Cassandra => "cassandra",
        }
    }

    /// DataGrip JDBC driver class (documentation parity — Grip uses native Rust).
    pub fn driver_class(self) -> &'static str {
        match self {
            Self::PostgreSQL => "org.postgresql.Driver",
            Self::MySQL => "com.mysql.cj.jdbc.Driver",
            Self::MariaDB => "org.mariadb.jdbc.Driver",
            Self::Sqlite => "org.sqlite.JDBC",
            Self::ClickHouse => "com.clickhouse.jdbc.ClickHouseDriver",
            Self::Cassandra => "com.datastax.oss.driver.api.core.CqlSession",
            Self::MongoDB => "com.mongodb.jdbc.MongoDriver",
            Self::Redis => "jdbc.redis.RedisDriver",
            Self::Redshift => "com.amazon.redshift.jdbc.Driver",
        }
    }

    pub fn rust_driver(self) -> &'static str {
        match self {
            Self::PostgreSQL | Self::Redshift => "postgres",
            Self::MySQL | Self::MariaDB => "mysql",
            Self::Sqlite => "rusqlite",
            Self::ClickHouse => "clickhouse (HTTP)",
            Self::Cassandra => "scylla (CQL)",
            Self::MongoDB => "mongodb",
            Self::Redis => "redis (RESP)",
        }
    }

    pub fn uses_file_path(self) -> bool {
        matches!(self, Self::Sqlite)
    }

    /// Wire protocol family for connection worker routing.
    pub fn wire(self) -> VendorWire {
        match self {
            Self::PostgreSQL | Self::Redshift => VendorWire::Postgres,
            Self::MySQL | Self::MariaDB => VendorWire::Mysql,
            Self::Sqlite => VendorWire::Sqlite,
            Self::ClickHouse => VendorWire::ClickHouse,
            Self::Cassandra => VendorWire::Cassandra,
            Self::MongoDB => VendorWire::Mongo,
            Self::Redis => VendorWire::Redis,
        }
    }

    pub fn database_label(self) -> &'static str {
        match self {
            Self::Sqlite => "Database file (or :memory:)",
            Self::Cassandra => "Keyspace",
            Self::MongoDB => "Database",
            Self::ClickHouse | Self::Redshift => "Database",
            Self::Redis => "Database index",
            _ => "Database",
        }
    }

    pub fn object_kind(self) -> &'static str {
        match self {
            Self::MongoDB => "collections",
            Self::Cassandra => "tables",
            Self::Redis => "keys",
            _ => "tables",
        }
    }

    pub fn supports_sql_explain(self) -> bool {
        matches!(
            self,
            Self::PostgreSQL
                | Self::MySQL
                | Self::MariaDB
                | Self::Sqlite
                | Self::ClickHouse
                | Self::Redshift
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VendorWire {
    Postgres,
    Mysql,
    Sqlite,
    ClickHouse,
    Cassandra,
    Mongo,
    Redis,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataSource {
    pub id: u64,
    pub name: String,
    pub vendor: DbVendor,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    #[serde(default)]
    pub password: String,
    pub jdbc_url: String,
    pub use_url: bool,
    pub remember_password: bool,
}

impl DataSource {
    pub fn new(id: u64) -> Self {
        Self::with_vendor(id, DbVendor::PostgreSQL)
    }

    pub fn with_vendor(id: u64, vendor: DbVendor) -> Self {
        let mut ds = Self {
            id,
            name: format!("@localhost"),
            vendor,
            host: "localhost".into(),
            port: vendor.default_port(),
            database: vendor.default_database().into(),
            user: vendor.default_user().into(),
            password: String::new(),
            jdbc_url: String::new(),
            use_url: false,
            remember_password: true,
        };
        ds.sync_jdbc_url();
        ds
    }

    pub fn apply_vendor_defaults(&mut self) {
        self.port = self.vendor.default_port();
        if self.database.is_empty() || matches!(self.vendor, DbVendor::Sqlite) {
            self.database = self.vendor.default_database().into();
        }
        if self.user.is_empty() {
            self.user = self.vendor.default_user().into();
        }
        self.sync_jdbc_url();
    }

    pub fn sync_jdbc_url(&mut self) {
        if self.use_url {
            return;
        }
        self.jdbc_url = crate::jdbc::build_url(self);
    }

    /// Label shown in Database Explorer (DataGrip-style `user@host`).
    pub fn tree_label(&self) -> String {
        if !self.name.trim().is_empty() && self.name != "@localhost" {
            return self.name.clone();
        }
        match self.vendor {
            DbVendor::Sqlite => {
                if self.database.is_empty() {
                    "sqlite".into()
                } else {
                    self.database.clone()
                }
            }
            DbVendor::MongoDB | DbVendor::Redis if self.user.is_empty() => {
                format!("@{}", self.host)
            }
            _ => format!("{}@{}", self.user, self.host),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SchemaNode {
    pub name: String,
    pub children: Vec<String>,
}
