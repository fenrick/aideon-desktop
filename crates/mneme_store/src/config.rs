use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use aideon_mneme_core::{MnemeError, MnemeResult};

const DEFAULT_CONFIG_NAME: &str = "mneme.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "lowercase")]
pub enum DatabaseConfig {
    Sqlite { path: Option<String> },
    Postgres { url: String },
    Mysql { url: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolConfig {
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub connect_timeout_ms: Option<u64>,
    pub acquire_timeout_ms: Option<u64>,
    pub idle_timeout_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub max_op_payload_bytes: Option<usize>,
    pub max_blob_bytes: Option<usize>,
    pub max_mv_values: Option<usize>,
    pub max_pending_jobs: Option<u32>,
    pub max_ingest_batch: Option<usize>,
}

impl LimitsConfig {
    pub fn with_defaults() -> Self {
        Self {
            max_op_payload_bytes: Some(1_048_576),
            max_blob_bytes: Some(4_194_304),
            max_mv_values: Some(100),
            max_pending_jobs: Some(10_000),
            max_ingest_batch: Some(5_000),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrityConfig {
    pub record_overlap_warnings: Option<bool>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValidationMode {
    Off,
    Warn,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MnemeConfig {
    pub database: DatabaseConfig,
    pub pool: Option<PoolConfig>,
    pub limits: Option<LimitsConfig>,
    pub integrity: Option<IntegrityConfig>,
    pub validation_mode: Option<ValidationMode>,
    pub failpoints: Option<Vec<String>>,
}

impl MnemeConfig {
    pub fn default_sqlite(path: impl Into<String>) -> Self {
        Self {
            database: DatabaseConfig::Sqlite {
                path: Some(path.into()),
            },
            pool: None,
            limits: Some(LimitsConfig::with_defaults()),
            integrity: Some(IntegrityConfig {
                record_overlap_warnings: Some(true),
            }),
            validation_mode: Some(ValidationMode::Error),
            failpoints: None,
        }
    }

    pub fn load_or_init(base_dir: &Path, default_sqlite_path: &Path) -> MnemeResult<Self> {
        fs::create_dir_all(base_dir)
            .map_err(|err| MnemeError::storage(format!("create config dir: {err}")))?;
        let config_path = base_dir.join(DEFAULT_CONFIG_NAME);
        if config_path.exists() {
            let raw = fs::read_to_string(&config_path)
                .map_err(|err| MnemeError::storage(format!("read config: {err}")))?;
            let config: MnemeConfig =
                serde_json::from_str(&raw).map_err(|err| MnemeError::invalid(err.to_string()))?;
            return Ok(config);
        }
        let default = MnemeConfig::default_sqlite(default_sqlite_path.to_string_lossy());
        let payload = serde_json::to_string_pretty(&default)
            .map_err(|err| MnemeError::storage(format!("serialize config: {err}")))?;
        fs::write(&config_path, payload)
            .map_err(|err| MnemeError::storage(format!("write config: {err}")))?;
        Ok(default)
    }

    pub fn sqlite_path(&self, base_dir: &Path) -> MnemeResult<PathBuf> {
        match &self.database {
            DatabaseConfig::Sqlite { path } => {
                let path = path.clone().unwrap_or_else(|| "praxis.sqlite".to_string());
                let candidate = PathBuf::from(path);
                if candidate.is_absolute() {
                    Ok(candidate)
                } else {
                    Ok(base_dir.join(candidate))
                }
            }
            _ => Err(MnemeError::invalid("config is not sqlite backend")),
        }
    }

    pub fn backend_name(&self) -> &'static str {
        match self.database {
            DatabaseConfig::Sqlite { .. } => "sqlite",
            DatabaseConfig::Postgres { .. } => "postgres",
            DatabaseConfig::Mysql { .. } => "mysql",
        }
    }

    pub fn connection_url(&self) -> Option<&str> {
        match &self.database {
            DatabaseConfig::Sqlite { .. } => None,
            DatabaseConfig::Postgres { url } | DatabaseConfig::Mysql { url } => Some(url.as_str()),
        }
    }
}
