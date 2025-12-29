use std::path::{Path, PathBuf};

use crate::{MnemeConfig, MnemeResult, MnemeStore};

const DEFAULT_DB_NAME: &str = "praxis.sqlite";

pub fn load_or_init_config(base: &Path) -> MnemeResult<MnemeConfig> {
    let default_sqlite = base.join(DEFAULT_DB_NAME);
    MnemeConfig::load_or_init(base, &default_sqlite)
}

pub async fn open_store(base: &Path) -> MnemeResult<MnemeStore> {
    let config = load_or_init_config(base)?;
    MnemeStore::connect(&config, base).await
}

pub fn default_sqlite_path(base: &Path) -> PathBuf {
    base.join(DEFAULT_DB_NAME)
}

#[cfg(test)]
mod tests {
    use super::{default_sqlite_path, load_or_init_config, open_store};
    use tempfile::tempdir;

    #[tokio::test]
    async fn opens_store_with_default_config() {
        let dir = tempdir().expect("tempdir");
        let base = dir.path();
        let config = load_or_init_config(base).expect("config");
        assert_eq!(config.backend_name(), "sqlite");
        let store = open_store(base).await.expect("open store");
        let path = default_sqlite_path(base);
        assert!(path.exists());
        let _ = store;
    }
}
