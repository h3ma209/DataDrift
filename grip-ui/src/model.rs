use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DbVendor {
    PostgreSQL,
    MySQL,
    Sqlite,
}

impl DbVendor {
    pub fn label(self) -> &'static str {
        match self {
            Self::PostgreSQL => "PostgreSQL",
            Self::MySQL => "MySQL",
            Self::Sqlite => "SQLite",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Self::PostgreSQL => 5432,
            Self::MySQL => 3306,
            Self::Sqlite => 0,
        }
    }

    pub fn driver_class(self) -> &'static str {
        match self {
            Self::PostgreSQL => "org.postgresql.Driver",
            Self::MySQL => "com.mysql.cj.jdbc.Driver",
            Self::Sqlite => "org.sqlite.JDBC",
        }
    }
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
        Self {
            id,
            name: format!("@localhost"),
            vendor: DbVendor::PostgreSQL,
            host: "localhost".into(),
            port: 5432,
            database: "postgres".into(),
            user: "postgres".into(),
            password: String::new(),
            jdbc_url: String::new(),
            use_url: false,
            remember_password: true,
        }
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
            _ => format!("{}@{}", self.user, self.host),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SchemaNode {
    pub name: String,
    pub children: Vec<String>,
}
