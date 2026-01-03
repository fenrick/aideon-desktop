pub mod config;
pub mod datastore;
mod db;
pub mod migration;
pub mod schema_manifest;
pub mod store;

pub mod api {
    pub use aideon_mneme_core::api::*;
}

pub mod ops {
    pub use aideon_mneme_core::ops::*;
}

pub mod schema {
    pub use aideon_mneme_core::schema::*;
}

pub mod value {
    pub use aideon_mneme_core::value::*;
}

pub use aideon_mneme_core::*;
pub use config::{
    DatabaseConfig, IntegrityConfig, LimitsConfig, MnemeConfig, PoolConfig, ValidationMode,
};
pub use datastore::{
    create_datastore, datastore_path, default_sqlite_path, load_or_init_config, open_store,
};
pub use schema_manifest::load_schema_manifest;
pub use store::{BackendCapabilities, MnemeStore};
