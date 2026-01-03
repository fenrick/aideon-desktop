use sea_orm_migration::prelude::*;

mod m20250101_000001_init;
mod m20251229_000002_ops_scenario;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250101_000001_init::Migration),
            Box::new(m20251229_000002_ops_scenario::Migration),
        ]
    }
}
