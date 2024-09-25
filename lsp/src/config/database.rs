use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub namespace: String,
    pub database: String,
    pub user: String,
    pub pass: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            namespace: "namespace".to_string(),
            database: "database".to_string(),
            user: "root".to_string(),
            pass: "root".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct DatabaseConfigFromFile {
    namespace: Option<String>,
    database: Option<String>,
    user: Option<String>,
    pass: Option<String>,
}

impl Into<DatabaseConfig> for DatabaseConfigFromFile {
    fn into(self) -> DatabaseConfig {
        DatabaseConfig {
            namespace: self.namespace.unwrap_or_else(|| {
                let val = "default_namespace";
                warn!("namespace not provided, defaulting to: {}", val);
                val.into()
            }),
            database: self.database.unwrap_or_else(|| {
                let val = "default_database";
                warn!("database not provided, defaulting to: {}", val);
                val.into()
            }),
            user: self.user.unwrap_or_else(|| {
                let val = "root";
                warn!("user not provided, defaulting to: {}", val);
                val.into()
            }),
            pass: self.pass.unwrap_or_else(|| {
                let val = "root";
                warn!("pass not provided, defaulting to: {}", val);
                val.into()
            }),
        }
    }
}
