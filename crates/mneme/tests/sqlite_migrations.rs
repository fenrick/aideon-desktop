use std::collections::HashSet;

use aideon_mneme::{MnemeConfig, MnemeResult, MnemeStore};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tempfile::tempdir;

async fn list_tables(store: &MnemeStore) -> MnemeResult<HashSet<String>> {
    let rows = store
        .connection()
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type = 'table'",
        ))
        .await
        .map_err(aideon_mneme::MnemeError::from)?;
    let mut tables = HashSet::new();
    for row in rows {
        let name: String = row
            .try_get("", "name")
            .map_err(aideon_mneme::MnemeError::from)?;
        tables.insert(name);
    }
    Ok(tables)
}

#[tokio::test]
async fn sqlite_migrations_create_core_tables() -> MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let tables = list_tables(&store).await?;
    for table in [
        "aideon_schema_version",
        "aideon_ops",
        "aideon_op_deps",
        "aideon_entities",
        "aideon_edges",
        "aideon_edge_exists_facts",
        "aideon_prop_fact_str",
        "aideon_types",
        "aideon_fields",
        "aideon_graph_projection_edges",
        "aideon_pagerank_runs",
        "aideon_pagerank_scores",
    ] {
        assert!(tables.contains(table), "expected table '{table}' to exist");
    }
    // Idempotency check.
    let _store = MnemeStore::connect(&config, base).await?;
    Ok(())
}
