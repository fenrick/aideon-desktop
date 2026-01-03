use aideon_mneme::core::WorkerHealth;
use aideon_mneme::store::MnemeConfig;

#[test]
fn reexports_core_and_store_types() {
    let health = WorkerHealth::healthy(123);
    assert!(health.message.is_none());

    let config = MnemeConfig::default_sqlite("memory");
    match config.database {
        aideon_mneme::store::DatabaseConfig::Sqlite { .. } => {}
        _ => panic!("expected sqlite config"),
    }
}
