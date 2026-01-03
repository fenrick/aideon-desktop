use aideon_mneme_store::{DatabaseConfig, MnemeConfig, PoolConfig};
use serde_json::json;

#[test]
fn deserializes_pool_settings() {
    let payload = json!({
        "database": { "backend": "sqlite", "path": "data.sqlite" },
        "pool": {
            "max_connections": 20,
            "min_connections": 2,
            "connect_timeout_ms": 1000,
            "acquire_timeout_ms": 500,
            "idle_timeout_ms": 60000
        }
    });
    let config: MnemeConfig = serde_json::from_value(payload).expect("config");
    match config.database {
        DatabaseConfig::Sqlite { path } => {
            assert_eq!(path.as_deref(), Some("data.sqlite"));
        }
        _ => panic!("expected sqlite backend"),
    }
    let pool = config.pool.expect("pool");
    assert_eq!(pool.max_connections, Some(20));
    assert_eq!(pool.min_connections, Some(2));
    assert_eq!(pool.connect_timeout_ms, Some(1000));
    assert_eq!(pool.acquire_timeout_ms, Some(500));
    assert_eq!(pool.idle_timeout_ms, Some(60000));
}

#[test]
fn default_sqlite_sets_no_pool() {
    let config = MnemeConfig::default_sqlite("mneme.sqlite");
    match config.database {
        DatabaseConfig::Sqlite { .. } => {}
        _ => panic!("expected sqlite backend"),
    }
    assert!(config.pool.is_none());
}

#[test]
fn pool_config_roundtrip() {
    let config = MnemeConfig {
        database: DatabaseConfig::Postgres {
            url: "postgres://user:pass@localhost/db".to_string(),
        },
        pool: Some(PoolConfig {
            max_connections: Some(5),
            min_connections: None,
            connect_timeout_ms: None,
            acquire_timeout_ms: Some(2000),
            idle_timeout_ms: None,
        }),
        limits: None,
        integrity: None,
        validation_mode: None,
        failpoints: None,
    };
    let encoded = serde_json::to_string(&config).expect("encode");
    let decoded: MnemeConfig = serde_json::from_str(&encoded).expect("decode");
    assert_eq!(decoded.pool.unwrap().acquire_timeout_ms, Some(2000));
}
