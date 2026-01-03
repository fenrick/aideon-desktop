use std::collections::HashSet;

use aideon_mneme_store::{MnemeConfig, MnemeResult, MnemeStore};
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
        .map_err(aideon_mneme_store::MnemeError::from)?;
    let mut tables = HashSet::new();
    for row in rows {
        let name: String = row
            .try_get("", "name")
            .map_err(aideon_mneme_store::MnemeError::from)?;
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
        "aideon_change_feed",
        "aideon_entities",
        "aideon_edges",
        "aideon_edge_exists_facts",
        "aideon_prop_fact_str",
        "aideon_prop_fact_i64",
        "aideon_prop_fact_f64",
        "aideon_prop_fact_bool",
        "aideon_prop_fact_time",
        "aideon_prop_fact_ref",
        "aideon_prop_fact_blob",
        "aideon_prop_fact_json",
        "aideon_types",
        "aideon_fields",
        "aideon_type_extends",
        "aideon_type_fields",
        "aideon_edge_type_rules",
        "aideon_effective_schema_cache",
        "aideon_type_schema_head",
        "aideon_integrity_head",
        "aideon_graph_projection_edges",
        "aideon_graph_degree_stats",
        "aideon_graph_edge_type_counts",
        "aideon_pagerank_runs",
        "aideon_pagerank_scores",
        "aideon_integrity_runs",
        "aideon_integrity_findings",
        "aideon_jobs",
        "aideon_job_events",
        "aideon_validation_rules",
        "aideon_computed_rules",
        "aideon_computed_cache_str",
        "aideon_computed_cache_i64",
        "aideon_computed_cache_f64",
        "aideon_computed_cache_bool",
        "aideon_computed_cache_time",
        "aideon_computed_cache_ref",
        "aideon_computed_cache_blob",
        "aideon_computed_cache_json",
        "aideon_metamodel_versions",
        "aideon_partitions",
        "aideon_actors",
        "aideon_hlc_state",
        "aideon_idx_field_str",
        "aideon_idx_field_i64",
        "aideon_idx_field_f64",
        "aideon_idx_field_bool",
        "aideon_idx_field_time",
        "aideon_idx_field_ref",
    ] {
        assert!(tables.contains(table), "expected table '{table}' to exist");
    }
    // Idempotency check.
    let _store = MnemeStore::connect(&config, base).await?;
    Ok(())
}
