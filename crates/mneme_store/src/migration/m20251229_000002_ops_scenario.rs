use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

use crate::db::AideonOps;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        manager
            .alter_table(
                Table::alter()
                    .table(AideonOps::Table)
                    .add_column(id_col(backend, AideonOps::ScenarioId, true))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AideonOps::Table)
                    .drop_column(AideonOps::ScenarioId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

fn id_col(backend: DatabaseBackend, col: impl Iden, nullable: bool) -> ColumnDef {
    let mut col_def = ColumnDef::new(col);
    match backend {
        DatabaseBackend::Postgres => {
            col_def.uuid();
        }
        DatabaseBackend::MySql => {
            col_def.binary_len(16);
        }
        DatabaseBackend::Sqlite => {
            col_def.string_len(36);
        }
        _ => {
            col_def.string_len(36);
        }
    }
    if nullable {
        col_def.null();
    } else {
        col_def.not_null();
    }
    col_def.to_owned()
}
