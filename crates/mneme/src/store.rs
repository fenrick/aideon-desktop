use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sea_orm::sea_query;
use sea_orm::sea_query::{
    Alias, Expr, ExprTrait, Func, MysqlQueryBuilder, OnConflict, Order, PostgresQueryBuilder,
    Query, QueryStatementWriter, SqliteQueryBuilder, Value as SeaValue,
};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, QueryResult,
    Statement, TransactionTrait,
};
use uuid::Uuid;
use unicode_normalization::UnicodeNormalization;

use crate::api::{
    AnalyticsApi, AnalyticsResultsApi, ChangeEvent, ChangeFeedApi, CreateScenarioInput, DiagnosticsApi,
    Direction, ExportOptions, ExportOpsInput, ExportRecord, GraphReadApi, GraphWriteApi, ImportOptions,
    ImportReport, JobRecord, JobSummary, MetamodelApi, MnemeExportApi, MnemeImportApi,
    MnemeProcessingApi, MnemeSnapshotApi, ProjectionEdge, PropertyWriteApi, RunWorkerInput,
    ScenarioApi, SnapshotOptions, SyncApi, TriggerProcessingInput, ValidationRule, ValidationRulesApi,
};
use crate::db::*;
use crate::migration::Migrator;
use crate::ops::{
    ClearPropIntervalInput, CounterUpdateInput, CreateEdgeInput, CreateNodeInput, OpPayload,
    OrSetUpdateInput, SetOp, SetPropIntervalInput,
};
use crate::schema::{EffectiveField, EffectiveSchema, MetamodelBatch, SchemaVersion};
use crate::value::{EntityKind, Layer, MergePolicy, Value, ValueType};
use crate::{
    ActorId, CompareOp, FieldFilter, Hlc, Id, ListEntitiesInput, ListEntitiesResultItem,
    MnemeConfig, MnemeError, MnemeResult, OpEnvelope, OpId, PartitionId, ReadEntityAtTimeInput,
    ReadEntityAtTimeResult, ReadValue, ScenarioId, TraverseAtTimeInput, TraverseEdgeItem, ValidTime,
};
use sea_orm_migration::MigratorTrait;

#[derive(Clone)]
pub struct MnemeStore {
    conn: DatabaseConnection,
    backend: DatabaseBackend,
}

#[derive(Clone, Copy, Debug)]
pub struct BackendCapabilities {
    pub transactional_ddl: bool,
    pub partial_indexes: bool,
    pub computed_columns: bool,
    pub json_types: bool,
}

impl MnemeStore {
    pub async fn connect(config: &MnemeConfig, base_dir: &Path) -> MnemeResult<Self> {
        let url = build_connection_url(config, base_dir)?;
        let mut options = ConnectOptions::new(url);
        if let Some(pool) = &config.pool {
            if let Some(max) = pool.max_connections {
                options.max_connections(max);
            }
            if let Some(min) = pool.min_connections {
                options.min_connections(min);
            }
            if let Some(timeout_ms) = pool.connect_timeout_ms {
                options.connect_timeout(Duration::from_millis(timeout_ms));
            }
            if let Some(timeout_ms) = pool.acquire_timeout_ms {
                options.acquire_timeout(Duration::from_millis(timeout_ms));
            }
            if let Some(timeout_ms) = pool.idle_timeout_ms {
                options.idle_timeout(Duration::from_millis(timeout_ms));
            }
        }
        let conn = Database::connect(options).await.map_err(MnemeError::from)?;
        let backend = conn.get_database_backend();
        let store = Self { conn, backend };
        Migrator::up(&store.conn, None)
            .await
            .map_err(MnemeError::from)?;
        Ok(store)
    }

    pub async fn connect_sqlite(path: &Path) -> MnemeResult<Self> {
        let config = MnemeConfig::default_sqlite(path.to_string_lossy());
        Self::connect(&config, path.parent().unwrap_or_else(|| Path::new("."))).await
    }

    pub fn connection(&self) -> &DatabaseConnection {
        &self.conn
    }

    pub fn capabilities(&self) -> BackendCapabilities {
        match self.backend {
            DatabaseBackend::Sqlite => BackendCapabilities {
                transactional_ddl: false,
                partial_indexes: true,
                computed_columns: true,
                json_types: false,
            },
            DatabaseBackend::Postgres => BackendCapabilities {
                transactional_ddl: true,
                partial_indexes: true,
                computed_columns: true,
                json_types: true,
            },
            DatabaseBackend::MySql => BackendCapabilities {
                transactional_ddl: false,
                partial_indexes: false,
                computed_columns: true,
                json_types: true,
            },
            _ => BackendCapabilities {
                transactional_ddl: false,
                partial_indexes: false,
                computed_columns: false,
                json_types: false,
            },
        }
    }

    async fn ensure_partition(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        actor: ActorId,
    ) -> MnemeResult<()> {
        let partition_id = id_value(self.backend, partition.0);
        let actor_id = id_value(self.backend, actor.0);
        let asserted_at = Hlc::now().as_i64();

        let insert_partitions = Query::insert()
            .into_table(AideonPartitions::Table)
            .columns([
                AideonPartitions::PartitionId,
                AideonPartitions::CreatedAtAsserted,
                AideonPartitions::CreatedByActor,
            ])
            .values_panic([
                partition_id.clone().into(),
                asserted_at.into(),
                actor_id.clone().into(),
            ])
            .on_conflict(
                OnConflict::column(AideonPartitions::PartitionId)
                    .do_nothing()
                    .to_owned(),
            )
            .to_owned();
        exec(tx, &insert_partitions).await?;

        let insert_actor = Query::insert()
            .into_table(AideonActors::Table)
            .columns([
                AideonActors::PartitionId,
                AideonActors::ActorId,
                AideonActors::MetadataJson,
                AideonActors::CreatedAtAsserted,
            ])
            .values_panic([
                partition_id.into(),
                actor_id.into(),
                SeaValue::String(None).into(),
                asserted_at.into(),
            ])
            .on_conflict(
                OnConflict::columns([AideonActors::PartitionId, AideonActors::ActorId])
                    .do_nothing()
                    .to_owned(),
            )
            .to_owned();
        exec(tx, &insert_actor).await?;
        Ok(())
    }

    async fn bump_hlc_state(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        asserted_at: Hlc,
    ) -> MnemeResult<()> {
        let select = Query::select()
            .from(AideonHlcState::Table)
            .column(AideonHlcState::LastHlc)
            .and_where(Expr::col(AideonHlcState::PartitionId).eq(id_value(self.backend, partition.0)))
            .limit(1)
            .to_owned();
        let row = query_one(tx, &select).await?;
        let asserted_i64 = asserted_at.as_i64();
        match row {
            Some(row) => {
                let current: i64 = row.try_get("", &col_name(AideonHlcState::LastHlc))?;
                if asserted_i64 > current {
                    let update = Query::update()
                        .table(AideonHlcState::Table)
                        .values([(AideonHlcState::LastHlc, asserted_i64.into())])
                        .and_where(
                            Expr::col(AideonHlcState::PartitionId)
                                .eq(id_value(self.backend, partition.0)),
                        )
                        .to_owned();
                    exec(tx, &update).await?;
                }
            }
            None => {
                let insert = Query::insert()
                    .into_table(AideonHlcState::Table)
                    .columns([AideonHlcState::PartitionId, AideonHlcState::LastHlc])
                    .values_panic([
                        id_value(self.backend, partition.0).into(),
                        asserted_i64.into(),
                    ])
                    .to_owned();
                exec(tx, &insert).await?;
            }
        }
        Ok(())
    }

    async fn insert_op(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        actor: ActorId,
        asserted_at: Hlc,
        payload: &OpPayload,
    ) -> MnemeResult<(OpId, Hlc, Vec<u8>, u16)> {
        let op_id = OpId(Id::new());
        let op_type = payload.op_type() as u16;
        let payload_bytes =
            serde_json::to_vec(payload).map_err(|err| MnemeError::storage(err.to_string()))?;
        self.bump_hlc_state(tx, partition, asserted_at).await?;
        let insert = Query::insert()
            .into_table(AideonOps::Table)
            .columns([
                AideonOps::PartitionId,
                AideonOps::OpId,
                AideonOps::ActorId,
                AideonOps::AssertedAtHlc,
                AideonOps::TxId,
                AideonOps::OpType,
                AideonOps::Payload,
                AideonOps::SchemaVersionHint,
                AideonOps::IngestedAssertedAtHlc,
            ])
            .values_panic([
                id_value(self.backend, partition.0).into(),
                id_value(self.backend, op_id.0).into(),
                id_value(self.backend, actor.0).into(),
                asserted_at.as_i64().into(),
                none_id_value(self.backend).into(),
                (op_type as i64).into(),
                payload_bytes.clone().into(),
                SeaValue::String(None).into(),
                SeaValue::BigInt(None).into(),
            ])
            .to_owned();
        exec(tx, &insert).await?;
        self.append_change_feed(tx, partition, op_id, asserted_at, payload)
            .await?;
        self.enqueue_jobs_for_op(tx, partition, payload).await?;
        Ok((op_id, asserted_at, payload_bytes, op_type))
    }

    fn layer_value(layer: Layer) -> i64 {
        layer as i64
    }

    async fn append_change_feed(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        op_id: OpId,
        asserted_at: Hlc,
        payload: &OpPayload,
    ) -> MnemeResult<()> {
        let (change_kind, entity_id, payload_json) = change_feed_payload(payload)?;
        let select = Query::select()
            .from(AideonChangeFeed::Table)
            .expr_as(Func::max(Expr::col(AideonChangeFeed::Sequence)), Alias::new("max_seq"))
            .and_where(
                Expr::col(AideonChangeFeed::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        let row = query_one(tx, &select).await?;
        let next_seq = match row {
            Some(row) => {
                let max_seq: Option<i64> = row.try_get("", "max_seq")?;
                max_seq.unwrap_or(0) + 1
            }
            None => 1,
        };
        let insert = Query::insert()
            .into_table(AideonChangeFeed::Table)
            .columns([
                AideonChangeFeed::PartitionId,
                AideonChangeFeed::Sequence,
                AideonChangeFeed::OpId,
                AideonChangeFeed::AssertedAtHlc,
                AideonChangeFeed::EntityId,
                AideonChangeFeed::ChangeKind,
                AideonChangeFeed::PayloadJson,
            ])
            .values_panic([
                id_value(self.backend, partition.0).into(),
                next_seq.into(),
                id_value(self.backend, op_id.0).into(),
                asserted_at.as_i64().into(),
                opt_id_value(self.backend, entity_id).into(),
                (change_kind as i64).into(),
                payload_json.map(|value| value.to_string()).into(),
            ])
            .to_owned();
        exec(tx, &insert).await?;
        Ok(())
    }

    async fn enqueue_jobs_for_op(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        payload: &OpPayload,
    ) -> MnemeResult<()> {
        match payload {
            OpPayload::UpsertMetamodelBatch(_) => {
                let job_payload = TriggerJobPayload {
                    partition_id: partition,
                    scenario_id: None,
                    type_id: None,
                    schema_version_hint: None,
                    reason: "metamodel_upsert".to_string(),
                };
                let payload = serde_json::to_vec(&job_payload)
                    .map_err(|err| MnemeError::storage(err.to_string()))?;
                let dedupe_key = format!("schema:{}:all", partition.0.to_uuid_string());
                self.enqueue_job(
                    tx,
                    partition,
                    JOB_TYPE_SCHEMA_REBUILD,
                    payload,
                    0,
                    Some(dedupe_key),
                )
                .await?;
            }
            OpPayload::CreateEdge(input) => {
                let job_payload = TriggerJobPayload {
                    partition_id: input.partition,
                    scenario_id: input.scenario_id,
                    type_id: None,
                    schema_version_hint: None,
                    reason: "edge_change".to_string(),
                };
                let payload = serde_json::to_vec(&job_payload)
                    .map_err(|err| MnemeError::storage(err.to_string()))?;
                let scenario_key = input
                    .scenario_id
                    .map(|s| s.0.to_uuid_string())
                    .unwrap_or_else(|| "baseline".to_string());
                let dedupe_key = format!(
                    "analytics:{}:{}",
                    input.partition.0.to_uuid_string(),
                    scenario_key
                );
                self.enqueue_job(
                    tx,
                    input.partition,
                    JOB_TYPE_ANALYTICS_REFRESH,
                    payload,
                    0,
                    Some(dedupe_key),
                )
                .await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn export_fact_table(
        &self,
        records: &mut Vec<ExportRecord>,
        opts: &SnapshotOptions,
        table: impl sea_query::Iden + Clone,
        entity_col: impl sea_query::Iden + Clone,
        field_col: impl sea_query::Iden + Clone,
        valid_from_col: impl sea_query::Iden + Clone,
        valid_to_col: impl sea_query::Iden + Clone,
        layer_col: impl sea_query::Iden + Clone,
        asserted_col: impl sea_query::Iden + Clone,
        tombstone_col: impl sea_query::Iden + Clone,
        value_col: impl sea_query::Iden + Clone,
        record_type: &str,
    ) -> MnemeResult<()> {
        let mut select = Query::select()
            .from(table.clone())
            .column(entity_col.clone())
            .column(field_col.clone())
            .column(valid_from_col.clone())
            .column(valid_to_col.clone())
            .column(layer_col.clone())
            .column(asserted_col.clone())
            .column(tombstone_col.clone())
            .column(value_col.clone())
            .and_where(
                Expr::col((table.clone(), AideonPropFactStr::PartitionId))
                    .eq(id_value(self.backend, opts.partition_id.0)),
            )
            .and_where(
                Expr::col((table.clone(), AideonPropFactStr::AssertedAtHlc))
                    .lte(opts.as_of_asserted_at.as_i64()),
            )
            .to_owned();
        if let Some(scenario_id) = opts.scenario_id {
            select.and_where(
                Expr::col((table.clone(), AideonPropFactStr::ScenarioId))
                    .eq(id_value(self.backend, scenario_id.0)),
            );
        } else {
            select.and_where(Expr::col((table.clone(), AideonPropFactStr::ScenarioId)).is_null());
        }
        let rows = query_all(&self.conn, &select).await?;
        for row in rows {
            let entity_id = read_id_by_name(&row, &col_name(entity_col.clone()))?;
            let field_id = read_id_by_name(&row, &col_name(field_col.clone()))?;
            let valid_from: i64 = row.try_get("", &col_name(valid_from_col.clone()))?;
            let valid_to: Option<i64> = row.try_get("", &col_name(valid_to_col.clone()))?;
            let layer: i64 = row.try_get("", &col_name(layer_col.clone()))?;
            let asserted_at = read_hlc_by_name(&row, &col_name(asserted_col.clone()))?;
            let is_tombstone: bool = row.try_get("", &col_name(tombstone_col.clone()))?;
            let value = match record_type {
                "snapshot_fact_str" => {
                    let value: String = row.try_get("", &col_name(value_col.clone()))?;
                    serde_json::Value::String(value)
                }
                "snapshot_fact_i64" => {
                    let value: i64 = row.try_get("", &col_name(value_col.clone()))?;
                    serde_json::Value::Number(value.into())
                }
                "snapshot_fact_f64" => {
                    let value: f64 = row.try_get("", &col_name(value_col.clone()))?;
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(value)
                            .ok_or_else(|| MnemeError::storage("invalid f64"))?,
                    )
                }
                "snapshot_fact_bool" => {
                    let value: bool = row.try_get("", &col_name(value_col.clone()))?;
                    serde_json::Value::Bool(value)
                }
                "snapshot_fact_time" => {
                    let value: i64 = row.try_get("", &col_name(value_col.clone()))?;
                    serde_json::Value::Number(value.into())
                }
                "snapshot_fact_ref" => {
                    let value = read_id_by_name(&row, &col_name(value_col.clone()))?;
                    serde_json::Value::String(value.to_uuid_string())
                }
                "snapshot_fact_blob" => {
                    let value: Vec<u8> = row.try_get("", &col_name(value_col.clone()))?;
                    serde_json::to_value(value)
                        .map_err(|err| MnemeError::storage(err.to_string()))?
                }
                "snapshot_fact_json" => {
                    let value: String = row.try_get("", &col_name(value_col.clone()))?;
                    serde_json::from_str(&value)
                        .map_err(|err| MnemeError::storage(err.to_string()))?
                }
                _ => serde_json::Value::Null,
            };
            records.push(ExportRecord {
                record_type: record_type.to_string(),
                data: serde_json::json!({
                    "entity_id": entity_id,
                    "field_id": field_id,
                    "valid_from": valid_from,
                    "valid_to": valid_to,
                    "layer": layer,
                    "asserted_at": asserted_at.as_i64(),
                    "is_tombstone": is_tombstone,
                    "value": value,
                }),
            });
        }
        Ok(())
    }

    async fn import_fact_record(
        &self,
        opts: &ImportOptions,
        record: &ExportRecord,
    ) -> MnemeResult<()> {
        let entity_id = Id::from_uuid_str(
            record
                .data
                .get("entity_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MnemeError::invalid("missing entity_id"))?,
        )?;
        let field_id = Id::from_uuid_str(
            record
                .data
                .get("field_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MnemeError::invalid("missing field_id"))?,
        )?;
        let valid_from = record
            .data
            .get("valid_from")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| MnemeError::invalid("missing valid_from"))?;
        let valid_to = record.data.get("valid_to").and_then(|v| v.as_i64());
        let layer = record
            .data
            .get("layer")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| MnemeError::invalid("missing layer"))?;
        let asserted_at = record
            .data
            .get("asserted_at")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| MnemeError::invalid("missing asserted_at"))?;
        let is_tombstone = record
            .data
            .get("is_tombstone")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let value = record
            .data
            .get("value")
            .ok_or_else(|| MnemeError::invalid("missing value"))?;

        match record.record_type.as_str() {
            "snapshot_fact_str" => {
                let value = value.as_str().unwrap_or_default().to_string();
                let insert = Query::insert()
                    .into_table(AideonPropFactStr::Table)
                    .columns([
                        AideonPropFactStr::PartitionId,
                        AideonPropFactStr::ScenarioId,
                        AideonPropFactStr::EntityId,
                        AideonPropFactStr::FieldId,
                        AideonPropFactStr::ValidFrom,
                        AideonPropFactStr::ValidTo,
                        AideonPropFactStr::ValidBucket,
                        AideonPropFactStr::Layer,
                        AideonPropFactStr::AssertedAtHlc,
                        AideonPropFactStr::OpId,
                        AideonPropFactStr::IsTombstone,
                        AideonPropFactStr::ValueText,
                    ])
                    .values_panic([
                        id_value(self.backend, opts.target_partition.0).into(),
                        opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                        id_value(self.backend, entity_id).into(),
                        id_value(self.backend, field_id).into(),
                        valid_from.into(),
                        valid_to.into(),
                        valid_bucket(ValidTime(valid_from)).into(),
                        layer.into(),
                        asserted_at.into(),
                        none_id_value(self.backend).into(),
                        is_tombstone.into(),
                        value.into(),
                    ])
                    .to_owned();
                exec(&self.conn, &insert).await?;
            }
            "snapshot_fact_i64" => {
                let value = value.as_i64().unwrap_or_default();
                let insert = Query::insert()
                    .into_table(AideonPropFactI64::Table)
                    .columns([
                        AideonPropFactI64::PartitionId,
                        AideonPropFactI64::ScenarioId,
                        AideonPropFactI64::EntityId,
                        AideonPropFactI64::FieldId,
                        AideonPropFactI64::ValidFrom,
                        AideonPropFactI64::ValidTo,
                        AideonPropFactI64::ValidBucket,
                        AideonPropFactI64::Layer,
                        AideonPropFactI64::AssertedAtHlc,
                        AideonPropFactI64::OpId,
                        AideonPropFactI64::IsTombstone,
                        AideonPropFactI64::ValueI64,
                    ])
                    .values_panic([
                        id_value(self.backend, opts.target_partition.0).into(),
                        opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                        id_value(self.backend, entity_id).into(),
                        id_value(self.backend, field_id).into(),
                        valid_from.into(),
                        valid_to.into(),
                        valid_bucket(ValidTime(valid_from)).into(),
                        layer.into(),
                        asserted_at.into(),
                        none_id_value(self.backend).into(),
                        is_tombstone.into(),
                        value.into(),
                    ])
                    .to_owned();
                exec(&self.conn, &insert).await?;
            }
            "snapshot_fact_f64" => {
                let value = value.as_f64().unwrap_or_default();
                let insert = Query::insert()
                    .into_table(AideonPropFactF64::Table)
                    .columns([
                        AideonPropFactF64::PartitionId,
                        AideonPropFactF64::ScenarioId,
                        AideonPropFactF64::EntityId,
                        AideonPropFactF64::FieldId,
                        AideonPropFactF64::ValidFrom,
                        AideonPropFactF64::ValidTo,
                        AideonPropFactF64::ValidBucket,
                        AideonPropFactF64::Layer,
                        AideonPropFactF64::AssertedAtHlc,
                        AideonPropFactF64::OpId,
                        AideonPropFactF64::IsTombstone,
                        AideonPropFactF64::ValueF64,
                    ])
                    .values_panic([
                        id_value(self.backend, opts.target_partition.0).into(),
                        opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                        id_value(self.backend, entity_id).into(),
                        id_value(self.backend, field_id).into(),
                        valid_from.into(),
                        valid_to.into(),
                        valid_bucket(ValidTime(valid_from)).into(),
                        layer.into(),
                        asserted_at.into(),
                        none_id_value(self.backend).into(),
                        is_tombstone.into(),
                        value.into(),
                    ])
                    .to_owned();
                exec(&self.conn, &insert).await?;
            }
            "snapshot_fact_bool" => {
                let value = value.as_bool().unwrap_or(false);
                let insert = Query::insert()
                    .into_table(AideonPropFactBool::Table)
                    .columns([
                        AideonPropFactBool::PartitionId,
                        AideonPropFactBool::ScenarioId,
                        AideonPropFactBool::EntityId,
                        AideonPropFactBool::FieldId,
                        AideonPropFactBool::ValidFrom,
                        AideonPropFactBool::ValidTo,
                        AideonPropFactBool::ValidBucket,
                        AideonPropFactBool::Layer,
                        AideonPropFactBool::AssertedAtHlc,
                        AideonPropFactBool::OpId,
                        AideonPropFactBool::IsTombstone,
                        AideonPropFactBool::ValueBool,
                    ])
                    .values_panic([
                        id_value(self.backend, opts.target_partition.0).into(),
                        opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                        id_value(self.backend, entity_id).into(),
                        id_value(self.backend, field_id).into(),
                        valid_from.into(),
                        valid_to.into(),
                        valid_bucket(ValidTime(valid_from)).into(),
                        layer.into(),
                        asserted_at.into(),
                        none_id_value(self.backend).into(),
                        is_tombstone.into(),
                        value.into(),
                    ])
                    .to_owned();
                exec(&self.conn, &insert).await?;
            }
            "snapshot_fact_time" => {
                let value = value.as_i64().unwrap_or_default();
                let insert = Query::insert()
                    .into_table(AideonPropFactTime::Table)
                    .columns([
                        AideonPropFactTime::PartitionId,
                        AideonPropFactTime::ScenarioId,
                        AideonPropFactTime::EntityId,
                        AideonPropFactTime::FieldId,
                        AideonPropFactTime::ValidFrom,
                        AideonPropFactTime::ValidTo,
                        AideonPropFactTime::ValidBucket,
                        AideonPropFactTime::Layer,
                        AideonPropFactTime::AssertedAtHlc,
                        AideonPropFactTime::OpId,
                        AideonPropFactTime::IsTombstone,
                        AideonPropFactTime::ValueTime,
                    ])
                    .values_panic([
                        id_value(self.backend, opts.target_partition.0).into(),
                        opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                        id_value(self.backend, entity_id).into(),
                        id_value(self.backend, field_id).into(),
                        valid_from.into(),
                        valid_to.into(),
                        valid_bucket(ValidTime(valid_from)).into(),
                        layer.into(),
                        asserted_at.into(),
                        none_id_value(self.backend).into(),
                        is_tombstone.into(),
                        value.into(),
                    ])
                    .to_owned();
                exec(&self.conn, &insert).await?;
            }
            "snapshot_fact_ref" => {
                let id = Id::from_uuid_str(value.as_str().unwrap_or_default())?;
                let insert = Query::insert()
                    .into_table(AideonPropFactRef::Table)
                    .columns([
                        AideonPropFactRef::PartitionId,
                        AideonPropFactRef::ScenarioId,
                        AideonPropFactRef::EntityId,
                        AideonPropFactRef::FieldId,
                        AideonPropFactRef::ValidFrom,
                        AideonPropFactRef::ValidTo,
                        AideonPropFactRef::ValidBucket,
                        AideonPropFactRef::Layer,
                        AideonPropFactRef::AssertedAtHlc,
                        AideonPropFactRef::OpId,
                        AideonPropFactRef::IsTombstone,
                        AideonPropFactRef::ValueRefEntityId,
                    ])
                    .values_panic([
                        id_value(self.backend, opts.target_partition.0).into(),
                        opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                        id_value(self.backend, entity_id).into(),
                        id_value(self.backend, field_id).into(),
                        valid_from.into(),
                        valid_to.into(),
                        valid_bucket(ValidTime(valid_from)).into(),
                        layer.into(),
                        asserted_at.into(),
                        none_id_value(self.backend).into(),
                        is_tombstone.into(),
                        id_value(self.backend, id).into(),
                    ])
                    .to_owned();
                exec(&self.conn, &insert).await?;
            }
            "snapshot_fact_blob" => {
                let bytes = match value {
                    serde_json::Value::Array(items) => items
                        .iter()
                        .filter_map(|v| v.as_u64())
                        .map(|v| v as u8)
                        .collect::<Vec<u8>>(),
                    _ => Vec::new(),
                };
                let insert = Query::insert()
                    .into_table(AideonPropFactBlob::Table)
                    .columns([
                        AideonPropFactBlob::PartitionId,
                        AideonPropFactBlob::ScenarioId,
                        AideonPropFactBlob::EntityId,
                        AideonPropFactBlob::FieldId,
                        AideonPropFactBlob::ValidFrom,
                        AideonPropFactBlob::ValidTo,
                        AideonPropFactBlob::ValidBucket,
                        AideonPropFactBlob::Layer,
                        AideonPropFactBlob::AssertedAtHlc,
                        AideonPropFactBlob::OpId,
                        AideonPropFactBlob::IsTombstone,
                        AideonPropFactBlob::ValueBlob,
                    ])
                    .values_panic([
                        id_value(self.backend, opts.target_partition.0).into(),
                        opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                        id_value(self.backend, entity_id).into(),
                        id_value(self.backend, field_id).into(),
                        valid_from.into(),
                        valid_to.into(),
                        valid_bucket(ValidTime(valid_from)).into(),
                        layer.into(),
                        asserted_at.into(),
                        none_id_value(self.backend).into(),
                        is_tombstone.into(),
                        bytes.into(),
                    ])
                    .to_owned();
                exec(&self.conn, &insert).await?;
            }
            "snapshot_fact_json" => {
                let insert = Query::insert()
                    .into_table(AideonPropFactJson::Table)
                    .columns([
                        AideonPropFactJson::PartitionId,
                        AideonPropFactJson::ScenarioId,
                        AideonPropFactJson::EntityId,
                        AideonPropFactJson::FieldId,
                        AideonPropFactJson::ValidFrom,
                        AideonPropFactJson::ValidTo,
                        AideonPropFactJson::ValidBucket,
                        AideonPropFactJson::Layer,
                        AideonPropFactJson::AssertedAtHlc,
                        AideonPropFactJson::OpId,
                        AideonPropFactJson::IsTombstone,
                        AideonPropFactJson::ValueJson,
                    ])
                    .values_panic([
                        id_value(self.backend, opts.target_partition.0).into(),
                        opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                        id_value(self.backend, entity_id).into(),
                        id_value(self.backend, field_id).into(),
                        valid_from.into(),
                        valid_to.into(),
                        valid_bucket(ValidTime(valid_from)).into(),
                        layer.into(),
                        asserted_at.into(),
                        none_id_value(self.backend).into(),
                        is_tombstone.into(),
                        value.to_string().into(),
                    ])
                    .to_owned();
                exec(&self.conn, &insert).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn read_entity_row_in_partition(
        &self,
        partition: PartitionId,
        scenario_id: Option<crate::ScenarioId>,
        entity_id: Id,
    ) -> MnemeResult<Option<(EntityKind, Option<Id>, bool)>> {
        let mut select = Query::select()
            .from(AideonEntities::Table)
            .columns([
                AideonEntities::EntityKind,
                AideonEntities::TypeId,
                AideonEntities::IsDeleted,
            ])
            .and_where(
                Expr::col(AideonEntities::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .and_where(Expr::col(AideonEntities::EntityId).eq(id_value(self.backend, entity_id)))
            .to_owned();
        if let Some(scenario_id) = scenario_id {
            select.and_where(
                Expr::col(AideonEntities::ScenarioId).eq(id_value(self.backend, scenario_id.0)),
            );
        } else {
            select.and_where(Expr::col(AideonEntities::ScenarioId).is_null());
        }
        let row = query_one(&self.conn, &select).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let kind_raw: i16 = row.try_get("", &col_name(AideonEntities::EntityKind))?;
        let type_id = read_opt_id(&row, AideonEntities::TypeId)?;
        let is_deleted: bool = row.try_get("", &col_name(AideonEntities::IsDeleted))?;
        let kind = EntityKind::from_i16(kind_raw)
            .ok_or_else(|| MnemeError::storage("invalid entity kind"))?;
        Ok(Some((kind, type_id, is_deleted)))
    }

    async fn read_entity_row_with_fallback(
        &self,
        partition: PartitionId,
        scenario_id: Option<crate::ScenarioId>,
        entity_id: Id,
    ) -> MnemeResult<(PartitionId, EntityKind, Option<Id>, bool)> {
        if let Some(scenario_id) = scenario_id {
            if let Some((kind, type_id, is_deleted)) = self
                .read_entity_row_in_partition(partition, Some(scenario_id), entity_id)
                .await?
            {
                return Ok((partition, kind, type_id, is_deleted));
            }
        }
        if let Some((kind, type_id, is_deleted)) = self
            .read_entity_row_in_partition(partition, None, entity_id)
            .await?
        {
            return Ok((partition, kind, type_id, is_deleted));
        }
        Err(MnemeError::not_found("entity not found"))
    }

    async fn fetch_field_def<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        field_id: Id,
    ) -> MnemeResult<(ValueType, MergePolicy, bool, bool)> {
        self.fetch_field_def_in_partition(conn, partition, field_id)
            .await?
            .ok_or_else(|| MnemeError::not_found("field not found"))
    }

    async fn fetch_field_def_in_partition<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        field_id: Id,
    ) -> MnemeResult<Option<(ValueType, MergePolicy, bool, bool)>> {
        let select = Query::select()
            .from(AideonFields::Table)
            .columns([
                AideonFields::ValueType,
                AideonFields::MergePolicy,
                AideonFields::Cardinality,
                AideonFields::IsIndexed,
            ])
            .and_where(Expr::col(AideonFields::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(Expr::col(AideonFields::FieldId).eq(id_value(self.backend, field_id)))
            .and_where(Expr::col(AideonFields::IsDeleted).eq(false))
            .to_owned();
        let row = query_one(conn, &select).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let value_type_raw: i16 = row.try_get("", &col_name(AideonFields::ValueType))?;
        let merge_policy_raw: i16 = row.try_get("", &col_name(AideonFields::MergePolicy))?;
        let cardinality_raw: i16 = row.try_get("", &col_name(AideonFields::Cardinality))?;
        let is_indexed: bool = row.try_get("", &col_name(AideonFields::IsIndexed))?;
        let value_type = ValueType::from_i16(value_type_raw)
            .ok_or_else(|| MnemeError::storage("invalid value type"))?;
        let merge_policy = MergePolicy::from_i16(merge_policy_raw)
            .ok_or_else(|| MnemeError::storage("invalid merge policy"))?;
        Ok(Some((
            value_type,
            merge_policy,
            cardinality_raw == 2,
            is_indexed,
        )))
    }
}

const MICROS_PER_DAY: i64 = 86_400_000_000;

fn valid_bucket(valid_from: ValidTime) -> i64 {
    valid_from.0 / MICROS_PER_DAY
}

const JOB_BACKOFF_BASE_MS: i64 = 1_000;
const JOB_BACKOFF_MAX_MS: i64 = 60_000;

fn job_backoff_millis(attempts: i32) -> i64 {
    if attempts <= 0 {
        return JOB_BACKOFF_BASE_MS;
    }
    let shift = std::cmp::min(attempts.saturating_sub(1), 10) as u32;
    let value = JOB_BACKOFF_BASE_MS.saturating_mul(1_i64 << shift);
    std::cmp::min(value, JOB_BACKOFF_MAX_MS)
}

fn normalize_index_text(value: &str) -> String {
    value.trim().nfc().collect::<String>().to_lowercase()
}
#[async_trait]
impl MetamodelApi for MnemeStore {
    async fn upsert_metamodel_batch(
        &self,
        partition: PartitionId,
        actor: ActorId,
        asserted_at: Hlc,
        batch: MetamodelBatch,
    ) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, partition, actor).await?;
        let payload = OpPayload::UpsertMetamodelBatch(batch.clone());
        let (_op_id, _asserted_at, _payload, _op_type) = self
            .insert_op(&tx, partition, actor, asserted_at, &payload)
            .await?;

        for t in batch.types {
            let insert = Query::insert()
                .into_table(AideonTypes::Table)
                .columns([
                    AideonTypes::PartitionId,
                    AideonTypes::TypeId,
                    AideonTypes::AppliesTo,
                    AideonTypes::Label,
                    AideonTypes::IsAbstract,
                    AideonTypes::IsDeleted,
                    AideonTypes::UpdatedAssertedAtHlc,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    id_value(self.backend, t.type_id).into(),
                    (t.applies_to.as_i16() as i64).into(),
                    t.label.clone().into(),
                    t.is_abstract.into(),
                    false.into(),
                    asserted_at.as_i64().into(),
                ])
                .on_conflict(
                    OnConflict::columns([AideonTypes::PartitionId, AideonTypes::TypeId])
                        .update_columns([
                            AideonTypes::AppliesTo,
                            AideonTypes::Label,
                            AideonTypes::IsAbstract,
                            AideonTypes::IsDeleted,
                            AideonTypes::UpdatedAssertedAtHlc,
                        ])
                        .to_owned(),
                )
                .to_owned();
            exec(&tx, &insert).await?;

            if let Some(parent) = t.parent_type_id {
                let insert_extends = Query::insert()
                    .into_table(AideonTypeExtends::Table)
                    .columns([
                        AideonTypeExtends::PartitionId,
                        AideonTypeExtends::TypeId,
                        AideonTypeExtends::ParentTypeId,
                    ])
                    .values_panic([
                        id_value(self.backend, partition.0).into(),
                        id_value(self.backend, t.type_id).into(),
                        id_value(self.backend, parent).into(),
                    ])
                    .on_conflict(
                        OnConflict::columns([
                            AideonTypeExtends::PartitionId,
                            AideonTypeExtends::TypeId,
                        ])
                        .update_columns([AideonTypeExtends::ParentTypeId])
                        .to_owned(),
                    )
                    .to_owned();
                exec(&tx, &insert_extends).await?;
            }
        }

        for f in batch.fields {
            let insert = Query::insert()
                .into_table(AideonFields::Table)
                .columns([
                    AideonFields::PartitionId,
                    AideonFields::FieldId,
                    AideonFields::ValueType,
                    AideonFields::Cardinality,
                    AideonFields::MergePolicy,
                    AideonFields::IsIndexed,
                    AideonFields::Label,
                    AideonFields::IsDeleted,
                    AideonFields::UpdatedAssertedAtHlc,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    id_value(self.backend, f.field_id).into(),
                    (f.value_type.as_i16() as i64).into(),
                    if f.cardinality_multi { 2i64 } else { 1i64 }.into(),
                    (f.merge_policy.as_i16() as i64).into(),
                    f.is_indexed.into(),
                    f.label.clone().into(),
                    false.into(),
                    asserted_at.as_i64().into(),
                ])
                .on_conflict(
                    OnConflict::columns([AideonFields::PartitionId, AideonFields::FieldId])
                        .update_columns([
                            AideonFields::ValueType,
                            AideonFields::Cardinality,
                            AideonFields::MergePolicy,
                            AideonFields::IsIndexed,
                            AideonFields::Label,
                            AideonFields::IsDeleted,
                            AideonFields::UpdatedAssertedAtHlc,
                        ])
                        .to_owned(),
                )
                .to_owned();
            exec(&tx, &insert).await?;
        }

        for tf in batch.type_fields {
            let default_kind = tf
                .default_value
                .as_ref()
                .map(|value| value.value_type().as_i16());
            let defaults = default_value_columns(self.backend, &tf.default_value);
            let insert = Query::insert()
                .into_table(AideonTypeFields::Table)
                .columns([
                    AideonTypeFields::PartitionId,
                    AideonTypeFields::TypeId,
                    AideonTypeFields::FieldId,
                    AideonTypeFields::IsRequired,
                    AideonTypeFields::DefaultValueKind,
                    AideonTypeFields::DefaultValueStr,
                    AideonTypeFields::DefaultValueI64,
                    AideonTypeFields::DefaultValueF64,
                    AideonTypeFields::DefaultValueBool,
                    AideonTypeFields::DefaultValueTime,
                    AideonTypeFields::DefaultValueRef,
                    AideonTypeFields::DefaultValueBlob,
                    AideonTypeFields::DefaultValueJson,
                    AideonTypeFields::OverrideDefault,
                    AideonTypeFields::TightenRequired,
                    AideonTypeFields::UpdatedAssertedAtHlc,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    id_value(self.backend, tf.type_id).into(),
                    id_value(self.backend, tf.field_id).into(),
                    tf.is_required.into(),
                    default_kind.map(|value| value as i64).into(),
                    defaults.str_value.into(),
                    defaults.i64_value.into(),
                    defaults.f64_value.into(),
                    defaults.bool_value.into(),
                    defaults.time_value.into(),
                    defaults.ref_value.into(),
                    defaults.blob_value.into(),
                    defaults.json_value.into(),
                    tf.override_default.into(),
                    tf.tighten_required.into(),
                    asserted_at.as_i64().into(),
                ])
                .on_conflict(
                    OnConflict::columns([
                        AideonTypeFields::PartitionId,
                        AideonTypeFields::TypeId,
                        AideonTypeFields::FieldId,
                    ])
                    .update_columns([
                        AideonTypeFields::IsRequired,
                        AideonTypeFields::DefaultValueKind,
                        AideonTypeFields::DefaultValueStr,
                        AideonTypeFields::DefaultValueI64,
                        AideonTypeFields::DefaultValueF64,
                        AideonTypeFields::DefaultValueBool,
                        AideonTypeFields::DefaultValueTime,
                        AideonTypeFields::DefaultValueRef,
                        AideonTypeFields::DefaultValueBlob,
                        AideonTypeFields::DefaultValueJson,
                        AideonTypeFields::OverrideDefault,
                        AideonTypeFields::TightenRequired,
                        AideonTypeFields::UpdatedAssertedAtHlc,
                    ])
                    .to_owned(),
                )
                .to_owned();
            exec(&tx, &insert).await?;
        }

        for rule in batch.edge_type_rules {
            let src_json = serde_json::to_string(&rule.allowed_src_type_ids)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let dst_json = serde_json::to_string(&rule.allowed_dst_type_ids)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let insert = Query::insert()
                .into_table(AideonEdgeTypeRules::Table)
                .columns([
                    AideonEdgeTypeRules::PartitionId,
                    AideonEdgeTypeRules::EdgeTypeId,
                    AideonEdgeTypeRules::AllowedSrcTypeIdsJson,
                    AideonEdgeTypeRules::AllowedDstTypeIdsJson,
                    AideonEdgeTypeRules::SemanticDirection,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    id_value(self.backend, rule.edge_type_id).into(),
                    src_json.into(),
                    dst_json.into(),
                    rule.semantic_direction.clone().into(),
                ])
                .on_conflict(
                    OnConflict::columns([
                        AideonEdgeTypeRules::PartitionId,
                        AideonEdgeTypeRules::EdgeTypeId,
                    ])
                    .update_columns([
                        AideonEdgeTypeRules::AllowedSrcTypeIdsJson,
                        AideonEdgeTypeRules::AllowedDstTypeIdsJson,
                        AideonEdgeTypeRules::SemanticDirection,
                    ])
                    .to_owned(),
                )
                .to_owned();
            exec(&tx, &insert).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn compile_effective_schema(
        &self,
        partition: PartitionId,
        _actor: ActorId,
        asserted_at: Hlc,
        type_id: Id,
    ) -> MnemeResult<SchemaVersion> {
        let schema = self.build_effective_schema(partition, type_id).await?;
        let payload =
            serde_json::to_vec(&schema).map_err(|err| MnemeError::storage(err.to_string()))?;
        let hash = blake3::hash(&payload).to_hex().to_string();
        let insert = Query::insert()
            .into_table(AideonEffectiveSchemaCache::Table)
            .columns([
                AideonEffectiveSchemaCache::PartitionId,
                AideonEffectiveSchemaCache::TypeId,
                AideonEffectiveSchemaCache::SchemaVersionHash,
                AideonEffectiveSchemaCache::Blob,
                AideonEffectiveSchemaCache::BuiltAssertedAtHlc,
            ])
            .values_panic([
                id_value(self.backend, partition.0).into(),
                id_value(self.backend, type_id).into(),
                hash.clone().into(),
                payload.into(),
                asserted_at.as_i64().into(),
            ])
            .to_owned();
        exec(&self.conn, &insert).await?;
        let update_head = Query::insert()
            .into_table(AideonTypeSchemaHead::Table)
            .columns([
                AideonTypeSchemaHead::PartitionId,
                AideonTypeSchemaHead::TypeId,
                AideonTypeSchemaHead::SchemaVersionHash,
                AideonTypeSchemaHead::UpdatedAssertedAtHlc,
            ])
            .values_panic([
                id_value(self.backend, partition.0).into(),
                id_value(self.backend, type_id).into(),
                hash.clone().into(),
                asserted_at.as_i64().into(),
            ])
            .on_conflict(
                OnConflict::columns([
                    AideonTypeSchemaHead::PartitionId,
                    AideonTypeSchemaHead::TypeId,
                ])
                .update_columns([
                    AideonTypeSchemaHead::SchemaVersionHash,
                    AideonTypeSchemaHead::UpdatedAssertedAtHlc,
                ])
                .to_owned(),
            )
            .to_owned();
        exec(&self.conn, &update_head).await?;
        Ok(SchemaVersion {
            schema_version_hash: hash,
        })
    }

    async fn get_effective_schema(
        &self,
        partition: PartitionId,
        type_id: Id,
    ) -> MnemeResult<Option<EffectiveSchema>> {
        let select = Query::select()
            .from(AideonEffectiveSchemaCache::Table)
            .column(AideonEffectiveSchemaCache::Blob)
            .and_where(
                Expr::col(AideonEffectiveSchemaCache::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
            )
            .and_where(
                Expr::col(AideonEffectiveSchemaCache::TypeId).eq(id_value(self.backend, type_id)),
            )
            .order_by(AideonEffectiveSchemaCache::BuiltAssertedAtHlc, Order::Desc)
            .limit(1)
            .to_owned();
        let row = query_one(&self.conn, &select).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let blob: Vec<u8> = row.try_get("", &col_name(AideonEffectiveSchemaCache::Blob))?;
        let schema: EffectiveSchema =
            serde_json::from_slice(&blob).map_err(|err| MnemeError::storage(err.to_string()))?;
        Ok(Some(schema))
    }

    async fn list_edge_type_rules(
        &self,
        partition: PartitionId,
        edge_type_id: Option<Id>,
    ) -> MnemeResult<Vec<crate::schema::EdgeTypeRule>> {
        let mut select = Query::select()
            .from(AideonEdgeTypeRules::Table)
            .columns([
                (AideonEdgeTypeRules::Table, AideonEdgeTypeRules::EdgeTypeId),
                (
                    AideonEdgeTypeRules::Table,
                    AideonEdgeTypeRules::AllowedSrcTypeIdsJson,
                ),
                (
                    AideonEdgeTypeRules::Table,
                    AideonEdgeTypeRules::AllowedDstTypeIdsJson,
                ),
                (
                    AideonEdgeTypeRules::Table,
                    AideonEdgeTypeRules::SemanticDirection,
                ),
            ])
            .and_where(
                Expr::col(AideonEdgeTypeRules::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        if let Some(edge_type_id) = edge_type_id {
            select.and_where(
                Expr::col(AideonEdgeTypeRules::EdgeTypeId)
                    .eq(id_value(self.backend, edge_type_id)),
            );
        }
        let rows = query_all(&self.conn, &select).await?;
        let mut rules = Vec::new();
        for row in rows {
            let edge_type_id = read_id(&row, AideonEdgeTypeRules::EdgeTypeId)?;
            let src_json: String =
                row.try_get("", &col_name(AideonEdgeTypeRules::AllowedSrcTypeIdsJson))?;
            let dst_json: String =
                row.try_get("", &col_name(AideonEdgeTypeRules::AllowedDstTypeIdsJson))?;
            let semantic_direction: Option<String> =
                row.try_get("", &col_name(AideonEdgeTypeRules::SemanticDirection))?;
            let allowed_src_type_ids: Vec<Id> = serde_json::from_str(&src_json)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let allowed_dst_type_ids: Vec<Id> = serde_json::from_str(&dst_json)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            rules.push(crate::schema::EdgeTypeRule {
                edge_type_id,
                allowed_src_type_ids,
                allowed_dst_type_ids,
                semantic_direction,
            });
        }
        Ok(rules)
    }
}

#[async_trait]
impl GraphWriteApi for MnemeStore {
    async fn create_node(&self, input: CreateNodeInput) -> MnemeResult<OpId> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, input.partition, input.actor)
            .await?;
        let payload = OpPayload::CreateNode(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(&tx, input.partition, input.actor, input.asserted_at, &payload)
            .await?;
        let insert_entity = Query::insert()
            .into_table(AideonEntities::Table)
            .columns([
                AideonEntities::PartitionId,
                AideonEntities::ScenarioId,
                AideonEntities::EntityId,
                AideonEntities::EntityKind,
                AideonEntities::TypeId,
                AideonEntities::IsDeleted,
                AideonEntities::CreatedOpId,
                AideonEntities::UpdatedOpId,
                AideonEntities::CreatedAssertedAtHlc,
                AideonEntities::UpdatedAssertedAtHlc,
            ])
            .values_panic([
                id_value(self.backend, input.partition.0).into(),
                opt_id_value(self.backend, input.scenario_id.map(|s| s.0)).into(),
                id_value(self.backend, input.node_id).into(),
                (EntityKind::Node.as_i16() as i64).into(),
                opt_id_value(self.backend, input.type_id).into(),
                false.into(),
                id_value(self.backend, op_id.0).into(),
                id_value(self.backend, op_id.0).into(),
                asserted_at.as_i64().into(),
                asserted_at.as_i64().into(),
            ])
            .on_conflict(
                OnConflict::columns([AideonEntities::PartitionId, AideonEntities::EntityId])
                    .do_nothing()
                    .to_owned(),
            )
            .to_owned();
        exec(&tx, &insert_entity).await?;
        tx.commit().await?;
        Ok(op_id)
    }

    async fn create_edge(&self, input: CreateEdgeInput) -> MnemeResult<OpId> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, input.partition, input.actor)
            .await?;
        let payload = OpPayload::CreateEdge(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(&tx, input.partition, input.actor, input.asserted_at, &payload)
            .await?;

        let insert_entity = Query::insert()
            .into_table(AideonEntities::Table)
            .columns([
                AideonEntities::PartitionId,
                AideonEntities::ScenarioId,
                AideonEntities::EntityId,
                AideonEntities::EntityKind,
                AideonEntities::TypeId,
                AideonEntities::IsDeleted,
                AideonEntities::CreatedOpId,
                AideonEntities::UpdatedOpId,
                AideonEntities::CreatedAssertedAtHlc,
                AideonEntities::UpdatedAssertedAtHlc,
            ])
            .values_panic([
                id_value(self.backend, input.partition.0).into(),
                opt_id_value(self.backend, input.scenario_id.map(|s| s.0)).into(),
                id_value(self.backend, input.edge_id).into(),
                (EntityKind::Edge.as_i16() as i64).into(),
                opt_id_value(self.backend, input.type_id).into(),
                false.into(),
                id_value(self.backend, op_id.0).into(),
                id_value(self.backend, op_id.0).into(),
                asserted_at.as_i64().into(),
                asserted_at.as_i64().into(),
            ])
            .on_conflict(
                OnConflict::columns([AideonEntities::PartitionId, AideonEntities::EntityId])
                    .do_nothing()
                    .to_owned(),
            )
            .to_owned();
        exec(&tx, &insert_entity).await?;

        let insert_edge = Query::insert()
            .into_table(AideonEdges::Table)
            .columns([
                AideonEdges::PartitionId,
                AideonEdges::ScenarioId,
                AideonEdges::EdgeId,
                AideonEdges::SrcEntityId,
                AideonEdges::DstEntityId,
                AideonEdges::EdgeTypeId,
            ])
            .values_panic([
                id_value(self.backend, input.partition.0).into(),
                opt_id_value(self.backend, input.scenario_id.map(|s| s.0)).into(),
                id_value(self.backend, input.edge_id).into(),
                id_value(self.backend, input.src_id).into(),
                id_value(self.backend, input.dst_id).into(),
                opt_id_value(self.backend, input.type_id).into(),
            ])
            .on_conflict(
                OnConflict::columns([AideonEdges::PartitionId, AideonEdges::EdgeId])
                    .update_columns([
                        AideonEdges::SrcEntityId,
                        AideonEdges::DstEntityId,
                        AideonEdges::EdgeTypeId,
                    ])
                    .to_owned(),
            )
            .to_owned();
        exec(&tx, &insert_edge).await?;

        let insert_exists = Query::insert()
            .into_table(AideonEdgeExistsFacts::Table)
            .columns([
                AideonEdgeExistsFacts::PartitionId,
                AideonEdgeExistsFacts::ScenarioId,
                AideonEdgeExistsFacts::EdgeId,
                AideonEdgeExistsFacts::ValidFrom,
                AideonEdgeExistsFacts::ValidTo,
                AideonEdgeExistsFacts::ValidBucket,
                AideonEdgeExistsFacts::Layer,
                AideonEdgeExistsFacts::AssertedAtHlc,
                AideonEdgeExistsFacts::OpId,
                AideonEdgeExistsFacts::IsTombstone,
            ])
            .values_panic([
                id_value(self.backend, input.partition.0).into(),
                opt_id_value(self.backend, input.scenario_id.map(|s| s.0)).into(),
                id_value(self.backend, input.edge_id).into(),
                input.exists_valid_from.0.into(),
                input.exists_valid_to.map(|v| v.0).into(),
                valid_bucket(input.exists_valid_from).into(),
                Self::layer_value(input.layer).into(),
                asserted_at.as_i64().into(),
                id_value(self.backend, op_id.0).into(),
                false.into(),
            ])
            .to_owned();
        exec(&tx, &insert_exists).await?;

        let insert_projection = Query::insert()
            .into_table(AideonGraphProjectionEdges::Table)
            .columns([
                AideonGraphProjectionEdges::PartitionId,
                AideonGraphProjectionEdges::ScenarioId,
                AideonGraphProjectionEdges::EdgeId,
                AideonGraphProjectionEdges::SrcEntityId,
                AideonGraphProjectionEdges::DstEntityId,
                AideonGraphProjectionEdges::EdgeTypeId,
                AideonGraphProjectionEdges::Weight,
                AideonGraphProjectionEdges::UpdatedAssertedAtHlc,
            ])
            .values_panic([
                id_value(self.backend, input.partition.0).into(),
                opt_id_value(self.backend, input.scenario_id.map(|s| s.0)).into(),
                id_value(self.backend, input.edge_id).into(),
                id_value(self.backend, input.src_id).into(),
                id_value(self.backend, input.dst_id).into(),
                opt_id_value(self.backend, input.type_id).into(),
                input.weight.unwrap_or(1.0).into(),
                asserted_at.as_i64().into(),
            ])
            .on_conflict(
                OnConflict::columns([
                    AideonGraphProjectionEdges::PartitionId,
                    AideonGraphProjectionEdges::EdgeId,
                ])
                .update_columns([
                    AideonGraphProjectionEdges::SrcEntityId,
                    AideonGraphProjectionEdges::DstEntityId,
                    AideonGraphProjectionEdges::EdgeTypeId,
                    AideonGraphProjectionEdges::Weight,
                    AideonGraphProjectionEdges::UpdatedAssertedAtHlc,
                ])
                .to_owned(),
            )
            .to_owned();
        exec(&tx, &insert_projection).await?;

        tx.commit().await?;
        Ok(op_id)
    }

    async fn set_edge_existence_interval(
        &self,
        input: crate::SetEdgeExistenceIntervalInput,
    ) -> MnemeResult<OpId> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, input.partition, input.actor)
            .await?;
        let payload = OpPayload::SetEdgeExistenceInterval(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(&tx, input.partition, input.actor, input.asserted_at, &payload)
            .await?;

        let insert_exists = Query::insert()
            .into_table(AideonEdgeExistsFacts::Table)
            .columns([
                AideonEdgeExistsFacts::PartitionId,
                AideonEdgeExistsFacts::ScenarioId,
                AideonEdgeExistsFacts::EdgeId,
                AideonEdgeExistsFacts::ValidFrom,
                AideonEdgeExistsFacts::ValidTo,
                AideonEdgeExistsFacts::ValidBucket,
                AideonEdgeExistsFacts::Layer,
                AideonEdgeExistsFacts::AssertedAtHlc,
                AideonEdgeExistsFacts::OpId,
                AideonEdgeExistsFacts::IsTombstone,
            ])
            .values_panic([
                id_value(self.backend, input.partition.0).into(),
                opt_id_value(self.backend, input.scenario_id.map(|s| s.0)).into(),
                id_value(self.backend, input.edge_id).into(),
                input.valid_from.0.into(),
                input.valid_to.map(|v| v.0).into(),
                valid_bucket(input.valid_from).into(),
                Self::layer_value(input.layer).into(),
                asserted_at.as_i64().into(),
                id_value(self.backend, op_id.0).into(),
                input.is_tombstone.into(),
            ])
            .to_owned();
        exec(&tx, &insert_exists).await?;

        tx.commit().await?;
        Ok(op_id)
    }

    async fn tombstone_entity(
        &self,
        partition: PartitionId,
        scenario_id: Option<crate::ScenarioId>,
        actor: ActorId,
        asserted_at: Hlc,
        entity_id: Id,
    ) -> MnemeResult<OpId> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, partition, actor).await?;
        let payload = OpPayload::TombstoneEntity {
            partition,
            scenario_id,
            actor,
            asserted_at,
            entity_id,
        };
        let (op_id, asserted_at, _payload, _op_type) =
            self.insert_op(&tx, partition, actor, asserted_at, &payload)
                .await?;

        let update = Query::update()
            .table(AideonEntities::Table)
            .values([
                (AideonEntities::IsDeleted, true.into()),
                (AideonEntities::UpdatedOpId, id_value(self.backend, op_id.0).into()),
                (
                    AideonEntities::UpdatedAssertedAtHlc,
                    asserted_at.as_i64().into(),
                ),
            ])
            .and_where(Expr::col(AideonEntities::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(Expr::col(AideonEntities::EntityId).eq(id_value(self.backend, entity_id)))
            .to_owned();
        exec(&tx, &update).await?;

        let select_kind = Query::select()
            .from(AideonEntities::Table)
            .column(AideonEntities::EntityKind)
            .and_where(Expr::col(AideonEntities::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(Expr::col(AideonEntities::EntityId).eq(id_value(self.backend, entity_id)))
            .to_owned();
        if let Some(row) = query_one(&tx, &select_kind).await? {
            let kind_raw: i16 = row.try_get("", &col_name(AideonEntities::EntityKind))?;
            if EntityKind::from_i16(kind_raw) == Some(EntityKind::Edge) {
                let insert_exists = Query::insert()
                    .into_table(AideonEdgeExistsFacts::Table)
                    .columns([
                        AideonEdgeExistsFacts::PartitionId,
                        AideonEdgeExistsFacts::ScenarioId,
                        AideonEdgeExistsFacts::EdgeId,
                        AideonEdgeExistsFacts::ValidFrom,
                        AideonEdgeExistsFacts::ValidTo,
                        AideonEdgeExistsFacts::Layer,
                        AideonEdgeExistsFacts::AssertedAtHlc,
                        AideonEdgeExistsFacts::OpId,
                        AideonEdgeExistsFacts::IsTombstone,
                    ])
                    .values_panic([
                        id_value(self.backend, partition.0).into(),
                        opt_id_value(self.backend, scenario_id.map(|s| s.0)).into(),
                        id_value(self.backend, entity_id).into(),
                        0i64.into(),
                        SeaValue::BigInt(None).into(),
                        Self::layer_value(Layer::Actual).into(),
                        asserted_at.as_i64().into(),
                        id_value(self.backend, op_id.0).into(),
                        true.into(),
                    ])
                    .to_owned();
                exec(&tx, &insert_exists).await?;

                let job_payload = TriggerJobPayload {
                    partition_id: partition,
                    scenario_id,
                    type_id: None,
                    schema_version_hint: None,
                    reason: "edge_change".to_string(),
                };
                let payload = serde_json::to_vec(&job_payload)
                    .map_err(|err| MnemeError::storage(err.to_string()))?;
                let scenario_key = scenario_id
                    .map(|s| s.0.to_uuid_string())
                    .unwrap_or_else(|| "baseline".to_string());
                let dedupe_key = format!(
                    "analytics:{}:{}",
                    partition.0.to_uuid_string(),
                    scenario_key
                );
                self.enqueue_job(
                    &tx,
                    partition,
                    JOB_TYPE_ANALYTICS_REFRESH,
                    payload,
                    0,
                    Some(dedupe_key),
                )
                .await?;
            }
        }

        let delete_projection = Query::delete()
            .from_table(AideonGraphProjectionEdges::Table)
            .and_where(
                Expr::col(AideonGraphProjectionEdges::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
            )
            .and_where(Expr::col(AideonGraphProjectionEdges::EdgeId).eq(id_value(self.backend, entity_id)))
            .to_owned();
        exec(&tx, &delete_projection).await?;

        tx.commit().await?;
        Ok(op_id)
    }
}

#[async_trait]
impl PropertyWriteApi for MnemeStore {
    async fn set_property_interval(&self, input: SetPropIntervalInput) -> MnemeResult<OpId> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, input.partition, input.actor)
            .await?;
        let (value_type, merge_policy, _multi, is_indexed) = self
            .fetch_field_def(&tx, input.partition, input.field_id)
            .await?;
        if value_type != input.value.value_type() {
            return Err(MnemeError::invalid("value type mismatch"));
        }
        if matches!(merge_policy, MergePolicy::OrSet | MergePolicy::Counter) {
            return Err(MnemeError::not_implemented(
                "use or_set_update/counter_update for this field",
            ));
        }
        let payload = OpPayload::SetProperty(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(&tx, input.partition, input.actor, input.asserted_at, &payload)
            .await?;
        insert_property_fact(
            &tx,
            input.partition,
            input.scenario_id,
            input.entity_id,
            input.field_id,
            &input.value,
            input.valid_from,
            input.valid_to,
            input.layer,
            asserted_at,
            op_id,
            false,
            self.backend,
        )
        .await?;
        if is_indexed {
            insert_index_row(
                &tx,
                input.partition,
                input.scenario_id,
                input.field_id,
                input.entity_id,
                &input.value,
                input.valid_from,
                input.valid_to,
                asserted_at,
                input.layer,
                self.backend,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(op_id)
    }

    async fn clear_property_interval(&self, input: ClearPropIntervalInput) -> MnemeResult<OpId> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, input.partition, input.actor)
            .await?;
        let (value_type, merge_policy, _multi, is_indexed) = self
            .fetch_field_def(&tx, input.partition, input.field_id)
            .await?;
        if matches!(merge_policy, MergePolicy::OrSet | MergePolicy::Counter) {
            return Err(MnemeError::not_implemented(
                "use or_set_update/counter_update for this field",
            ));
        }
        let payload = OpPayload::ClearProperty(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(&tx, input.partition, input.actor, input.asserted_at, &payload)
            .await?;
        let value = match value_type {
            ValueType::Str => Value::Str(String::new()),
            ValueType::I64 => Value::I64(0),
            ValueType::F64 => Value::F64(0.0),
            ValueType::Bool => Value::Bool(false),
            ValueType::Time => Value::Time(ValidTime(0)),
            ValueType::Ref => Value::Ref(Id::from_bytes([0u8; 16])),
            ValueType::Blob => Value::Blob(Vec::new()),
            ValueType::Json => Value::Json(serde_json::Value::Null),
        };
        insert_property_fact(
            &tx,
            input.partition,
            input.scenario_id,
            input.entity_id,
            input.field_id,
            &value,
            input.valid_from,
            input.valid_to,
            input.layer,
            asserted_at,
            op_id,
            true,
            self.backend,
        )
        .await?;
        if is_indexed {
            delete_index_row(
                &tx,
                input.partition,
                input.scenario_id,
                input.field_id,
                input.entity_id,
                &value,
                self.backend,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(op_id)
    }

    async fn or_set_update(&self, input: OrSetUpdateInput) -> MnemeResult<OpId> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, input.partition, input.actor)
            .await?;
        let (value_type, merge_policy, _multi, is_indexed) = self
            .fetch_field_def(&tx, input.partition, input.field_id)
            .await?;
        if value_type != input.element.value_type() {
            return Err(MnemeError::invalid("value type mismatch"));
        }
        if merge_policy != MergePolicy::OrSet {
            return Err(MnemeError::invalid(
                "field is not configured for OR_SET merge policy",
            ));
        }
        let payload = OpPayload::OrSetUpdate(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(&tx, input.partition, input.actor, input.asserted_at, &payload)
            .await?;
        let is_tombstone = matches!(input.op, SetOp::Remove);
        insert_property_fact(
            &tx,
            input.partition,
            input.scenario_id,
            input.entity_id,
            input.field_id,
            &input.element,
            input.valid_from,
            input.valid_to,
            input.layer,
            asserted_at,
            op_id,
            is_tombstone,
            self.backend,
        )
        .await?;
        if is_indexed {
            if is_tombstone {
                delete_index_row(
                    &tx,
                    input.partition,
                    input.scenario_id,
                    input.field_id,
                    input.entity_id,
                    &input.element,
                    self.backend,
                )
                .await?;
            } else {
                insert_index_row(
                    &tx,
                    input.partition,
                    input.scenario_id,
                    input.field_id,
                    input.entity_id,
                    &input.element,
                    input.valid_from,
                    input.valid_to,
                    asserted_at,
                    input.layer,
                    self.backend,
                )
                .await?;
            }
        }
        tx.commit().await?;
        Ok(op_id)
    }

    async fn counter_update(&self, input: CounterUpdateInput) -> MnemeResult<OpId> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, input.partition, input.actor)
            .await?;
        let (_value_type, merge_policy, _multi, is_indexed) = self
            .fetch_field_def(&tx, input.partition, input.field_id)
            .await?;
        if merge_policy != MergePolicy::Counter {
            return Err(MnemeError::invalid(
                "field is not configured for COUNTER merge policy",
            ));
        }
        let payload = OpPayload::CounterUpdate(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(&tx, input.partition, input.actor, input.asserted_at, &payload)
            .await?;
        let value = Value::I64(input.delta);
        insert_property_fact(
            &tx,
            input.partition,
            input.scenario_id,
            input.entity_id,
            input.field_id,
            &value,
            input.valid_from,
            input.valid_to,
            input.layer,
            asserted_at,
            op_id,
            false,
            self.backend,
        )
        .await?;
        if is_indexed {
            insert_index_row(
                &tx,
                input.partition,
                input.scenario_id,
                input.field_id,
                input.entity_id,
                &value,
                input.valid_from,
                input.valid_to,
                asserted_at,
                input.layer,
                self.backend,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(op_id)
    }
}

#[async_trait]
impl GraphReadApi for MnemeStore {
    async fn read_entity_at_time(
        &self,
        input: ReadEntityAtTimeInput,
    ) -> MnemeResult<ReadEntityAtTimeResult> {
        let (resolved_partition, kind, type_id, is_deleted) = self
            .read_entity_row_with_fallback(input.partition, input.scenario_id, input.entity_id)
            .await?;

        let field_ids = if let Some(field_ids) = input.field_ids.clone() {
            field_ids
        } else if let Some(type_id) = type_id {
            let select = Query::select()
                .from(AideonTypeFields::Table)
                .column(AideonTypeFields::FieldId)
                .and_where(
                    Expr::col(AideonTypeFields::PartitionId)
                        .eq(id_value(self.backend, resolved_partition.0)),
                )
                .and_where(Expr::col(AideonTypeFields::TypeId).eq(id_value(self.backend, type_id)))
                .to_owned();
            let rows = query_all(&self.conn, &select).await?;
            rows.into_iter()
                .map(|row| read_id(&row, AideonTypeFields::FieldId))
                .collect::<MnemeResult<Vec<_>>>()?
        } else {
            Vec::new()
        };

        let mut properties = HashMap::new();
        for field_id in field_ids {
            let (value_type, merge_policy, _multi, _is_indexed) = self
                .fetch_field_def(&self.conn, resolved_partition, field_id)
                .await?;
            let facts = fetch_property_facts_with_fallback(
                &self.conn,
                resolved_partition,
                input.scenario_id,
                input.entity_id,
                field_id,
                input.at_valid_time,
                input.as_of_asserted_at,
                value_type,
                self.backend,
            )
            .await?;
            if facts.is_empty() {
                if input.include_defaults {
                    if let Some(default) =
                        default_value_for_field(&self.conn, resolved_partition, type_id, field_id)
                            .await?
                    {
                        properties.insert(field_id, ReadValue::Single(default));
                    }
                }
                continue;
            }
            let resolved = resolve_property(merge_policy, facts)?;
            if let Some(resolved) = resolved {
                properties.insert(field_id, resolved);
            }
        }

        Ok(ReadEntityAtTimeResult {
            entity_id: input.entity_id,
            kind,
            type_id,
            is_deleted,
            properties,
        })
    }

    async fn traverse_at_time(
        &self,
        input: TraverseAtTimeInput,
    ) -> MnemeResult<Vec<TraverseEdgeItem>> {
        let mut facts_by_edge: HashMap<Id, Vec<EdgeFact>> = HashMap::new();
        let mut edge_ids = std::collections::HashSet::new();

        let scenario_facts = fetch_edge_facts_in_partition(
            &self.conn,
            input.partition,
            input.scenario_id,
            &input,
            self.backend,
        )
        .await?;
        for fact in scenario_facts {
            edge_ids.insert(fact.edge_id);
            facts_by_edge.entry(fact.edge_id).or_default().push(fact);
        }

        if input.scenario_id.is_some() {
            let baseline_facts = fetch_edge_facts_in_partition(
                &self.conn,
                input.partition,
                None,
                &input,
                self.backend,
            )
            .await?;
            for fact in baseline_facts {
                if edge_ids.contains(&fact.edge_id) {
                    continue;
                }
                facts_by_edge.entry(fact.edge_id).or_default().push(fact);
            }
        }
        let mut edges = Vec::new();
        for (_edge_id, facts) in facts_by_edge {
            if let Some(edge) = resolve_edge(facts)? {
                edges.push(edge);
                if edges.len() >= input.limit as usize {
                    break;
                }
            }
        }
        Ok(edges)
    }

    async fn list_entities(
        &self,
        input: crate::ListEntitiesInput,
    ) -> MnemeResult<Vec<crate::ListEntitiesResultItem>> {
        if input.limit == 0 {
            return Ok(Vec::new());
        }
        let mut candidate_ids: Option<std::collections::HashSet<Id>> = None;
        for filter in &input.filters {
            let (value_type, _merge_policy, _multi, is_indexed) = self
                .fetch_field_def(&self.conn, input.partition, filter.field_id)
                .await?;
            if !is_indexed {
                return Err(MnemeError::invalid("field is not indexed"));
            }
            if filter.value.value_type() != value_type {
                return Err(MnemeError::invalid("filter value type mismatch"));
            }
            let ids = if let Some(scenario_id) = input.scenario_id {
                let mut scenario_ids = fetch_index_candidates(
                    &self.conn,
                    input.partition,
                    Some(scenario_id),
                    input.at_valid_time,
                    input.as_of_asserted_at,
                    filter,
                    self.backend,
                )
                .await?;
                let baseline_ids = fetch_index_candidates(
                    &self.conn,
                    input.partition,
                    None,
                    input.at_valid_time,
                    input.as_of_asserted_at,
                    filter,
                    self.backend,
                )
                .await?;
                scenario_ids.extend(baseline_ids);
                scenario_ids
            } else {
                fetch_index_candidates(
                    &self.conn,
                    input.partition,
                    None,
                    input.at_valid_time,
                    input.as_of_asserted_at,
                    filter,
                    self.backend,
                )
                .await?
            };
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            candidate_ids = Some(match candidate_ids.take() {
                Some(existing) => existing
                    .intersection(&ids)
                    .copied()
                    .collect::<std::collections::HashSet<_>>(),
                None => ids,
            });
        }

        let candidates: Vec<Id> = match candidate_ids {
            Some(ids) => ids.into_iter().collect(),
            None => {
                return list_entities_without_filters(
                    &self.conn,
                    input,
                    self.backend,
                )
                .await
            }
        };

        let mut results = Vec::new();
        for entity_id in candidates {
            let (_partition, kind, type_id, is_deleted) = self
                .read_entity_row_with_fallback(input.partition, input.scenario_id, entity_id)
                .await?;
            if is_deleted {
                continue;
            }
            if let Some(kind_filter) = input.kind {
                if kind != kind_filter {
                    continue;
                }
            }
            if let Some(type_filter) = input.type_id {
                if type_id != Some(type_filter) {
                    continue;
                }
            }
            let mut matches = true;
            for filter in &input.filters {
                let (value_type, merge_policy, _multi, _is_indexed) = self
                    .fetch_field_def(&self.conn, input.partition, filter.field_id)
                    .await?;
                let facts = fetch_property_facts_with_fallback(
                    &self.conn,
                    input.partition,
                    input.scenario_id,
                    entity_id,
                    filter.field_id,
                    input.at_valid_time,
                    input.as_of_asserted_at,
                    value_type,
                    self.backend,
                )
                .await?;
                let resolved = resolve_property(merge_policy, facts)?;
                if !filter_matches(resolved, filter)? {
                    matches = false;
                    break;
                }
            }
            if matches {
                results.push(crate::ListEntitiesResultItem {
                    entity_id,
                    kind,
                    type_id,
                });
            }
        }

        results.sort_by(|a, b| a.entity_id.as_bytes().cmp(&b.entity_id.as_bytes()));
        if let Some(cursor) = input.cursor.as_deref() {
            let cursor_id = parse_cursor_id(cursor)?;
            results.retain(|item| item.entity_id.as_bytes() > cursor_id.as_bytes());
        }
        if results.len() > input.limit as usize {
            results.truncate(input.limit as usize);
        }
        Ok(results)
    }
}

#[async_trait]
impl AnalyticsApi for MnemeStore {
    async fn get_projection_edges(
        &self,
        input: crate::GetProjectionEdgesInput,
    ) -> MnemeResult<Vec<ProjectionEdge>> {
        let limit = input.limit.unwrap_or(u32::MAX) as usize;
        let mut edges = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let scenario_edges = fetch_projection_edges_in_partition(
            &self.conn,
            input.partition,
            input.scenario_id,
            input.at_valid_time,
            input.as_of_asserted_at,
            input.edge_type_filter.clone(),
            self.backend,
        )
        .await?;
        for (edge_id, edge) in scenario_edges {
            seen.insert(edge_id);
            edges.push(edge);
            if edges.len() >= limit {
                return Ok(edges);
            }
        }

        if input.scenario_id.is_some() {
            let baseline_edges = fetch_projection_edges_in_partition(
                &self.conn,
                input.partition,
                None,
                input.at_valid_time,
                input.as_of_asserted_at,
                input.edge_type_filter.clone(),
                self.backend,
            )
            .await?;
            for (edge_id, edge) in baseline_edges {
                if seen.contains(&edge_id) {
                    continue;
                }
                edges.push(edge);
                if edges.len() >= limit {
                    break;
                }
            }
        }

        Ok(edges)
    }
}

#[async_trait]
impl AnalyticsResultsApi for MnemeStore {
    async fn store_pagerank_scores(
        &self,
        partition: PartitionId,
        actor: ActorId,
        as_of_valid_time: Option<ValidTime>,
        as_of_asserted_at: Option<Hlc>,
        spec: crate::PageRankRunSpec,
        scores: Vec<(Id, f64)>,
    ) -> MnemeResult<Id> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, partition, actor).await?;
        let run_id = Id::new();
        let insert = Query::insert()
            .into_table(AideonPagerankRuns::Table)
            .columns([
                AideonPagerankRuns::PartitionId,
                AideonPagerankRuns::RunId,
                AideonPagerankRuns::AsOfValidTime,
                AideonPagerankRuns::AsOfAssertedAtHlc,
                AideonPagerankRuns::ParamsJson,
                AideonPagerankRuns::CreatedAssertedAtHlc,
            ])
            .values_panic([
                id_value(self.backend, partition.0).into(),
                id_value(self.backend, run_id).into(),
                as_of_valid_time.map(|v| v.0).into(),
                as_of_asserted_at.map(|v| v.as_i64()).into(),
                serde_json::to_string(&spec)
                    .map_err(|err| MnemeError::storage(err.to_string()))?
                    .into(),
                Hlc::now().as_i64().into(),
            ])
            .to_owned();
        exec(&tx, &insert).await?;

        for (id, score) in scores {
            let insert_item = Query::insert()
                .into_table(AideonPagerankScores::Table)
                .columns([
                    AideonPagerankScores::PartitionId,
                    AideonPagerankScores::RunId,
                    AideonPagerankScores::EntityId,
                    AideonPagerankScores::Score,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    id_value(self.backend, run_id).into(),
                    id_value(self.backend, id).into(),
                    score.into(),
                ])
                .to_owned();
            exec(&tx, &insert_item).await?;
        }

        tx.commit().await?;
        Ok(run_id)
    }

    async fn get_pagerank_scores(
        &self,
        partition: PartitionId,
        run_id: Id,
        top_n: u32,
    ) -> MnemeResult<Vec<(Id, f64)>> {
        let select = Query::select()
            .from(AideonPagerankScores::Table)
            .columns([
                (AideonPagerankScores::Table, AideonPagerankScores::EntityId),
                (AideonPagerankScores::Table, AideonPagerankScores::Score),
            ])
            .and_where(
                Expr::col(AideonPagerankScores::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .and_where(Expr::col(AideonPagerankScores::RunId).eq(id_value(self.backend, run_id)))
            .order_by(AideonPagerankScores::Score, Order::Desc)
            .limit(top_n as u64)
            .to_owned();
        let rows = query_all(&self.conn, &select).await?;
        let mut scores = Vec::new();
        for row in rows {
            let id = read_id(&row, AideonPagerankScores::EntityId)?;
            let score: f64 = row.try_get("", &col_name(AideonPagerankScores::Score))?;
            scores.push((id, score));
        }
        Ok(scores)
    }
}

#[async_trait]
impl SyncApi for MnemeStore {
    async fn export_ops(&self, input: ExportOpsInput) -> MnemeResult<Vec<OpEnvelope>> {
        let mut select = Query::select()
            .from(AideonOps::Table)
            .columns([
                (AideonOps::Table, AideonOps::OpId),
                (AideonOps::Table, AideonOps::ActorId),
                (AideonOps::Table, AideonOps::AssertedAtHlc),
                (AideonOps::Table, AideonOps::OpType),
                (AideonOps::Table, AideonOps::Payload),
            ])
            .and_where(
                Expr::col(AideonOps::PartitionId).eq(id_value(self.backend, input.partition.0)),
            )
            .order_by(AideonOps::AssertedAtHlc, Order::Asc)
            .limit(input.limit as u64)
            .to_owned();
        if let Some(since) = input.since_asserted_at {
            select.and_where(Expr::col(AideonOps::AssertedAtHlc).gt(since.as_i64()));
        }
        let rows = query_all(&self.conn, &select).await?;
        let mut ops = Vec::new();
        for row in rows {
            let op_id = read_id(&row, AideonOps::OpId)?;
            let actor_id = read_id(&row, AideonOps::ActorId)?;
            let asserted_at = read_hlc(&row, AideonOps::AssertedAtHlc)?;
            let op_type: i64 = row.try_get("", &col_name(AideonOps::OpType))?;
            let payload: Vec<u8> = row.try_get("", &col_name(AideonOps::Payload))?;
            let payload_scenario = scenario_id_from_payload(&payload)?;
            if let Some(filter_scenario) = input.scenario_id {
                if payload_scenario != Some(filter_scenario) {
                    continue;
                }
            } else if payload_scenario.is_some() {
                continue;
            }
            let deps = fetch_op_deps(&self.conn, input.partition, op_id).await?;
            ops.push(OpEnvelope {
                op_id: OpId(op_id),
                actor_id: ActorId(actor_id),
                asserted_at,
                op_type: op_type as u16,
                payload,
                deps: deps.into_iter().map(OpId).collect(),
            });
        }
        Ok(ops)
    }

    async fn ingest_ops(&self, partition: PartitionId, ops: Vec<OpEnvelope>) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        for op in ops {
            let payload: OpPayload = serde_json::from_slice(&op.payload)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            self.bump_hlc_state(&tx, partition, op.asserted_at).await?;
            let insert = Query::insert()
                .into_table(AideonOps::Table)
                .columns([
                    AideonOps::PartitionId,
                    AideonOps::OpId,
                    AideonOps::ActorId,
                    AideonOps::AssertedAtHlc,
                    AideonOps::TxId,
                    AideonOps::OpType,
                    AideonOps::Payload,
                    AideonOps::SchemaVersionHint,
                    AideonOps::IngestedAssertedAtHlc,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    id_value(self.backend, op.op_id.0).into(),
                    id_value(self.backend, op.actor_id.0).into(),
                    op.asserted_at.as_i64().into(),
                    none_id_value(self.backend).into(),
                    (op.op_type as i64).into(),
                    op.payload.clone().into(),
                    SeaValue::String(None).into(),
                    Hlc::now().as_i64().into(),
                ])
                .on_conflict(
                    OnConflict::columns([AideonOps::PartitionId, AideonOps::OpId])
                        .do_nothing()
                        .to_owned(),
                )
                .to_owned();
            exec(&tx, &insert).await?;

            for dep in op.deps {
                let insert_dep = Query::insert()
                    .into_table(AideonOpDeps::Table)
                    .columns([
                        AideonOpDeps::PartitionId,
                        AideonOpDeps::OpId,
                        AideonOpDeps::DepOpId,
                    ])
                    .values_panic([
                        id_value(self.backend, partition.0).into(),
                        id_value(self.backend, op.op_id.0).into(),
                        id_value(self.backend, dep.0).into(),
                    ])
                    .on_conflict(
                        OnConflict::columns([
                            AideonOpDeps::PartitionId,
                            AideonOpDeps::OpId,
                            AideonOpDeps::DepOpId,
                        ])
                        .do_nothing()
                        .to_owned(),
                    )
                    .to_owned();
                exec(&tx, &insert_dep).await?;
            }
            self.append_change_feed(&tx, partition, op.op_id, op.asserted_at, &payload)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn get_partition_head(&self, partition: PartitionId) -> MnemeResult<Hlc> {
        let select = Query::select()
            .from(AideonOps::Table)
            .column(AideonOps::AssertedAtHlc)
            .and_where(Expr::col(AideonOps::PartitionId).eq(id_value(self.backend, partition.0)))
            .order_by(AideonOps::AssertedAtHlc, Order::Desc)
            .limit(1)
            .to_owned();
        let row = query_one(&self.conn, &select).await?;
        let Some(row) = row else {
            return Ok(Hlc(0));
        };
        read_hlc(&row, AideonOps::AssertedAtHlc)
    }
}

#[async_trait]
impl ScenarioApi for MnemeStore {
    async fn create_scenario(&self, input: CreateScenarioInput) -> MnemeResult<crate::ScenarioId> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, input.partition, input.actor)
            .await?;
        let scenario_id = crate::ScenarioId(Id::new());
        let payload = OpPayload::CreateScenario {
            partition: input.partition,
            scenario_id,
            actor: input.actor,
            asserted_at: input.asserted_at,
            name: input.name.clone(),
        };
        let (_op_id, _asserted_at, _payload, _op_type) = self
            .insert_op(&tx, input.partition, input.actor, input.asserted_at, &payload)
            .await?;
        tx.commit().await?;
        Ok(scenario_id)
    }

    async fn delete_scenario(
        &self,
        partition: PartitionId,
        actor: ActorId,
        asserted_at: Hlc,
        scenario: ScenarioId,
    ) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, partition, actor).await?;
        let payload = OpPayload::DeleteScenario {
            partition,
            scenario_id: scenario,
            actor,
            asserted_at,
        };
        let (_op_id, _asserted_at, _payload, _op_type) = self
            .insert_op(&tx, partition, actor, asserted_at, &payload)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

const JOB_STATUS_PENDING: u8 = 0;
const JOB_STATUS_RUNNING: u8 = 1;
const JOB_STATUS_SUCCEEDED: u8 = 2;
const JOB_STATUS_FAILED: u8 = 3;

const JOB_TYPE_SCHEMA_REBUILD: &str = "schema_rebuild";
const JOB_TYPE_INTEGRITY_REFRESH: &str = "integrity_refresh";
const JOB_TYPE_ANALYTICS_REFRESH: &str = "analytics_refresh";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TriggerJobPayload {
    partition_id: PartitionId,
    scenario_id: Option<ScenarioId>,
    type_id: Option<Id>,
    schema_version_hint: Option<String>,
    reason: String,
}

impl MnemeStore {
    async fn enqueue_job(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        job_type: &str,
        payload: Vec<u8>,
        priority: i32,
        dedupe_key: Option<String>,
    ) -> MnemeResult<Id> {
        let job_id = Id::new();
        let now = Hlc::now().as_i64();
        let mut insert = Query::insert()
            .into_table(AideonJobs::Table)
            .columns([
                AideonJobs::PartitionId,
                AideonJobs::JobId,
                AideonJobs::JobType,
                AideonJobs::Status,
                AideonJobs::Priority,
                AideonJobs::Attempts,
                AideonJobs::MaxAttempts,
                AideonJobs::LeaseExpiresAt,
                AideonJobs::NextRunAfter,
                AideonJobs::CreatedAssertedAtHlc,
                AideonJobs::UpdatedAssertedAtHlc,
                AideonJobs::Payload,
                AideonJobs::DedupeKey,
                AideonJobs::LastError,
            ])
            .values_panic([
                id_value(self.backend, partition.0).into(),
                id_value(self.backend, job_id).into(),
                job_type.to_string().into(),
                (JOB_STATUS_PENDING as i64).into(),
                (priority as i64).into(),
                0i64.into(),
                3i64.into(),
                SeaValue::BigInt(None).into(),
                SeaValue::BigInt(None).into(),
                now.into(),
                now.into(),
                payload.into(),
                dedupe_key.clone().into(),
                SeaValue::String(None).into(),
            ])
            .to_owned();
        if dedupe_key.is_some() {
            insert.on_conflict(
                OnConflict::columns([
                    AideonJobs::PartitionId,
                    AideonJobs::JobType,
                    AideonJobs::DedupeKey,
                    AideonJobs::Status,
                ])
                .do_nothing()
                .to_owned(),
            );
        }
        exec(tx, &insert).await?;
        Ok(job_id)
    }

    async fn list_jobs_internal(
        &self,
        partition: PartitionId,
        status: Option<u8>,
        limit: u32,
    ) -> MnemeResult<Vec<JobRecord>> {
        let mut select = Query::select()
            .from(AideonJobs::Table)
            .columns([
                AideonJobs::JobId,
                AideonJobs::JobType,
                AideonJobs::Status,
                AideonJobs::Priority,
                AideonJobs::Attempts,
                AideonJobs::MaxAttempts,
                AideonJobs::LeaseExpiresAt,
                AideonJobs::NextRunAfter,
                AideonJobs::CreatedAssertedAtHlc,
                AideonJobs::UpdatedAssertedAtHlc,
                AideonJobs::DedupeKey,
                AideonJobs::LastError,
                AideonJobs::Payload,
            ])
            .and_where(Expr::col(AideonJobs::PartitionId).eq(id_value(self.backend, partition.0)))
            .order_by(AideonJobs::CreatedAssertedAtHlc, Order::Desc)
            .limit(limit as u64)
            .to_owned();
        if let Some(status) = status {
            select.and_where(Expr::col(AideonJobs::Status).eq(status as i64));
        }
        let rows = query_all(&self.conn, &select).await?;
        let mut jobs = Vec::new();
        for row in rows {
            let job_id = read_id(&row, AideonJobs::JobId)?;
            let job_type: String = row.try_get("", &col_name(AideonJobs::JobType))?;
            let status: i64 = row.try_get("", &col_name(AideonJobs::Status))?;
            let priority: i64 = row.try_get("", &col_name(AideonJobs::Priority))?;
            let attempts: i64 = row.try_get("", &col_name(AideonJobs::Attempts))?;
            let max_attempts: i64 = row.try_get("", &col_name(AideonJobs::MaxAttempts))?;
            let lease_expires_at: Option<i64> =
                row.try_get("", &col_name(AideonJobs::LeaseExpiresAt))?;
            let next_run_after: Option<i64> =
                row.try_get("", &col_name(AideonJobs::NextRunAfter))?;
            let created_asserted_at = read_hlc(&row, AideonJobs::CreatedAssertedAtHlc)?;
            let updated_asserted_at = read_hlc(&row, AideonJobs::UpdatedAssertedAtHlc)?;
            let dedupe_key: Option<String> =
                row.try_get("", &col_name(AideonJobs::DedupeKey))?;
            let last_error: Option<String> =
                row.try_get("", &col_name(AideonJobs::LastError))?;
            let payload: Vec<u8> = row.try_get("", &col_name(AideonJobs::Payload))?;
            jobs.push(JobRecord {
                partition,
                job_id,
                job_type,
                status: status as u8,
                priority: priority as i32,
                attempts: attempts as i32,
                max_attempts: max_attempts as i32,
                lease_expires_at,
                next_run_after,
                created_asserted_at,
                updated_asserted_at,
                dedupe_key,
                last_error,
                payload,
            });
        }
        Ok(jobs)
    }

    async fn claim_pending_jobs(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        max_jobs: u32,
        lease_millis: u64,
    ) -> MnemeResult<Vec<JobRecord>> {
        let now = Hlc::now().as_i64();
        let select = Query::select()
            .from(AideonJobs::Table)
            .columns([
                AideonJobs::JobId,
                AideonJobs::JobType,
                AideonJobs::Status,
                AideonJobs::Priority,
                AideonJobs::Attempts,
                AideonJobs::MaxAttempts,
                AideonJobs::LeaseExpiresAt,
                AideonJobs::NextRunAfter,
                AideonJobs::CreatedAssertedAtHlc,
                AideonJobs::UpdatedAssertedAtHlc,
                AideonJobs::DedupeKey,
                AideonJobs::LastError,
                AideonJobs::Payload,
            ])
            .and_where(Expr::col(AideonJobs::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(
                Expr::col(AideonJobs::Status)
                    .eq(JOB_STATUS_PENDING as i64)
                    .or(
                        Expr::col(AideonJobs::Status)
                            .eq(JOB_STATUS_RUNNING as i64)
                            .and(Expr::col(AideonJobs::LeaseExpiresAt).lt(now)),
                    ),
            )
            .and_where(
                Expr::col(AideonJobs::NextRunAfter)
                    .is_null()
                    .or(Expr::col(AideonJobs::NextRunAfter).lte(now)),
            )
            .order_by(AideonJobs::Priority, Order::Desc)
            .order_by(AideonJobs::CreatedAssertedAtHlc, Order::Asc)
            .limit(max_jobs as u64)
            .to_owned();
        let rows = query_all(tx, &select).await?;
        let mut claimed = Vec::new();
        let lease_expires_at = now + lease_millis as i64;
        for row in rows {
            let job_id = read_id(&row, AideonJobs::JobId)?;
            let update = Query::update()
                .table(AideonJobs::Table)
                .values([
                    (AideonJobs::Status, (JOB_STATUS_RUNNING as i64).into()),
                    (AideonJobs::LeaseExpiresAt, lease_expires_at.into()),
                    (AideonJobs::NextRunAfter, SeaValue::BigInt(None).into()),
                    (AideonJobs::UpdatedAssertedAtHlc, now.into()),
                ])
                .and_where(Expr::col(AideonJobs::PartitionId).eq(id_value(self.backend, partition.0)))
                .and_where(Expr::col(AideonJobs::JobId).eq(id_value(self.backend, job_id)))
                .and_where(
                    Expr::col(AideonJobs::Status)
                        .eq(JOB_STATUS_PENDING as i64)
                        .or(
                            Expr::col(AideonJobs::Status)
                                .eq(JOB_STATUS_RUNNING as i64)
                                .and(Expr::col(AideonJobs::LeaseExpiresAt).lt(now)),
                        ),
                )
                .to_owned();
            let result = tx.execute(&update).await?;
            if result.rows_affected() == 0 {
                continue;
            }
            let job_type: String = row.try_get("", &col_name(AideonJobs::JobType))?;
            let status: i64 = row.try_get("", &col_name(AideonJobs::Status))?;
            let priority: i64 = row.try_get("", &col_name(AideonJobs::Priority))?;
            let attempts: i64 = row.try_get("", &col_name(AideonJobs::Attempts))?;
            let max_attempts: i64 = row.try_get("", &col_name(AideonJobs::MaxAttempts))?;
            let lease_expires_at: Option<i64> =
                row.try_get("", &col_name(AideonJobs::LeaseExpiresAt))?;
            let next_run_after: Option<i64> =
                row.try_get("", &col_name(AideonJobs::NextRunAfter))?;
            let created_asserted_at = read_hlc(&row, AideonJobs::CreatedAssertedAtHlc)?;
            let updated_asserted_at = read_hlc(&row, AideonJobs::UpdatedAssertedAtHlc)?;
            let dedupe_key: Option<String> =
                row.try_get("", &col_name(AideonJobs::DedupeKey))?;
            let last_error: Option<String> =
                row.try_get("", &col_name(AideonJobs::LastError))?;
            let payload: Vec<u8> = row.try_get("", &col_name(AideonJobs::Payload))?;
            claimed.push(JobRecord {
                partition,
                job_id,
                job_type,
                status: status as u8,
                priority: priority as i32,
                attempts: attempts as i32,
                max_attempts: max_attempts as i32,
                lease_expires_at,
                next_run_after,
                created_asserted_at,
                updated_asserted_at,
                dedupe_key,
                last_error,
                payload,
            });
        }
        Ok(claimed)
    }

    async fn mark_job_succeeded(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        job_id: Id,
    ) -> MnemeResult<()> {
        let now = Hlc::now().as_i64();
        let update = Query::update()
            .table(AideonJobs::Table)
            .values([
                (AideonJobs::Status, (JOB_STATUS_SUCCEEDED as i64).into()),
                (AideonJobs::LeaseExpiresAt, SeaValue::BigInt(None).into()),
                (AideonJobs::NextRunAfter, SeaValue::BigInt(None).into()),
                (AideonJobs::LastError, SeaValue::String(None).into()),
                (AideonJobs::UpdatedAssertedAtHlc, now.into()),
            ])
            .and_where(Expr::col(AideonJobs::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(Expr::col(AideonJobs::JobId).eq(id_value(self.backend, job_id)))
            .to_owned();
        exec(tx, &update).await?;
        Ok(())
    }

    async fn mark_job_failed(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        job: &JobRecord,
        error: &str,
    ) -> MnemeResult<()> {
        let now = Hlc::now().as_i64();
        let next_attempts = job.attempts + 1;
        let status = if next_attempts >= job.max_attempts {
            JOB_STATUS_FAILED
        } else {
            JOB_STATUS_PENDING
        };
        let next_run_after = if status == JOB_STATUS_PENDING {
            Some(now + job_backoff_millis(next_attempts))
        } else {
            None
        };
        let update = Query::update()
            .table(AideonJobs::Table)
            .values([
                (AideonJobs::Status, (status as i64).into()),
                (AideonJobs::Attempts, (next_attempts as i64).into()),
                (AideonJobs::LeaseExpiresAt, SeaValue::BigInt(None).into()),
                (AideonJobs::NextRunAfter, next_run_after.into()),
                (AideonJobs::LastError, error.to_string().into()),
                (AideonJobs::UpdatedAssertedAtHlc, now.into()),
            ])
            .and_where(Expr::col(AideonJobs::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(Expr::col(AideonJobs::JobId).eq(id_value(self.backend, job.job_id)))
            .to_owned();
        exec(tx, &update).await?;

        let insert_event = Query::insert()
            .into_table(AideonJobEvents::Table)
            .columns([
                AideonJobEvents::PartitionId,
                AideonJobEvents::JobId,
                AideonJobEvents::EventTime,
                AideonJobEvents::Message,
            ])
            .values_panic([
                id_value(self.backend, partition.0).into(),
                id_value(self.backend, job.job_id).into(),
                now.into(),
                error.to_string().into(),
            ])
            .to_owned();
        exec(tx, &insert_event).await?;
        Ok(())
    }

    async fn refresh_integrity(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        scenario_id: Option<ScenarioId>,
        reason: &str,
    ) -> MnemeResult<()> {
        let run_id = Id::new();
        let params = serde_json::json!({ "reason": reason });
        let now = Hlc::now().as_i64();
        let insert_run = Query::insert()
            .into_table(AideonIntegrityRuns::Table)
            .columns([
                AideonIntegrityRuns::PartitionId,
                AideonIntegrityRuns::RunId,
                AideonIntegrityRuns::ScenarioId,
                AideonIntegrityRuns::AsOfValidTime,
                AideonIntegrityRuns::AsOfAssertedAtHlc,
                AideonIntegrityRuns::ParamsJson,
                AideonIntegrityRuns::CreatedAssertedAtHlc,
            ])
            .values_panic([
                id_value(self.backend, partition.0).into(),
                id_value(self.backend, run_id).into(),
                opt_id_value(self.backend, scenario_id.map(|s| s.0)).into(),
                SeaValue::BigInt(None).into(),
                SeaValue::BigInt(None).into(),
                params.to_string().into(),
                now.into(),
            ])
            .to_owned();
        exec(tx, &insert_run).await?;

        let insert_head = Query::insert()
            .into_table(AideonIntegrityHead::Table)
            .columns([
                AideonIntegrityHead::PartitionId,
                AideonIntegrityHead::ScenarioId,
                AideonIntegrityHead::RunId,
                AideonIntegrityHead::UpdatedAssertedAtHlc,
            ])
            .values_panic([
                id_value(self.backend, partition.0).into(),
                opt_id_value(self.backend, scenario_id.map(|s| s.0)).into(),
                id_value(self.backend, run_id).into(),
                now.into(),
            ])
            .on_conflict(
                OnConflict::columns([
                    AideonIntegrityHead::PartitionId,
                    AideonIntegrityHead::ScenarioId,
                ])
                .update_columns([
                    AideonIntegrityHead::RunId,
                    AideonIntegrityHead::UpdatedAssertedAtHlc,
                ])
                .to_owned(),
            )
            .to_owned();
        exec(tx, &insert_head).await?;
        Ok(())
    }

    async fn refresh_analytics(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        scenario_id: Option<ScenarioId>,
    ) -> MnemeResult<()> {
        let delete_degree = Query::delete()
            .from_table(AideonGraphDegreeStats::Table)
            .and_where(Expr::col(AideonGraphDegreeStats::PartitionId).eq(id_value(
                self.backend,
                partition.0,
            )))
            .and_where(match scenario_id {
                Some(scenario_id) => Expr::col(AideonGraphDegreeStats::ScenarioId)
                    .eq(id_value(self.backend, scenario_id.0)),
                None => Expr::col(AideonGraphDegreeStats::ScenarioId).is_null(),
            })
            .to_owned();
        exec(tx, &delete_degree).await?;

        let delete_counts = Query::delete()
            .from_table(AideonGraphEdgeTypeCounts::Table)
            .and_where(Expr::col(AideonGraphEdgeTypeCounts::PartitionId).eq(id_value(
                self.backend,
                partition.0,
            )))
            .and_where(match scenario_id {
                Some(scenario_id) => Expr::col(AideonGraphEdgeTypeCounts::ScenarioId)
                    .eq(id_value(self.backend, scenario_id.0)),
                None => Expr::col(AideonGraphEdgeTypeCounts::ScenarioId).is_null(),
            })
            .to_owned();
        exec(tx, &delete_counts).await?;

        let now = Hlc::now().as_i64();
        let mut out_select = Query::select();
        out_select
            .from(AideonGraphProjectionEdges::Table)
            .columns([
                AideonGraphProjectionEdges::SrcEntityId,
                AideonGraphProjectionEdges::ScenarioId,
            ])
            .expr_as(Func::count(Expr::col(AideonGraphProjectionEdges::EdgeId)), Alias::new("cnt"))
            .and_where(
                Expr::col(AideonGraphProjectionEdges::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
            )
            .group_by_col(AideonGraphProjectionEdges::SrcEntityId)
            .group_by_col(AideonGraphProjectionEdges::ScenarioId);
        if let Some(scenario_id) = scenario_id {
            out_select.and_where(
                Expr::col(AideonGraphProjectionEdges::ScenarioId)
                    .eq(id_value(self.backend, scenario_id.0)),
            );
        } else {
            out_select.and_where(Expr::col(AideonGraphProjectionEdges::ScenarioId).is_null());
        }
        let out_rows = query_all(tx, &out_select).await?;

        let mut in_select = Query::select();
        in_select
            .from(AideonGraphProjectionEdges::Table)
            .columns([
                AideonGraphProjectionEdges::DstEntityId,
                AideonGraphProjectionEdges::ScenarioId,
            ])
            .expr_as(Func::count(Expr::col(AideonGraphProjectionEdges::EdgeId)), Alias::new("cnt"))
            .and_where(
                Expr::col(AideonGraphProjectionEdges::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
            )
            .group_by_col(AideonGraphProjectionEdges::DstEntityId)
            .group_by_col(AideonGraphProjectionEdges::ScenarioId);
        if let Some(scenario_id) = scenario_id {
            in_select.and_where(
                Expr::col(AideonGraphProjectionEdges::ScenarioId)
                    .eq(id_value(self.backend, scenario_id.0)),
            );
        } else {
            in_select.and_where(Expr::col(AideonGraphProjectionEdges::ScenarioId).is_null());
        }
        let in_rows = query_all(tx, &in_select).await?;

        let mut degrees: HashMap<Id, (i32, i32)> = HashMap::new();
        for row in out_rows {
            let entity_id = read_id_by_name(&row, &col_name(AideonGraphProjectionEdges::SrcEntityId))?;
            let count: i64 = row.try_get("", "cnt")?;
            let entry = degrees.entry(entity_id).or_insert((0, 0));
            entry.0 = count as i32;
        }
        for row in in_rows {
            let entity_id = read_id_by_name(&row, &col_name(AideonGraphProjectionEdges::DstEntityId))?;
            let count: i64 = row.try_get("", "cnt")?;
            let entry = degrees.entry(entity_id).or_insert((0, 0));
            entry.1 = count as i32;
        }

        for (entity_id, (out_degree, in_degree)) in degrees {
            let insert = Query::insert()
                .into_table(AideonGraphDegreeStats::Table)
                .columns([
                    AideonGraphDegreeStats::PartitionId,
                    AideonGraphDegreeStats::ScenarioId,
                    AideonGraphDegreeStats::AsOfValidTime,
                    AideonGraphDegreeStats::EntityId,
                    AideonGraphDegreeStats::OutDegree,
                    AideonGraphDegreeStats::InDegree,
                    AideonGraphDegreeStats::ComputedAssertedAtHlc,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    opt_id_value(self.backend, scenario_id.map(|s| s.0)).into(),
                    SeaValue::BigInt(None).into(),
                    id_value(self.backend, entity_id).into(),
                    (out_degree as i64).into(),
                    (in_degree as i64).into(),
                    now.into(),
                ])
                .to_owned();
            exec(tx, &insert).await?;
        }

        let mut counts_select = Query::select();
        counts_select
            .from(AideonGraphProjectionEdges::Table)
            .columns([
                AideonGraphProjectionEdges::EdgeTypeId,
                AideonGraphProjectionEdges::ScenarioId,
            ])
            .expr_as(Func::count(Expr::col(AideonGraphProjectionEdges::EdgeId)), Alias::new("cnt"))
            .and_where(
                Expr::col(AideonGraphProjectionEdges::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
            )
            .group_by_col(AideonGraphProjectionEdges::EdgeTypeId)
            .group_by_col(AideonGraphProjectionEdges::ScenarioId);
        if let Some(scenario_id) = scenario_id {
            counts_select.and_where(
                Expr::col(AideonGraphProjectionEdges::ScenarioId)
                    .eq(id_value(self.backend, scenario_id.0)),
            );
        } else {
            counts_select.and_where(Expr::col(AideonGraphProjectionEdges::ScenarioId).is_null());
        }
        let count_rows = query_all(tx, &counts_select).await?;
        for row in count_rows {
            let edge_type_id = read_opt_id(&row, AideonGraphProjectionEdges::EdgeTypeId)?;
            let count: i64 = row.try_get("", "cnt")?;
            let insert = Query::insert()
                .into_table(AideonGraphEdgeTypeCounts::Table)
                .columns([
                    AideonGraphEdgeTypeCounts::PartitionId,
                    AideonGraphEdgeTypeCounts::ScenarioId,
                    AideonGraphEdgeTypeCounts::EdgeTypeId,
                    AideonGraphEdgeTypeCounts::Count,
                    AideonGraphEdgeTypeCounts::ComputedAssertedAtHlc,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    opt_id_value(self.backend, scenario_id.map(|s| s.0)).into(),
                    opt_id_value(self.backend, edge_type_id.map(|id| id)).into(),
                    count.into(),
                    now.into(),
                ])
                .to_owned();
            exec(tx, &insert).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl MnemeProcessingApi for MnemeStore {
    async fn trigger_rebuild_effective_schema(
        &self,
        input: TriggerProcessingInput,
    ) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        let payload = TriggerJobPayload {
            partition_id: input.partition,
            scenario_id: input.scenario_id,
            type_id: None,
            schema_version_hint: None,
            reason: input.reason,
        };
        let payload = serde_json::to_vec(&payload)
            .map_err(|err| MnemeError::storage(err.to_string()))?;
        self.enqueue_job(
            &tx,
            input.partition,
            JOB_TYPE_SCHEMA_REBUILD,
            payload,
            0,
            Some("schema:rebuild".to_string()),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn trigger_refresh_integrity(&self, input: TriggerProcessingInput) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        let payload = TriggerJobPayload {
            partition_id: input.partition,
            scenario_id: input.scenario_id,
            type_id: None,
            schema_version_hint: None,
            reason: input.reason,
        };
        let payload = serde_json::to_vec(&payload)
            .map_err(|err| MnemeError::storage(err.to_string()))?;
        self.enqueue_job(
            &tx,
            input.partition,
            JOB_TYPE_INTEGRITY_REFRESH,
            payload,
            0,
            Some("integrity:refresh".to_string()),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn trigger_refresh_analytics_projections(
        &self,
        input: TriggerProcessingInput,
    ) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        let payload = TriggerJobPayload {
            partition_id: input.partition,
            scenario_id: input.scenario_id,
            type_id: None,
            schema_version_hint: None,
            reason: input.reason,
        };
        let payload = serde_json::to_vec(&payload)
            .map_err(|err| MnemeError::storage(err.to_string()))?;
        self.enqueue_job(
            &tx,
            input.partition,
            JOB_TYPE_ANALYTICS_REFRESH,
            payload,
            0,
            Some("analytics:refresh".to_string()),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn run_processing_worker(&self, input: RunWorkerInput) -> MnemeResult<u32> {
        let mut processed = 0u32;
        let select_partition = Query::select()
            .from(AideonJobs::Table)
            .column(AideonJobs::PartitionId)
            .and_where(Expr::col(AideonJobs::Status).eq(JOB_STATUS_PENDING as i64))
            .order_by(AideonJobs::Priority, Order::Desc)
            .order_by(AideonJobs::CreatedAssertedAtHlc, Order::Asc)
            .limit(1)
            .to_owned();
        let partition_row = query_one(&self.conn, &select_partition).await?;
        let Some(partition_row) = partition_row else {
            return Ok(0);
        };
        let partition_id = read_id(&partition_row, AideonJobs::PartitionId)?;
        let partition = PartitionId(partition_id);

        let tx = self.conn.begin().await?;
        let mut jobs = self
            .claim_pending_jobs(&tx, partition, input.max_jobs, input.lease_millis)
            .await?;
        tx.commit().await?;

        for job in jobs.drain(..) {
            let payload: TriggerJobPayload = serde_json::from_slice(&job.payload)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let tx = self.conn.begin().await?;
            let result = match job.job_type.as_str() {
                JOB_TYPE_SCHEMA_REBUILD => {
                    let mut type_ids = Vec::new();
                    if let Some(type_id) = payload.type_id {
                        type_ids.push(type_id);
                    } else {
                        let select = Query::select()
                            .from(AideonTypes::Table)
                            .column(AideonTypes::TypeId)
                            .and_where(
                                Expr::col(AideonTypes::PartitionId)
                                    .eq(id_value(self.backend, payload.partition_id.0)),
                            )
                            .and_where(Expr::col(AideonTypes::IsDeleted).eq(false))
                            .to_owned();
                        let rows = query_all(&tx, &select).await?;
                        for row in rows {
                            type_ids.push(read_id(&row, AideonTypes::TypeId)?);
                        }
                    }

                    for type_id in type_ids {
                        let version = self
                            .compile_effective_schema(
                                payload.partition_id,
                                ActorId(Id::new()),
                                Hlc::now(),
                                type_id,
                            )
                            .await?;
                        let update = Query::insert()
                            .into_table(AideonTypeSchemaHead::Table)
                            .columns([
                                AideonTypeSchemaHead::PartitionId,
                                AideonTypeSchemaHead::TypeId,
                                AideonTypeSchemaHead::SchemaVersionHash,
                                AideonTypeSchemaHead::UpdatedAssertedAtHlc,
                            ])
                            .values_panic([
                                id_value(self.backend, payload.partition_id.0).into(),
                                id_value(self.backend, type_id).into(),
                                version.schema_version_hash.clone().into(),
                                Hlc::now().as_i64().into(),
                            ])
                            .on_conflict(
                                OnConflict::columns([
                                    AideonTypeSchemaHead::PartitionId,
                                    AideonTypeSchemaHead::TypeId,
                                ])
                                .update_columns([
                                    AideonTypeSchemaHead::SchemaVersionHash,
                                    AideonTypeSchemaHead::UpdatedAssertedAtHlc,
                                ])
                                .to_owned(),
                            )
                            .to_owned();
                        exec(&tx, &update).await?;
                    }
                    Ok(())
                }
                JOB_TYPE_INTEGRITY_REFRESH => {
                    self.refresh_integrity(&tx, payload.partition_id, payload.scenario_id, &payload.reason)
                        .await
                }
                JOB_TYPE_ANALYTICS_REFRESH => {
                    self.refresh_analytics(&tx, payload.partition_id, payload.scenario_id)
                        .await
                }
                other => Err(MnemeError::not_implemented(format!(
                    "unknown job type {other}"
                ))),
            };
            match result {
                Ok(()) => {
                    self.mark_job_succeeded(&tx, payload.partition_id, job.job_id)
                        .await?;
                    tx.commit().await?;
                    processed += 1;
                }
                Err(err) => {
                    self.mark_job_failed(&tx, payload.partition_id, &job, &err.to_string())
                        .await?;
                    tx.commit().await?;
                }
            }
        }
        Ok(processed)
    }

    async fn list_jobs(
        &self,
        partition: PartitionId,
        status: Option<u8>,
        limit: u32,
    ) -> MnemeResult<Vec<JobSummary>> {
        let jobs = self.list_jobs_internal(partition, status, limit).await?;
        Ok(jobs
            .into_iter()
            .map(|job| JobSummary {
                partition,
                job_id: job.job_id,
                job_type: job.job_type,
                status: job.status,
                priority: job.priority,
                attempts: job.attempts,
                max_attempts: job.max_attempts,
                lease_expires_at: job.lease_expires_at,
                next_run_after: job.next_run_after,
                created_asserted_at: job.created_asserted_at,
                updated_asserted_at: job.updated_asserted_at,
                dedupe_key: job.dedupe_key,
                last_error: job.last_error,
            })
            .collect())
    }
}

#[async_trait]
impl ChangeFeedApi for MnemeStore {
    async fn get_changes_since(
        &self,
        partition: PartitionId,
        from_sequence: Option<i64>,
        limit: u32,
    ) -> MnemeResult<Vec<ChangeEvent>> {
        let mut select = Query::select()
            .from(AideonChangeFeed::Table)
            .columns([
                AideonChangeFeed::Sequence,
                AideonChangeFeed::OpId,
                AideonChangeFeed::AssertedAtHlc,
                AideonChangeFeed::EntityId,
                AideonChangeFeed::ChangeKind,
                AideonChangeFeed::PayloadJson,
            ])
            .and_where(
                Expr::col(AideonChangeFeed::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .order_by(AideonChangeFeed::Sequence, Order::Asc)
            .limit(limit as u64)
            .to_owned();
        if let Some(from_sequence) = from_sequence {
            select.and_where(Expr::col(AideonChangeFeed::Sequence).gt(from_sequence));
        }
        let rows = query_all(&self.conn, &select).await?;
        let mut events = Vec::new();
        for row in rows {
            let sequence: i64 = row.try_get("", &col_name(AideonChangeFeed::Sequence))?;
            let op_id = read_id(&row, AideonChangeFeed::OpId)?;
            let asserted_at = read_hlc(&row, AideonChangeFeed::AssertedAtHlc)?;
            let entity_id = read_opt_id(&row, AideonChangeFeed::EntityId)?;
            let change_kind: i64 = row.try_get("", &col_name(AideonChangeFeed::ChangeKind))?;
            let payload_json: Option<String> =
                row.try_get("", &col_name(AideonChangeFeed::PayloadJson))?;
            let payload = payload_json
                .map(|raw| serde_json::from_str(&raw))
                .transpose()
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            events.push(ChangeEvent {
                partition,
                sequence,
                op_id: OpId(op_id),
                asserted_at,
                entity_id,
                change_kind: change_kind as u8,
                payload,
            });
        }
        Ok(events)
    }
}

#[async_trait]
impl ValidationRulesApi for MnemeStore {
    async fn upsert_validation_rules(
        &self,
        partition: PartitionId,
        _actor: ActorId,
        asserted_at: Hlc,
        rules: Vec<ValidationRule>,
    ) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        for rule in rules {
            let params = serde_json::to_string(&rule.params)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let insert = Query::insert()
                .into_table(AideonValidationRules::Table)
                .columns([
                    AideonValidationRules::PartitionId,
                    AideonValidationRules::RuleId,
                    AideonValidationRules::ScopeKind,
                    AideonValidationRules::ScopeId,
                    AideonValidationRules::Severity,
                    AideonValidationRules::TemplateKind,
                    AideonValidationRules::ParamsJson,
                    AideonValidationRules::UpdatedAssertedAtHlc,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    id_value(self.backend, rule.rule_id).into(),
                    (rule.scope_kind as i64).into(),
                    opt_id_value(self.backend, rule.scope_id).into(),
                    (rule.severity as i64).into(),
                    rule.template_kind.into(),
                    params.into(),
                    asserted_at.as_i64().into(),
                ])
                .on_conflict(
                    OnConflict::columns([AideonValidationRules::PartitionId, AideonValidationRules::RuleId])
                        .update_columns([
                            AideonValidationRules::ScopeKind,
                            AideonValidationRules::ScopeId,
                            AideonValidationRules::Severity,
                            AideonValidationRules::TemplateKind,
                            AideonValidationRules::ParamsJson,
                            AideonValidationRules::UpdatedAssertedAtHlc,
                        ])
                        .to_owned(),
                )
                .to_owned();
            exec(&tx, &insert).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_validation_rules(
        &self,
        partition: PartitionId,
    ) -> MnemeResult<Vec<ValidationRule>> {
        let select = Query::select()
            .from(AideonValidationRules::Table)
            .columns([
                AideonValidationRules::RuleId,
                AideonValidationRules::ScopeKind,
                AideonValidationRules::ScopeId,
                AideonValidationRules::Severity,
                AideonValidationRules::TemplateKind,
                AideonValidationRules::ParamsJson,
            ])
            .and_where(
                Expr::col(AideonValidationRules::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        let rows = query_all(&self.conn, &select).await?;
        let mut rules = Vec::new();
        for row in rows {
            let rule_id = read_id(&row, AideonValidationRules::RuleId)?;
            let scope_kind: i64 = row.try_get("", &col_name(AideonValidationRules::ScopeKind))?;
            let scope_id = read_opt_id(&row, AideonValidationRules::ScopeId)?;
            let severity: i64 = row.try_get("", &col_name(AideonValidationRules::Severity))?;
            let template_kind: String =
                row.try_get("", &col_name(AideonValidationRules::TemplateKind))?;
            let params_json: String =
                row.try_get("", &col_name(AideonValidationRules::ParamsJson))?;
            let params = serde_json::from_str(&params_json)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            rules.push(ValidationRule {
                rule_id,
                scope_kind: scope_kind as u8,
                scope_id,
                severity: severity as u8,
                template_kind,
                params,
            });
        }
        Ok(rules)
    }
}

#[async_trait]
impl DiagnosticsApi for MnemeStore {
    async fn get_integrity_head(
        &self,
        partition: PartitionId,
        scenario_id: Option<ScenarioId>,
    ) -> MnemeResult<Option<crate::api::IntegrityHead>> {
        let mut select = Query::select()
            .from(AideonIntegrityHead::Table)
            .columns([
                AideonIntegrityHead::RunId,
                AideonIntegrityHead::UpdatedAssertedAtHlc,
            ])
            .and_where(
                Expr::col(AideonIntegrityHead::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        if let Some(scenario_id) = scenario_id {
            select.and_where(
                Expr::col(AideonIntegrityHead::ScenarioId)
                    .eq(id_value(self.backend, scenario_id.0)),
            );
        } else {
            select.and_where(Expr::col(AideonIntegrityHead::ScenarioId).is_null());
        }
        let row = query_one(&self.conn, &select).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let run_id = read_id(&row, AideonIntegrityHead::RunId)?;
        let updated_asserted_at = read_hlc(&row, AideonIntegrityHead::UpdatedAssertedAtHlc)?;
        Ok(Some(crate::api::IntegrityHead {
            partition,
            scenario_id,
            run_id,
            updated_asserted_at,
        }))
    }

    async fn get_last_schema_compile(
        &self,
        partition: PartitionId,
        type_id: Id,
    ) -> MnemeResult<Option<crate::api::SchemaHead>> {
        let select = Query::select()
            .from(AideonTypeSchemaHead::Table)
            .columns([
                AideonTypeSchemaHead::SchemaVersionHash,
                AideonTypeSchemaHead::UpdatedAssertedAtHlc,
            ])
            .and_where(
                Expr::col(AideonTypeSchemaHead::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
            )
            .and_where(Expr::col(AideonTypeSchemaHead::TypeId).eq(id_value(self.backend, type_id)))
            .limit(1)
            .to_owned();
        let row = query_one(&self.conn, &select).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let schema_version_hash: String =
            row.try_get("", &col_name(AideonTypeSchemaHead::SchemaVersionHash))?;
        let updated_asserted_at = read_hlc(&row, AideonTypeSchemaHead::UpdatedAssertedAtHlc)?;
        Ok(Some(crate::api::SchemaHead {
            partition,
            type_id,
            schema_version_hash,
            updated_asserted_at,
        }))
    }

    async fn list_failed_jobs(
        &self,
        partition: PartitionId,
        limit: u32,
    ) -> MnemeResult<Vec<JobSummary>> {
        let jobs = self
            .list_jobs_internal(partition, Some(JOB_STATUS_FAILED), limit)
            .await?;
        Ok(jobs
            .into_iter()
            .map(|job| JobSummary {
                partition,
                job_id: job.job_id,
                job_type: job.job_type,
                status: job.status,
                priority: job.priority,
                attempts: job.attempts,
                max_attempts: job.max_attempts,
                lease_expires_at: job.lease_expires_at,
                next_run_after: job.next_run_after,
                created_asserted_at: job.created_asserted_at,
                updated_asserted_at: job.updated_asserted_at,
                dedupe_key: job.dedupe_key,
                last_error: job.last_error,
            })
            .collect())
    }
}

#[async_trait]
impl MnemeExportApi for MnemeStore {
    async fn export_ops_stream(
        &self,
        options: ExportOptions,
    ) -> MnemeResult<Box<dyn Iterator<Item = ExportRecord> + Send>> {
        let mut select = Query::select()
            .from(AideonOps::Table)
            .columns([
                (AideonOps::Table, AideonOps::OpId),
                (AideonOps::Table, AideonOps::ActorId),
                (AideonOps::Table, AideonOps::AssertedAtHlc),
                (AideonOps::Table, AideonOps::OpType),
                (AideonOps::Table, AideonOps::Payload),
            ])
            .and_where(
                Expr::col(AideonOps::PartitionId).eq(id_value(self.backend, options.partition.0)),
            )
            .order_by(AideonOps::AssertedAtHlc, Order::Asc)
            .to_owned();
        if let Some(since) = options.since_asserted_at {
            select.and_where(Expr::col(AideonOps::AssertedAtHlc).gt(since.as_i64()));
        }
        if let Some(until) = options.until_asserted_at {
            select.and_where(Expr::col(AideonOps::AssertedAtHlc).lte(until.as_i64()));
        }
        let rows = query_all(&self.conn, &select).await?;
        let mut records = Vec::new();
        records.push(ExportRecord {
            record_type: "header".to_string(),
            data: serde_json::json!({
                "partition_id": options.partition,
                "scenario_id": options.scenario_id,
                "since_asserted_at": options.since_asserted_at.map(|v| v.as_i64()),
                "until_asserted_at": options.until_asserted_at.map(|v| v.as_i64()),
            }),
        });
        let mut hasher = blake3::Hasher::new();
        let mut count = 0u32;
        for row in rows {
            let op_id = read_id(&row, AideonOps::OpId)?;
            let actor_id = read_id(&row, AideonOps::ActorId)?;
            let asserted_at = read_hlc(&row, AideonOps::AssertedAtHlc)?;
            let op_type: i64 = row.try_get("", &col_name(AideonOps::OpType))?;
            let payload: Vec<u8> = row.try_get("", &col_name(AideonOps::Payload))?;
            let deps = fetch_op_deps(&self.conn, options.partition, op_id).await?;
            let envelope = OpEnvelope {
                op_id: OpId(op_id),
                actor_id: ActorId(actor_id),
                asserted_at,
                op_type: op_type as u16,
                payload,
                deps: deps.into_iter().map(OpId).collect(),
            };
            let data = serde_json::to_value(&envelope)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let bytes = serde_json::to_vec(&envelope)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            hasher.update(&bytes);
            count += 1;
            records.push(ExportRecord {
                record_type: "op".to_string(),
                data,
            });
        }
        let checksum = hasher.finalize().to_hex().to_string();
        records.push(ExportRecord {
            record_type: "footer".to_string(),
            data: serde_json::json!({
                "op_count": count,
                "checksum": checksum,
            }),
        });
        Ok(Box::new(records.into_iter()))
    }
}

#[async_trait]
impl MnemeImportApi for MnemeStore {
    async fn import_ops_stream<I>(
        &self,
        options: ImportOptions,
        records: I,
    ) -> MnemeResult<ImportReport>
    where
        I: Iterator<Item = ExportRecord> + Send,
    {
        let mut hasher = blake3::Hasher::new();
        let mut ops = Vec::new();
        let mut reported_checksum: Option<String> = None;
        for record in records {
            match record.record_type.as_str() {
                "header" => {}
                "footer" => {
                    reported_checksum = record
                        .data
                        .get("checksum")
                        .and_then(|value| value.as_str())
                        .map(|value| value.to_string());
                }
                "op" => {
                    let mut envelope: OpEnvelope = serde_json::from_value(record.data)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    let mut payload: OpPayload = serde_json::from_slice(&envelope.payload)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    if let Some(remapped) = options.remap_actor_ids.get(&envelope.actor_id) {
                        envelope.actor_id = *remapped;
                        match &mut payload {
                            OpPayload::CreateNode(input) => input.actor = *remapped,
                            OpPayload::CreateEdge(input) => input.actor = *remapped,
                            OpPayload::SetEdgeExistenceInterval(input) => input.actor = *remapped,
                            OpPayload::TombstoneEntity { actor, .. } => *actor = *remapped,
                            OpPayload::SetProperty(input) => input.actor = *remapped,
                            OpPayload::ClearProperty(input) => input.actor = *remapped,
                            OpPayload::OrSetUpdate(input) => input.actor = *remapped,
                            OpPayload::CounterUpdate(input) => input.actor = *remapped,
                            OpPayload::UpsertMetamodelBatch(_) => {}
                            OpPayload::CreateScenario { actor, .. } => *actor = *remapped,
                            OpPayload::DeleteScenario { actor, .. } => *actor = *remapped,
                        }
                    }
                    if let Some(scenario_id) = options.scenario_id {
                        match &mut payload {
                            OpPayload::CreateNode(input) => input.scenario_id = Some(scenario_id),
                            OpPayload::CreateEdge(input) => input.scenario_id = Some(scenario_id),
                            OpPayload::SetEdgeExistenceInterval(input) => {
                                input.scenario_id = Some(scenario_id)
                            }
                            OpPayload::TombstoneEntity { scenario_id: sid, .. } => {
                                *sid = Some(scenario_id)
                            }
                            OpPayload::SetProperty(input) => input.scenario_id = Some(scenario_id),
                            OpPayload::ClearProperty(input) => input.scenario_id = Some(scenario_id),
                            OpPayload::OrSetUpdate(input) => input.scenario_id = Some(scenario_id),
                            OpPayload::CounterUpdate(input) => input.scenario_id = Some(scenario_id),
                            OpPayload::UpsertMetamodelBatch(_) => {}
                            OpPayload::CreateScenario { .. } => {}
                            OpPayload::DeleteScenario { .. } => {}
                        }
                    }
                    let payload_bytes =
                        serde_json::to_vec(&payload).map_err(|err| MnemeError::storage(err.to_string()))?;
                    envelope.payload = payload_bytes;
                    let bytes = serde_json::to_vec(&envelope)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    hasher.update(&bytes);
                    ops.push(envelope);
                }
                _ => {}
            }
        }
        if let Some(checksum) = reported_checksum {
            let computed = hasher.finalize().to_hex().to_string();
            if checksum != computed && options.strict_schema {
                return Err(MnemeError::invalid("checksum mismatch"));
            }
        }

        if options.allow_partition_create {
            if let Some(first) = ops.first() {
                let tx = self.conn.begin().await?;
                self.ensure_partition(&tx, options.target_partition, first.actor_id)
                    .await?;
                tx.commit().await?;
            }
        }

        let mut imported = 0u32;
        for chunk in ops.chunks(1000) {
            self.ingest_ops(options.target_partition, chunk.to_vec())
                .await?;
            imported += chunk.len() as u32;
        }
        Ok(ImportReport {
            ops_imported: imported,
            ops_skipped: 0,
            errors: 0,
        })
    }
}

#[async_trait]
impl MnemeSnapshotApi for MnemeStore {
    async fn export_snapshot_stream(
        &self,
        opts: SnapshotOptions,
    ) -> MnemeResult<Box<dyn Iterator<Item = ExportRecord> + Send>> {
        let mut records = Vec::new();
        records.push(ExportRecord {
            record_type: "snapshot_header".to_string(),
            data: serde_json::json!({
                "partition_id": opts.partition_id,
                "scenario_id": opts.scenario_id,
                "as_of_asserted_at": opts.as_of_asserted_at.as_i64(),
                "include_entities": opts.include_entities,
                "include_facts": opts.include_facts,
            }),
        });
        if opts.include_entities {
            let mut select = Query::select()
                .from(AideonEntities::Table)
                .columns([
                    AideonEntities::EntityId,
                    AideonEntities::EntityKind,
                    AideonEntities::TypeId,
                    AideonEntities::IsDeleted,
                    AideonEntities::UpdatedAssertedAtHlc,
                ])
                .and_where(
                    Expr::col(AideonEntities::PartitionId)
                        .eq(id_value(self.backend, opts.partition_id.0)),
                )
                .and_where(
                    Expr::col(AideonEntities::UpdatedAssertedAtHlc)
                        .lte(opts.as_of_asserted_at.as_i64()),
                )
                .to_owned();
            if let Some(scenario_id) = opts.scenario_id {
                select.and_where(
                    Expr::col(AideonEntities::ScenarioId)
                        .eq(id_value(self.backend, scenario_id.0)),
                );
            } else {
                select.and_where(Expr::col(AideonEntities::ScenarioId).is_null());
            }
            for row in query_all(&self.conn, &select).await? {
                let entity_id = read_id(&row, AideonEntities::EntityId)?;
                let kind_raw: i16 = row.try_get("", &col_name(AideonEntities::EntityKind))?;
                let type_id = read_opt_id(&row, AideonEntities::TypeId)?;
                let is_deleted: bool = row.try_get("", &col_name(AideonEntities::IsDeleted))?;
                records.push(ExportRecord {
                    record_type: "snapshot_entity".to_string(),
                    data: serde_json::json!({
                        "entity_id": entity_id,
                        "kind": kind_raw,
                        "type_id": type_id,
                        "is_deleted": is_deleted,
                    }),
                });
            }

            let mut edge_select = Query::select()
                .from(AideonEdges::Table)
                .columns([
                    AideonEdges::EdgeId,
                    AideonEdges::SrcEntityId,
                    AideonEdges::DstEntityId,
                    AideonEdges::EdgeTypeId,
                ])
                .and_where(
                    Expr::col(AideonEdges::PartitionId)
                        .eq(id_value(self.backend, opts.partition_id.0)),
                )
                .to_owned();
            if let Some(scenario_id) = opts.scenario_id {
                edge_select.and_where(
                    Expr::col(AideonEdges::ScenarioId)
                        .eq(id_value(self.backend, scenario_id.0)),
                );
            } else {
                edge_select.and_where(Expr::col(AideonEdges::ScenarioId).is_null());
            }
            for row in query_all(&self.conn, &edge_select).await? {
                let edge_id = read_id(&row, AideonEdges::EdgeId)?;
                let src_id = read_id(&row, AideonEdges::SrcEntityId)?;
                let dst_id = read_id(&row, AideonEdges::DstEntityId)?;
                let edge_type_id = read_opt_id(&row, AideonEdges::EdgeTypeId)?;
                records.push(ExportRecord {
                    record_type: "snapshot_edge".to_string(),
                    data: serde_json::json!({
                        "edge_id": edge_id,
                        "src_id": src_id,
                        "dst_id": dst_id,
                        "edge_type_id": edge_type_id,
                    }),
                });
            }
        }

        if opts.include_facts {
            let mut edge_fact_select = Query::select()
                .from(AideonEdgeExistsFacts::Table)
                .columns([
                    AideonEdgeExistsFacts::EdgeId,
                    AideonEdgeExistsFacts::ValidFrom,
                    AideonEdgeExistsFacts::ValidTo,
                    AideonEdgeExistsFacts::Layer,
                    AideonEdgeExistsFacts::AssertedAtHlc,
                    AideonEdgeExistsFacts::IsTombstone,
                ])
                .and_where(
                    Expr::col(AideonEdgeExistsFacts::PartitionId)
                        .eq(id_value(self.backend, opts.partition_id.0)),
                )
                .and_where(
                    Expr::col(AideonEdgeExistsFacts::AssertedAtHlc)
                        .lte(opts.as_of_asserted_at.as_i64()),
                )
                .to_owned();
            if let Some(scenario_id) = opts.scenario_id {
                edge_fact_select.and_where(
                    Expr::col(AideonEdgeExistsFacts::ScenarioId)
                        .eq(id_value(self.backend, scenario_id.0)),
                );
            } else {
                edge_fact_select.and_where(Expr::col(AideonEdgeExistsFacts::ScenarioId).is_null());
            }
            for row in query_all(&self.conn, &edge_fact_select).await? {
                let edge_id = read_id(&row, AideonEdgeExistsFacts::EdgeId)?;
                let valid_from: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::ValidFrom))?;
                let valid_to: Option<i64> =
                    row.try_get("", &col_name(AideonEdgeExistsFacts::ValidTo))?;
                let layer: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::Layer))?;
                let asserted_at = read_hlc(&row, AideonEdgeExistsFacts::AssertedAtHlc)?;
                let is_tombstone: bool =
                    row.try_get("", &col_name(AideonEdgeExistsFacts::IsTombstone))?;
                records.push(ExportRecord {
                    record_type: "snapshot_edge_exists".to_string(),
                    data: serde_json::json!({
                        "edge_id": edge_id,
                        "valid_from": valid_from,
                        "valid_to": valid_to,
                        "layer": layer,
                        "asserted_at": asserted_at.as_i64(),
                        "is_tombstone": is_tombstone,
                    }),
                });
            }

            self.export_fact_table(
                &mut records,
                &opts,
                AideonPropFactStr::Table,
                AideonPropFactStr::EntityId,
                AideonPropFactStr::FieldId,
                AideonPropFactStr::ValidFrom,
                AideonPropFactStr::ValidTo,
                AideonPropFactStr::Layer,
                AideonPropFactStr::AssertedAtHlc,
                AideonPropFactStr::IsTombstone,
                AideonPropFactStr::ValueText,
                "snapshot_fact_str",
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                AideonPropFactI64::Table,
                AideonPropFactI64::EntityId,
                AideonPropFactI64::FieldId,
                AideonPropFactI64::ValidFrom,
                AideonPropFactI64::ValidTo,
                AideonPropFactI64::Layer,
                AideonPropFactI64::AssertedAtHlc,
                AideonPropFactI64::IsTombstone,
                AideonPropFactI64::ValueI64,
                "snapshot_fact_i64",
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                AideonPropFactF64::Table,
                AideonPropFactF64::EntityId,
                AideonPropFactF64::FieldId,
                AideonPropFactF64::ValidFrom,
                AideonPropFactF64::ValidTo,
                AideonPropFactF64::Layer,
                AideonPropFactF64::AssertedAtHlc,
                AideonPropFactF64::IsTombstone,
                AideonPropFactF64::ValueF64,
                "snapshot_fact_f64",
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                AideonPropFactBool::Table,
                AideonPropFactBool::EntityId,
                AideonPropFactBool::FieldId,
                AideonPropFactBool::ValidFrom,
                AideonPropFactBool::ValidTo,
                AideonPropFactBool::Layer,
                AideonPropFactBool::AssertedAtHlc,
                AideonPropFactBool::IsTombstone,
                AideonPropFactBool::ValueBool,
                "snapshot_fact_bool",
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                AideonPropFactTime::Table,
                AideonPropFactTime::EntityId,
                AideonPropFactTime::FieldId,
                AideonPropFactTime::ValidFrom,
                AideonPropFactTime::ValidTo,
                AideonPropFactTime::Layer,
                AideonPropFactTime::AssertedAtHlc,
                AideonPropFactTime::IsTombstone,
                AideonPropFactTime::ValueTime,
                "snapshot_fact_time",
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                AideonPropFactRef::Table,
                AideonPropFactRef::EntityId,
                AideonPropFactRef::FieldId,
                AideonPropFactRef::ValidFrom,
                AideonPropFactRef::ValidTo,
                AideonPropFactRef::Layer,
                AideonPropFactRef::AssertedAtHlc,
                AideonPropFactRef::IsTombstone,
                AideonPropFactRef::ValueRefEntityId,
                "snapshot_fact_ref",
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                AideonPropFactBlob::Table,
                AideonPropFactBlob::EntityId,
                AideonPropFactBlob::FieldId,
                AideonPropFactBlob::ValidFrom,
                AideonPropFactBlob::ValidTo,
                AideonPropFactBlob::Layer,
                AideonPropFactBlob::AssertedAtHlc,
                AideonPropFactBlob::IsTombstone,
                AideonPropFactBlob::ValueBlob,
                "snapshot_fact_blob",
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                AideonPropFactJson::Table,
                AideonPropFactJson::EntityId,
                AideonPropFactJson::FieldId,
                AideonPropFactJson::ValidFrom,
                AideonPropFactJson::ValidTo,
                AideonPropFactJson::Layer,
                AideonPropFactJson::AssertedAtHlc,
                AideonPropFactJson::IsTombstone,
                AideonPropFactJson::ValueJson,
                "snapshot_fact_json",
            )
            .await?;
        }

        records.push(ExportRecord {
            record_type: "snapshot_footer".to_string(),
            data: serde_json::json!({ "complete": true }),
        });
        Ok(Box::new(records.into_iter()))
    }

    async fn import_snapshot_stream<I>(
        &self,
        opts: ImportOptions,
        records: I,
    ) -> MnemeResult<()>
    where
        I: Iterator<Item = ExportRecord> + Send,
    {
        if opts.allow_partition_create {
            let tx = self.conn.begin().await?;
            self.ensure_partition(&tx, opts.target_partition, ActorId(Id::new()))
                .await?;
            tx.commit().await?;
        }
        for record in records {
            match record.record_type.as_str() {
                "snapshot_entity" => {
                    let entity_id = Id::from_uuid_str(
                        record
                            .data
                            .get("entity_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| MnemeError::invalid("missing entity_id"))?,
                    )?;
                    let kind: i64 = record
                        .data
                        .get("kind")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| MnemeError::invalid("missing kind"))?;
                    let type_id = match record.data.get("type_id") {
                        Some(value) if !value.is_null() => {
                            Some(Id::from_uuid_str(value.as_str().unwrap_or_default())?)
                        }
                        _ => None,
                    };
                    let is_deleted = record
                        .data
                        .get("is_deleted")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let insert = Query::insert()
                        .into_table(AideonEntities::Table)
                        .columns([
                            AideonEntities::PartitionId,
                            AideonEntities::ScenarioId,
                            AideonEntities::EntityId,
                            AideonEntities::EntityKind,
                            AideonEntities::TypeId,
                            AideonEntities::IsDeleted,
                            AideonEntities::CreatedOpId,
                            AideonEntities::UpdatedOpId,
                            AideonEntities::CreatedAssertedAtHlc,
                            AideonEntities::UpdatedAssertedAtHlc,
                        ])
                        .values_panic([
                            id_value(self.backend, opts.target_partition.0).into(),
                            opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                            id_value(self.backend, entity_id).into(),
                            kind.into(),
                            opt_id_value(self.backend, type_id).into(),
                            is_deleted.into(),
                            none_id_value(self.backend).into(),
                            none_id_value(self.backend).into(),
                            Hlc::now().as_i64().into(),
                            Hlc::now().as_i64().into(),
                        ])
                        .to_owned();
                    exec(&self.conn, &insert).await?;
                }
                "snapshot_edge" => {
                    let edge_id = Id::from_uuid_str(
                        record
                            .data
                            .get("edge_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| MnemeError::invalid("missing edge_id"))?,
                    )?;
                    let src_id = Id::from_uuid_str(
                        record
                            .data
                            .get("src_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| MnemeError::invalid("missing src_id"))?,
                    )?;
                    let dst_id = Id::from_uuid_str(
                        record
                            .data
                            .get("dst_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| MnemeError::invalid("missing dst_id"))?,
                    )?;
                    let edge_type_id = match record.data.get("edge_type_id") {
                        Some(value) if !value.is_null() => {
                            Some(Id::from_uuid_str(value.as_str().unwrap_or_default())?)
                        }
                        _ => None,
                    };
                    let insert = Query::insert()
                        .into_table(AideonEdges::Table)
                        .columns([
                            AideonEdges::PartitionId,
                            AideonEdges::ScenarioId,
                            AideonEdges::EdgeId,
                            AideonEdges::SrcEntityId,
                            AideonEdges::DstEntityId,
                            AideonEdges::EdgeTypeId,
                        ])
                        .values_panic([
                            id_value(self.backend, opts.target_partition.0).into(),
                            opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                            id_value(self.backend, edge_id).into(),
                            id_value(self.backend, src_id).into(),
                            id_value(self.backend, dst_id).into(),
                            opt_id_value(self.backend, edge_type_id).into(),
                        ])
                        .to_owned();
                    exec(&self.conn, &insert).await?;
                }
                "snapshot_edge_exists" => {
                    let edge_id = Id::from_uuid_str(
                        record
                            .data
                            .get("edge_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| MnemeError::invalid("missing edge_id"))?,
                    )?;
                    let valid_from = record
                        .data
                        .get("valid_from")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| MnemeError::invalid("missing valid_from"))?;
                    let valid_to = record.data.get("valid_to").and_then(|v| v.as_i64());
                    let layer = record
                        .data
                        .get("layer")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| MnemeError::invalid("missing layer"))?;
                    let asserted_at = record
                        .data
                        .get("asserted_at")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| MnemeError::invalid("missing asserted_at"))?;
                    let is_tombstone = record
                        .data
                        .get("is_tombstone")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let insert = Query::insert()
                        .into_table(AideonEdgeExistsFacts::Table)
                        .columns([
                            AideonEdgeExistsFacts::PartitionId,
                            AideonEdgeExistsFacts::ScenarioId,
                            AideonEdgeExistsFacts::EdgeId,
                            AideonEdgeExistsFacts::ValidFrom,
                            AideonEdgeExistsFacts::ValidTo,
                            AideonEdgeExistsFacts::ValidBucket,
                            AideonEdgeExistsFacts::Layer,
                            AideonEdgeExistsFacts::AssertedAtHlc,
                            AideonEdgeExistsFacts::OpId,
                            AideonEdgeExistsFacts::IsTombstone,
                        ])
                        .values_panic([
                            id_value(self.backend, opts.target_partition.0).into(),
                            opt_id_value(self.backend, opts.scenario_id.map(|s| s.0)).into(),
                            id_value(self.backend, edge_id).into(),
                            valid_from.into(),
                            valid_to.into(),
                            valid_bucket(ValidTime(valid_from)).into(),
                            layer.into(),
                            asserted_at.into(),
                            none_id_value(self.backend).into(),
                            is_tombstone.into(),
                        ])
                        .to_owned();
                    exec(&self.conn, &insert).await?;
                }
                "snapshot_fact_str"
                | "snapshot_fact_i64"
                | "snapshot_fact_f64"
                | "snapshot_fact_bool"
                | "snapshot_fact_time"
                | "snapshot_fact_ref"
                | "snapshot_fact_blob"
                | "snapshot_fact_json" => {
                    self.import_fact_record(&opts, &record).await?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}


#[derive(Clone)]
struct PropertyFact {
    value: Value,
    valid_from: i64,
    valid_to: Option<i64>,
    layer: i64,
    asserted_at: Hlc,
    op_id: Id,
    is_tombstone: bool,
}

struct EdgeFact {
    edge_id: Id,
    src_id: Id,
    dst_id: Id,
    edge_type_id: Option<Id>,
    valid_from: i64,
    valid_to: Option<i64>,
    layer: i64,
    asserted_at: Hlc,
    op_id: Id,
    is_tombstone: bool,
}

struct DefaultValueColumns {
    str_value: SeaValue,
    i64_value: SeaValue,
    f64_value: SeaValue,
    bool_value: SeaValue,
    time_value: SeaValue,
    ref_value: SeaValue,
    blob_value: SeaValue,
    json_value: SeaValue,
}

fn default_value_columns(backend: DatabaseBackend, value: &Option<Value>) -> DefaultValueColumns {
    match value {
        Some(Value::Str(value)) => DefaultValueColumns {
            str_value: value.clone().into(),
            i64_value: SeaValue::BigInt(None),
            f64_value: SeaValue::Double(None),
            bool_value: SeaValue::Bool(None),
            time_value: SeaValue::BigInt(None),
            ref_value: none_id_value(backend),
            blob_value: SeaValue::Bytes(None),
            json_value: SeaValue::String(None),
        },
        Some(Value::I64(value)) => DefaultValueColumns {
            str_value: SeaValue::String(None),
            i64_value: (*value).into(),
            f64_value: SeaValue::Double(None),
            bool_value: SeaValue::Bool(None),
            time_value: SeaValue::BigInt(None),
            ref_value: none_id_value(backend),
            blob_value: SeaValue::Bytes(None),
            json_value: SeaValue::String(None),
        },
        Some(Value::F64(value)) => DefaultValueColumns {
            str_value: SeaValue::String(None),
            i64_value: SeaValue::BigInt(None),
            f64_value: (*value).into(),
            bool_value: SeaValue::Bool(None),
            time_value: SeaValue::BigInt(None),
            ref_value: none_id_value(backend),
            blob_value: SeaValue::Bytes(None),
            json_value: SeaValue::String(None),
        },
        Some(Value::Bool(value)) => DefaultValueColumns {
            str_value: SeaValue::String(None),
            i64_value: SeaValue::BigInt(None),
            f64_value: SeaValue::Double(None),
            bool_value: (*value).into(),
            time_value: SeaValue::BigInt(None),
            ref_value: none_id_value(backend),
            blob_value: SeaValue::Bytes(None),
            json_value: SeaValue::String(None),
        },
        Some(Value::Time(value)) => DefaultValueColumns {
            str_value: SeaValue::String(None),
            i64_value: SeaValue::BigInt(None),
            f64_value: SeaValue::Double(None),
            bool_value: SeaValue::Bool(None),
            time_value: value.0.into(),
            ref_value: none_id_value(backend),
            blob_value: SeaValue::Bytes(None),
            json_value: SeaValue::String(None),
        },
        Some(Value::Ref(value)) => DefaultValueColumns {
            str_value: SeaValue::String(None),
            i64_value: SeaValue::BigInt(None),
            f64_value: SeaValue::Double(None),
            bool_value: SeaValue::Bool(None),
            time_value: SeaValue::BigInt(None),
            ref_value: id_value(backend, *value),
            blob_value: SeaValue::Bytes(None),
            json_value: SeaValue::String(None),
        },
        Some(Value::Blob(value)) => DefaultValueColumns {
            str_value: SeaValue::String(None),
            i64_value: SeaValue::BigInt(None),
            f64_value: SeaValue::Double(None),
            bool_value: SeaValue::Bool(None),
            time_value: SeaValue::BigInt(None),
            ref_value: none_id_value(backend),
            blob_value: value.clone().into(),
            json_value: SeaValue::String(None),
        },
        Some(Value::Json(value)) => DefaultValueColumns {
            str_value: SeaValue::String(None),
            i64_value: SeaValue::BigInt(None),
            f64_value: SeaValue::Double(None),
            bool_value: SeaValue::Bool(None),
            time_value: SeaValue::BigInt(None),
            ref_value: none_id_value(backend),
            blob_value: SeaValue::Bytes(None),
            json_value: value.to_string().into(),
        },
        None => DefaultValueColumns {
            str_value: SeaValue::String(None),
            i64_value: SeaValue::BigInt(None),
            f64_value: SeaValue::Double(None),
            bool_value: SeaValue::Bool(None),
            time_value: SeaValue::BigInt(None),
            ref_value: none_id_value(backend),
            blob_value: SeaValue::Bytes(None),
            json_value: SeaValue::String(None),
        },
    }
}

fn id_value(backend: DatabaseBackend, id: Id) -> SeaValue {
    match backend {
        DatabaseBackend::Postgres => {
            let uuid = Uuid::from_bytes(id.as_bytes());
            SeaValue::Uuid(Some(uuid))
        }
        DatabaseBackend::MySql => SeaValue::Bytes(Some(id.as_vec())),
        DatabaseBackend::Sqlite => SeaValue::String(Some(id.to_uuid_string())),
        _ => SeaValue::String(Some(id.to_uuid_string())),
    }
}

fn none_id_value(backend: DatabaseBackend) -> SeaValue {
    match backend {
        DatabaseBackend::Postgres => SeaValue::Uuid(None),
        DatabaseBackend::MySql => SeaValue::Bytes(None),
        DatabaseBackend::Sqlite => SeaValue::String(None),
        _ => SeaValue::String(None),
    }
}

fn opt_id_value(backend: DatabaseBackend, id: Option<Id>) -> SeaValue {
    match id {
        Some(id) => id_value(backend, id),
        None => none_id_value(backend),
    }
}

fn bytes_to_id(bytes: Vec<u8>) -> Option<Id> {
    if bytes.len() == 16 {
        let mut buf = [0u8; 16];
        buf.copy_from_slice(&bytes);
        Some(Id::from_bytes(buf))
    } else {
        None
    }
}

fn read_id(row: &QueryResult, column: impl sea_query::Iden) -> MnemeResult<Id> {
    let name = col_name(column);
    if let Ok(value) = row.try_get::<String>("", &name) {
        return Id::from_uuid_str(&value);
    }
    if let Ok(value) = row.try_get::<Uuid>("", &name) {
        return Ok(Id::from_bytes(*value.as_bytes()));
    }
    if let Ok(value) = row.try_get::<Vec<u8>>("", &name) {
        return bytes_to_id(value).ok_or_else(|| MnemeError::storage("invalid id length"));
    }
    Err(MnemeError::storage("unsupported id format"))
}

fn read_opt_id(row: &QueryResult, column: impl sea_query::Iden) -> MnemeResult<Option<Id>> {
    let name = col_name(column);
    if let Ok(value) = row.try_get::<Option<String>>("", &name) {
        return value.map(|value| Id::from_uuid_str(&value)).transpose();
    }
    if let Ok(value) = row.try_get::<Option<Uuid>>("", &name) {
        return Ok(value.map(|value| Id::from_bytes(*value.as_bytes())));
    }
    if let Ok(value) = row.try_get::<Option<Vec<u8>>>("", &name) {
        return Ok(value.and_then(bytes_to_id));
    }
    Ok(None)
}

fn read_hlc(row: &QueryResult, column: impl sea_query::Iden) -> MnemeResult<Hlc> {
    let value: i64 = row.try_get("", &col_name(column))?;
    Ok(Hlc::from_i64(value))
}

fn col_name(column: impl sea_query::Iden) -> String {
    column.to_string()
}

fn build_stmt<S: QueryStatementWriter>(
    backend: DatabaseBackend,
    stmt: &S,
) -> (String, sea_orm::sea_query::Values) {
    match backend {
        DatabaseBackend::Sqlite => stmt.build(SqliteQueryBuilder),
        DatabaseBackend::Postgres => stmt.build(PostgresQueryBuilder),
        DatabaseBackend::MySql => stmt.build(MysqlQueryBuilder),
        _ => stmt.build(SqliteQueryBuilder),
    }
}

async fn exec<C, S>(conn: &C, stmt: &S) -> MnemeResult<()>
where
    C: ConnectionTrait,
    S: QueryStatementWriter,
{
    let backend = conn.get_database_backend();
    let (sql, values) = build_stmt(backend, stmt);
    conn.execute_raw(Statement::from_sql_and_values(backend, sql, values))
        .await?;
    Ok(())
}

async fn query_all<C, S>(conn: &C, stmt: &S) -> MnemeResult<Vec<QueryResult>>
where
    C: ConnectionTrait,
    S: QueryStatementWriter,
{
    let backend = conn.get_database_backend();
    let (sql, values) = build_stmt(backend, stmt);
    let rows = conn
        .query_all_raw(Statement::from_sql_and_values(backend, sql, values))
        .await?;
    Ok(rows)
}

async fn query_one<C, S>(conn: &C, stmt: &S) -> MnemeResult<Option<QueryResult>>
where
    C: ConnectionTrait,
    S: QueryStatementWriter,
{
    let backend = conn.get_database_backend();
    let (sql, values) = build_stmt(backend, stmt);
    let row = conn
        .query_one_raw(Statement::from_sql_and_values(backend, sql, values))
        .await?;
    Ok(row)
}

fn build_connection_url(config: &MnemeConfig, base_dir: &Path) -> MnemeResult<String> {
    match &config.database {
        crate::DatabaseConfig::Sqlite { .. } => {
            let path = config.sqlite_path(base_dir)?;
            Ok(format!("sqlite://{}?mode=rwc", path.display()))
        }
        crate::DatabaseConfig::Postgres { url } => Ok(url.clone()),
        crate::DatabaseConfig::Mysql { url } => Ok(url.clone()),
    }
}

fn scenario_id_from_payload(payload: &[u8]) -> MnemeResult<Option<crate::ScenarioId>> {
    let op: OpPayload =
        serde_json::from_slice(payload).map_err(|err| MnemeError::storage(err.to_string()))?;
    Ok(match op {
        OpPayload::CreateNode(input) => input.scenario_id,
        OpPayload::CreateEdge(input) => input.scenario_id,
        OpPayload::SetEdgeExistenceInterval(input) => input.scenario_id,
        OpPayload::TombstoneEntity { scenario_id, .. } => scenario_id,
        OpPayload::SetProperty(input) => input.scenario_id,
        OpPayload::ClearProperty(input) => input.scenario_id,
        OpPayload::OrSetUpdate(input) => input.scenario_id,
        OpPayload::CounterUpdate(input) => input.scenario_id,
        OpPayload::UpsertMetamodelBatch(_) => None,
        OpPayload::CreateScenario { scenario_id, .. } => Some(scenario_id),
        OpPayload::DeleteScenario { scenario_id, .. } => Some(scenario_id),
    })
}

const CHANGE_KIND_ENTITY: u8 = 1;
const CHANGE_KIND_EDGE: u8 = 2;
const CHANGE_KIND_PROPERTY: u8 = 3;
const CHANGE_KIND_SCHEMA: u8 = 4;
const CHANGE_KIND_SCENARIO: u8 = 5;

fn change_feed_payload(
    payload: &OpPayload,
) -> MnemeResult<(u8, Option<Id>, Option<serde_json::Value>)> {
    let (kind, entity_id) = match payload {
        OpPayload::CreateNode(input) => (CHANGE_KIND_ENTITY, Some(input.node_id)),
        OpPayload::CreateEdge(input) => (CHANGE_KIND_EDGE, Some(input.edge_id)),
        OpPayload::SetEdgeExistenceInterval(input) => (CHANGE_KIND_EDGE, Some(input.edge_id)),
        OpPayload::TombstoneEntity { entity_id, .. } => (CHANGE_KIND_ENTITY, Some(*entity_id)),
        OpPayload::SetProperty(input) => (CHANGE_KIND_PROPERTY, Some(input.entity_id)),
        OpPayload::ClearProperty(input) => (CHANGE_KIND_PROPERTY, Some(input.entity_id)),
        OpPayload::OrSetUpdate(input) => (CHANGE_KIND_PROPERTY, Some(input.entity_id)),
        OpPayload::CounterUpdate(input) => (CHANGE_KIND_PROPERTY, Some(input.entity_id)),
        OpPayload::UpsertMetamodelBatch(_) => (CHANGE_KIND_SCHEMA, None),
        OpPayload::CreateScenario { .. } => (CHANGE_KIND_SCENARIO, None),
        OpPayload::DeleteScenario { .. } => (CHANGE_KIND_SCENARIO, None),
    };
    Ok((kind, entity_id, None))
}

async fn insert_property_fact(
    tx: &sea_orm::DatabaseTransaction,
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    entity_id: Id,
    field_id: Id,
    value: &Value,
    valid_from: ValidTime,
    valid_to: Option<ValidTime>,
    layer: Layer,
    asserted_at: Hlc,
    op_id: OpId,
    is_tombstone: bool,
    backend: DatabaseBackend,
) -> MnemeResult<()> {
    let value_type = value.value_type();
    let (table, column) = value_table_column(value_type);
    let insert = Query::insert()
        .into_table(table.clone())
        .columns([
            Alias::new("partition_id"),
            Alias::new("scenario_id"),
            Alias::new("entity_id"),
            Alias::new("field_id"),
            Alias::new("valid_from"),
            Alias::new("valid_to"),
            Alias::new("valid_bucket"),
            Alias::new("layer"),
            Alias::new("asserted_at_hlc"),
            Alias::new("op_id"),
            Alias::new("is_tombstone"),
            column,
        ])
        .values_panic([
            id_value(backend, partition.0).into(),
            opt_id_value(backend, scenario_id.map(|s| s.0)).into(),
            id_value(backend, entity_id).into(),
            id_value(backend, field_id).into(),
            valid_from.0.into(),
            valid_to.map(|v| v.0).into(),
            valid_bucket(valid_from).into(),
            (layer as i64).into(),
            asserted_at.as_i64().into(),
            id_value(backend, op_id.0).into(),
            is_tombstone.into(),
            value_to_sea(backend, value).into(),
        ])
        .to_owned();
    exec(tx, &insert).await
}

async fn insert_index_row(
    tx: &sea_orm::DatabaseTransaction,
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    field_id: Id,
    entity_id: Id,
    value: &Value,
    valid_from: ValidTime,
    valid_to: Option<ValidTime>,
    asserted_at: Hlc,
    layer: Layer,
    backend: DatabaseBackend,
) -> MnemeResult<()> {
    match value {
        Value::Str(text) => {
            let norm = normalize_index_text(text);
            let insert = Query::insert()
                .into_table(AideonIdxFieldStr::Table)
                .columns([
                    AideonIdxFieldStr::PartitionId,
                    AideonIdxFieldStr::ScenarioId,
                    AideonIdxFieldStr::FieldId,
                    AideonIdxFieldStr::ValueTextNorm,
                    AideonIdxFieldStr::EntityId,
                    AideonIdxFieldStr::ValidFrom,
                    AideonIdxFieldStr::ValidTo,
                    AideonIdxFieldStr::ValidBucket,
                    AideonIdxFieldStr::AssertedAtHlc,
                    AideonIdxFieldStr::Layer,
                ])
                .values_panic([
                    id_value(backend, partition.0).into(),
                    opt_id_value(backend, scenario_id.map(|s| s.0)).into(),
                    id_value(backend, field_id).into(),
                    norm.into(),
                    id_value(backend, entity_id).into(),
                    valid_from.0.into(),
                    valid_to.map(|v| v.0).into(),
                    valid_bucket(valid_from).into(),
                    asserted_at.as_i64().into(),
                    (layer as i64).into(),
                ])
                .to_owned();
            exec(tx, &insert).await?;
        }
        Value::I64(value) => {
            let insert = Query::insert()
                .into_table(AideonIdxFieldI64::Table)
                .columns([
                    AideonIdxFieldI64::PartitionId,
                    AideonIdxFieldI64::ScenarioId,
                    AideonIdxFieldI64::FieldId,
                    AideonIdxFieldI64::ValueI64,
                    AideonIdxFieldI64::EntityId,
                    AideonIdxFieldI64::ValidFrom,
                    AideonIdxFieldI64::ValidTo,
                    AideonIdxFieldI64::ValidBucket,
                    AideonIdxFieldI64::AssertedAtHlc,
                    AideonIdxFieldI64::Layer,
                ])
                .values_panic([
                    id_value(backend, partition.0).into(),
                    opt_id_value(backend, scenario_id.map(|s| s.0)).into(),
                    id_value(backend, field_id).into(),
                    (*value).into(),
                    id_value(backend, entity_id).into(),
                    valid_from.0.into(),
                    valid_to.map(|v| v.0).into(),
                    valid_bucket(valid_from).into(),
                    asserted_at.as_i64().into(),
                    (layer as i64).into(),
                ])
                .to_owned();
            exec(tx, &insert).await?;
        }
        Value::F64(value) => {
            let insert = Query::insert()
                .into_table(AideonIdxFieldF64::Table)
                .columns([
                    AideonIdxFieldF64::PartitionId,
                    AideonIdxFieldF64::ScenarioId,
                    AideonIdxFieldF64::FieldId,
                    AideonIdxFieldF64::ValueF64,
                    AideonIdxFieldF64::EntityId,
                    AideonIdxFieldF64::ValidFrom,
                    AideonIdxFieldF64::ValidTo,
                    AideonIdxFieldF64::ValidBucket,
                    AideonIdxFieldF64::AssertedAtHlc,
                    AideonIdxFieldF64::Layer,
                ])
                .values_panic([
                    id_value(backend, partition.0).into(),
                    opt_id_value(backend, scenario_id.map(|s| s.0)).into(),
                    id_value(backend, field_id).into(),
                    (*value).into(),
                    id_value(backend, entity_id).into(),
                    valid_from.0.into(),
                    valid_to.map(|v| v.0).into(),
                    valid_bucket(valid_from).into(),
                    asserted_at.as_i64().into(),
                    (layer as i64).into(),
                ])
                .to_owned();
            exec(tx, &insert).await?;
        }
        Value::Bool(value) => {
            let insert = Query::insert()
                .into_table(AideonIdxFieldBool::Table)
                .columns([
                    AideonIdxFieldBool::PartitionId,
                    AideonIdxFieldBool::ScenarioId,
                    AideonIdxFieldBool::FieldId,
                    AideonIdxFieldBool::ValueBool,
                    AideonIdxFieldBool::EntityId,
                    AideonIdxFieldBool::ValidFrom,
                    AideonIdxFieldBool::ValidTo,
                    AideonIdxFieldBool::ValidBucket,
                    AideonIdxFieldBool::AssertedAtHlc,
                    AideonIdxFieldBool::Layer,
                ])
                .values_panic([
                    id_value(backend, partition.0).into(),
                    opt_id_value(backend, scenario_id.map(|s| s.0)).into(),
                    id_value(backend, field_id).into(),
                    (*value).into(),
                    id_value(backend, entity_id).into(),
                    valid_from.0.into(),
                    valid_to.map(|v| v.0).into(),
                    valid_bucket(valid_from).into(),
                    asserted_at.as_i64().into(),
                    (layer as i64).into(),
                ])
                .to_owned();
            exec(tx, &insert).await?;
        }
        Value::Time(value) => {
            let insert = Query::insert()
                .into_table(AideonIdxFieldTime::Table)
                .columns([
                    AideonIdxFieldTime::PartitionId,
                    AideonIdxFieldTime::ScenarioId,
                    AideonIdxFieldTime::FieldId,
                    AideonIdxFieldTime::ValueTime,
                    AideonIdxFieldTime::EntityId,
                    AideonIdxFieldTime::ValidFrom,
                    AideonIdxFieldTime::ValidTo,
                    AideonIdxFieldTime::ValidBucket,
                    AideonIdxFieldTime::AssertedAtHlc,
                    AideonIdxFieldTime::Layer,
                ])
                .values_panic([
                    id_value(backend, partition.0).into(),
                    opt_id_value(backend, scenario_id.map(|s| s.0)).into(),
                    id_value(backend, field_id).into(),
                    value.0.into(),
                    id_value(backend, entity_id).into(),
                    valid_from.0.into(),
                    valid_to.map(|v| v.0).into(),
                    valid_bucket(valid_from).into(),
                    asserted_at.as_i64().into(),
                    (layer as i64).into(),
                ])
                .to_owned();
            exec(tx, &insert).await?;
        }
        Value::Ref(value) => {
            let insert = Query::insert()
                .into_table(AideonIdxFieldRef::Table)
                .columns([
                    AideonIdxFieldRef::PartitionId,
                    AideonIdxFieldRef::ScenarioId,
                    AideonIdxFieldRef::FieldId,
                    AideonIdxFieldRef::ValueRefEntityId,
                    AideonIdxFieldRef::EntityId,
                    AideonIdxFieldRef::ValidFrom,
                    AideonIdxFieldRef::ValidTo,
                    AideonIdxFieldRef::ValidBucket,
                    AideonIdxFieldRef::AssertedAtHlc,
                    AideonIdxFieldRef::Layer,
                ])
                .values_panic([
                    id_value(backend, partition.0).into(),
                    opt_id_value(backend, scenario_id.map(|s| s.0)).into(),
                    id_value(backend, field_id).into(),
                    id_value(backend, *value).into(),
                    id_value(backend, entity_id).into(),
                    valid_from.0.into(),
                    valid_to.map(|v| v.0).into(),
                    valid_bucket(valid_from).into(),
                    asserted_at.as_i64().into(),
                    (layer as i64).into(),
                ])
                .to_owned();
            exec(tx, &insert).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn delete_index_row(
    tx: &sea_orm::DatabaseTransaction,
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    field_id: Id,
    entity_id: Id,
    value: &Value,
    backend: DatabaseBackend,
) -> MnemeResult<()> {
    match value {
        Value::Str(text) => {
            let norm = normalize_index_text(text);
            let mut delete = Query::delete()
                .from_table(AideonIdxFieldStr::Table)
                .and_where(Expr::col(AideonIdxFieldStr::PartitionId).eq(id_value(backend, partition.0)))
                .and_where(Expr::col(AideonIdxFieldStr::FieldId).eq(id_value(backend, field_id)))
                .and_where(Expr::col(AideonIdxFieldStr::EntityId).eq(id_value(backend, entity_id)))
                .and_where(Expr::col(AideonIdxFieldStr::ValueTextNorm).eq(norm))
                .to_owned();
            if let Some(scenario_id) = scenario_id {
                delete.and_where(
                    Expr::col(AideonIdxFieldStr::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                delete.and_where(Expr::col(AideonIdxFieldStr::ScenarioId).is_null());
            }
            exec(tx, &delete).await?;
        }
        Value::I64(value) => {
            let mut delete = Query::delete()
                .from_table(AideonIdxFieldI64::Table)
                .and_where(Expr::col(AideonIdxFieldI64::PartitionId).eq(id_value(backend, partition.0)))
                .and_where(Expr::col(AideonIdxFieldI64::FieldId).eq(id_value(backend, field_id)))
                .and_where(Expr::col(AideonIdxFieldI64::EntityId).eq(id_value(backend, entity_id)))
                .and_where(Expr::col(AideonIdxFieldI64::ValueI64).eq(*value))
                .to_owned();
            if let Some(scenario_id) = scenario_id {
                delete.and_where(
                    Expr::col(AideonIdxFieldI64::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                delete.and_where(Expr::col(AideonIdxFieldI64::ScenarioId).is_null());
            }
            exec(tx, &delete).await?;
        }
        Value::F64(value) => {
            let mut delete = Query::delete()
                .from_table(AideonIdxFieldF64::Table)
                .and_where(Expr::col(AideonIdxFieldF64::PartitionId).eq(id_value(backend, partition.0)))
                .and_where(Expr::col(AideonIdxFieldF64::FieldId).eq(id_value(backend, field_id)))
                .and_where(Expr::col(AideonIdxFieldF64::EntityId).eq(id_value(backend, entity_id)))
                .and_where(Expr::col(AideonIdxFieldF64::ValueF64).eq(*value))
                .to_owned();
            if let Some(scenario_id) = scenario_id {
                delete.and_where(
                    Expr::col(AideonIdxFieldF64::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                delete.and_where(Expr::col(AideonIdxFieldF64::ScenarioId).is_null());
            }
            exec(tx, &delete).await?;
        }
        Value::Bool(value) => {
            let mut delete = Query::delete()
                .from_table(AideonIdxFieldBool::Table)
                .and_where(Expr::col(AideonIdxFieldBool::PartitionId).eq(id_value(backend, partition.0)))
                .and_where(Expr::col(AideonIdxFieldBool::FieldId).eq(id_value(backend, field_id)))
                .and_where(Expr::col(AideonIdxFieldBool::EntityId).eq(id_value(backend, entity_id)))
                .and_where(Expr::col(AideonIdxFieldBool::ValueBool).eq(*value))
                .to_owned();
            if let Some(scenario_id) = scenario_id {
                delete.and_where(
                    Expr::col(AideonIdxFieldBool::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                delete.and_where(Expr::col(AideonIdxFieldBool::ScenarioId).is_null());
            }
            exec(tx, &delete).await?;
        }
        Value::Time(value) => {
            let mut delete = Query::delete()
                .from_table(AideonIdxFieldTime::Table)
                .and_where(Expr::col(AideonIdxFieldTime::PartitionId).eq(id_value(backend, partition.0)))
                .and_where(Expr::col(AideonIdxFieldTime::FieldId).eq(id_value(backend, field_id)))
                .and_where(Expr::col(AideonIdxFieldTime::EntityId).eq(id_value(backend, entity_id)))
                .and_where(Expr::col(AideonIdxFieldTime::ValueTime).eq(value.0))
                .to_owned();
            if let Some(scenario_id) = scenario_id {
                delete.and_where(
                    Expr::col(AideonIdxFieldTime::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                delete.and_where(Expr::col(AideonIdxFieldTime::ScenarioId).is_null());
            }
            exec(tx, &delete).await?;
        }
        Value::Ref(value) => {
            let mut delete = Query::delete()
                .from_table(AideonIdxFieldRef::Table)
                .and_where(Expr::col(AideonIdxFieldRef::PartitionId).eq(id_value(backend, partition.0)))
                .and_where(Expr::col(AideonIdxFieldRef::FieldId).eq(id_value(backend, field_id)))
                .and_where(Expr::col(AideonIdxFieldRef::EntityId).eq(id_value(backend, entity_id)))
                .and_where(Expr::col(AideonIdxFieldRef::ValueRefEntityId).eq(id_value(backend, *value)))
                .to_owned();
            if let Some(scenario_id) = scenario_id {
                delete.and_where(
                    Expr::col(AideonIdxFieldRef::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                delete.and_where(Expr::col(AideonIdxFieldRef::ScenarioId).is_null());
            }
            exec(tx, &delete).await?;
        }
        _ => {}
    }
    Ok(())
}

fn value_table_column(value_type: ValueType) -> (Alias, Alias) {
    match value_type {
        ValueType::Str => (Alias::new("aideon_prop_fact_str"), Alias::new("value_text")),
        ValueType::I64 => (Alias::new("aideon_prop_fact_i64"), Alias::new("value_i64")),
        ValueType::F64 => (Alias::new("aideon_prop_fact_f64"), Alias::new("value_f64")),
        ValueType::Bool => (
            Alias::new("aideon_prop_fact_bool"),
            Alias::new("value_bool"),
        ),
        ValueType::Time => (
            Alias::new("aideon_prop_fact_time"),
            Alias::new("value_time"),
        ),
        ValueType::Ref => (
            Alias::new("aideon_prop_fact_ref"),
            Alias::new("value_ref_entity_id"),
        ),
        ValueType::Blob => (
            Alias::new("aideon_prop_fact_blob"),
            Alias::new("value_blob"),
        ),
        ValueType::Json => (
            Alias::new("aideon_prop_fact_json"),
            Alias::new("value_json"),
        ),
    }
}

fn value_to_sea(backend: DatabaseBackend, value: &Value) -> SeaValue {
    match value {
        Value::Str(value) => value.clone().into(),
        Value::I64(value) => (*value).into(),
        Value::F64(value) => (*value).into(),
        Value::Bool(value) => (*value).into(),
        Value::Time(value) => value.0.into(),
        Value::Ref(value) => id_value(backend, *value),
        Value::Blob(value) => value.clone().into(),
        Value::Json(value) => value.to_string().into(),
    }
}

async fn fetch_property_facts_with_fallback(
    conn: &DatabaseConnection,
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    entity_id: Id,
    field_id: Id,
    at_valid_time: ValidTime,
    as_of_asserted_at: Option<Hlc>,
    value_type: ValueType,
    backend: DatabaseBackend,
) -> MnemeResult<Vec<PropertyFact>> {
    let facts = fetch_property_facts_in_partition(
        conn,
        partition,
        scenario_id,
        entity_id,
        field_id,
        at_valid_time,
        as_of_asserted_at,
        value_type,
        backend,
    )
    .await?;
    if !facts.is_empty() {
        return Ok(facts);
    }
    if scenario_id.is_some() {
        return fetch_property_facts_in_partition(
            conn,
            partition,
            None,
            entity_id,
            field_id,
            at_valid_time,
            as_of_asserted_at,
            value_type,
            backend,
        )
        .await;
    }
    Ok(facts)
}

async fn fetch_property_facts_in_partition(
    conn: &DatabaseConnection,
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    entity_id: Id,
    field_id: Id,
    at_valid_time: ValidTime,
    as_of_asserted_at: Option<Hlc>,
    value_type: ValueType,
    backend: DatabaseBackend,
) -> MnemeResult<Vec<PropertyFact>> {
    let (table, column) = value_table_column(value_type);
    let mut select = Query::select()
        .from(table.clone())
        .columns([
            column.clone(),
            Alias::new("valid_from"),
            Alias::new("valid_to"),
            Alias::new("layer"),
            Alias::new("asserted_at_hlc"),
            Alias::new("is_tombstone"),
            Alias::new("op_id"),
        ])
        .and_where(
            Expr::col((table.clone(), Alias::new("partition_id")))
                .eq(id_value(backend, partition.0)),
        )
        .and_where(
            Expr::col((table.clone(), Alias::new("entity_id")))
                .eq(id_value(backend, entity_id)),
        )
        .and_where(
            Expr::col((table.clone(), Alias::new("field_id")))
                .eq(id_value(backend, field_id)),
        )
        .and_where(Expr::col((table.clone(), Alias::new("valid_from"))).lte(at_valid_time.0))
        .and_where(
            Expr::col((table.clone(), Alias::new("valid_to")))
                .gt(at_valid_time.0)
                .or(Expr::col((table.clone(), Alias::new("valid_to"))).is_null()),
        )
        .to_owned();
    if let Some(scenario_id) = scenario_id {
        select.and_where(
            Expr::col((table.clone(), Alias::new("scenario_id")))
                .eq(id_value(backend, scenario_id.0)),
        );
    } else {
        select.and_where(Expr::col((table.clone(), Alias::new("scenario_id"))).is_null());
    }
    if let Some(as_of) = as_of_asserted_at {
        select.and_where(
            Expr::col((table.clone(), Alias::new("asserted_at_hlc"))).lte(as_of.as_i64()),
        );
    }
    let rows = query_all(conn, &select).await?;
    let mut facts = Vec::new();
    for row in rows {
        let value = read_value(value_type, &row, &col_name(column.clone()))?;
        let valid_from: i64 = row.try_get("", "valid_from")?;
        let valid_to: Option<i64> = row.try_get("", "valid_to")?;
        let layer: i64 = row.try_get("", "layer")?;
        let asserted_at = read_hlc_by_name(&row, "asserted_at_hlc")?;
        let is_tombstone: bool = row.try_get("", "is_tombstone")?;
        let op_id = read_id_by_name(&row, "op_id")?;
        facts.push(PropertyFact {
            value,
            valid_from,
            valid_to,
            layer,
            asserted_at,
            op_id,
            is_tombstone,
        });
    }
    Ok(facts)
}

async fn fetch_edge_facts_in_partition(
    conn: &DatabaseConnection,
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    input: &TraverseAtTimeInput,
    backend: DatabaseBackend,
) -> MnemeResult<Vec<EdgeFact>> {
    let direction_col = match input.direction {
        Direction::Out => AideonEdges::SrcEntityId,
        Direction::In => AideonEdges::DstEntityId,
    };
    let mut select = Query::select()
        .from(AideonEdges::Table)
        .inner_join(
            AideonEdgeExistsFacts::Table,
            Expr::col((AideonEdges::Table, AideonEdges::PartitionId))
                .equals((
                    AideonEdgeExistsFacts::Table,
                    AideonEdgeExistsFacts::PartitionId,
                ))
                .and(
                    Expr::col((AideonEdges::Table, AideonEdges::EdgeId))
                        .equals((AideonEdgeExistsFacts::Table, AideonEdgeExistsFacts::EdgeId)),
                ),
        )
        .column((AideonEdges::Table, AideonEdges::EdgeId))
        .column((AideonEdges::Table, AideonEdges::SrcEntityId))
        .column((AideonEdges::Table, AideonEdges::DstEntityId))
        .column((AideonEdges::Table, AideonEdges::EdgeTypeId))
        .column((
            AideonEdgeExistsFacts::Table,
            AideonEdgeExistsFacts::ValidFrom,
        ))
        .column((AideonEdgeExistsFacts::Table, AideonEdgeExistsFacts::ValidTo))
        .column((AideonEdgeExistsFacts::Table, AideonEdgeExistsFacts::Layer))
        .column((
            AideonEdgeExistsFacts::Table,
            AideonEdgeExistsFacts::AssertedAtHlc,
        ))
        .column((
            AideonEdgeExistsFacts::Table,
            AideonEdgeExistsFacts::IsTombstone,
        ))
        .column((AideonEdgeExistsFacts::Table, AideonEdgeExistsFacts::OpId))
        .and_where(
            Expr::col((AideonEdges::Table, AideonEdges::PartitionId))
                .eq(id_value(backend, partition.0)),
        )
        .and_where(
            Expr::col((AideonEdges::Table, direction_col))
                .eq(id_value(backend, input.from_entity_id)),
        )
        .and_where(
            Expr::col((
                AideonEdgeExistsFacts::Table,
                AideonEdgeExistsFacts::ValidFrom,
            ))
            .lte(input.at_valid_time.0),
        )
        .and_where(
            Expr::col((AideonEdgeExistsFacts::Table, AideonEdgeExistsFacts::ValidTo))
                .gt(input.at_valid_time.0)
                .or(
                    Expr::col((AideonEdgeExistsFacts::Table, AideonEdgeExistsFacts::ValidTo))
                        .is_null(),
                ),
        )
        .to_owned();
    if let Some(scenario_id) = scenario_id {
        select.and_where(
            Expr::col((AideonEdges::Table, AideonEdges::ScenarioId))
                .eq(id_value(backend, scenario_id.0)),
        );
        select.and_where(
            Expr::col((AideonEdgeExistsFacts::Table, AideonEdgeExistsFacts::ScenarioId))
                .eq(id_value(backend, scenario_id.0)),
        );
    } else {
        select.and_where(Expr::col((AideonEdges::Table, AideonEdges::ScenarioId)).is_null());
        select
            .and_where(Expr::col((AideonEdgeExistsFacts::Table, AideonEdgeExistsFacts::ScenarioId)).is_null());
    }
    if let Some(as_of) = input.as_of_asserted_at {
        select.and_where(
            Expr::col((
                AideonEdgeExistsFacts::Table,
                AideonEdgeExistsFacts::AssertedAtHlc,
            ))
            .lte(as_of.as_i64()),
        );
    }
    if let Some(edge_type_id) = input.edge_type_id {
        select.and_where(
            Expr::col((AideonEdges::Table, AideonEdges::EdgeTypeId))
                .eq(id_value(backend, edge_type_id)),
        );
    }
    let rows = query_all(conn, &select).await?;
    let mut facts = Vec::new();
    for row in rows {
        let edge_id = read_id(&row, AideonEdges::EdgeId)?;
        let src_id = read_id(&row, AideonEdges::SrcEntityId)?;
        let dst_id = read_id(&row, AideonEdges::DstEntityId)?;
        let edge_type_id = read_opt_id(&row, AideonEdges::EdgeTypeId)?;
        let valid_from: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::ValidFrom))?;
        let valid_to: Option<i64> = row.try_get("", &col_name(AideonEdgeExistsFacts::ValidTo))?;
        let layer: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::Layer))?;
        let asserted_at = read_hlc(&row, AideonEdgeExistsFacts::AssertedAtHlc)?;
        let is_tombstone: bool = row.try_get("", &col_name(AideonEdgeExistsFacts::IsTombstone))?;
        let op_id = read_id(&row, AideonEdgeExistsFacts::OpId)?;
        facts.push(EdgeFact {
            edge_id,
            src_id,
            dst_id,
            edge_type_id,
            valid_from,
            valid_to,
            layer,
            asserted_at,
            op_id,
            is_tombstone,
        });
    }
    Ok(facts)
}

async fn fetch_projection_edges_in_partition(
    conn: &DatabaseConnection,
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    at_valid_time: Option<ValidTime>,
    as_of_asserted_at: Option<Hlc>,
    edge_type_filter: Option<Vec<Id>>,
    backend: DatabaseBackend,
) -> MnemeResult<Vec<(Id, ProjectionEdge)>> {
    let mut select = Query::select()
        .from(AideonGraphProjectionEdges::Table)
        .columns([
            (
                AideonGraphProjectionEdges::Table,
                AideonGraphProjectionEdges::EdgeId,
            ),
            (
                AideonGraphProjectionEdges::Table,
                AideonGraphProjectionEdges::SrcEntityId,
            ),
            (
                AideonGraphProjectionEdges::Table,
                AideonGraphProjectionEdges::DstEntityId,
            ),
            (
                AideonGraphProjectionEdges::Table,
                AideonGraphProjectionEdges::EdgeTypeId,
            ),
            (
                AideonGraphProjectionEdges::Table,
                AideonGraphProjectionEdges::Weight,
            ),
        ])
        .and_where(
            Expr::col(AideonGraphProjectionEdges::PartitionId).eq(id_value(backend, partition.0)),
        )
        .to_owned();
    if let Some(scenario_id) = scenario_id {
        select.and_where(
            Expr::col(AideonGraphProjectionEdges::ScenarioId)
                .eq(id_value(backend, scenario_id.0)),
        );
    } else {
        select.and_where(Expr::col(AideonGraphProjectionEdges::ScenarioId).is_null());
    }
    if let Some(filter) = edge_type_filter {
        select.and_where(
            Expr::col(AideonGraphProjectionEdges::EdgeTypeId)
                .is_in(filter.into_iter().map(|id| id_value(backend, id))),
        );
    }
    let rows = query_all(conn, &select).await?;
    let mut edges = Vec::new();
    for row in rows {
        let edge_id = read_id(&row, AideonGraphProjectionEdges::EdgeId)?;
        if let Some(at_valid_time) = at_valid_time {
            let exists = edge_exists_at(
                conn,
                partition,
                scenario_id,
                edge_id,
                at_valid_time,
                as_of_asserted_at,
                backend,
            )
            .await?;
            if !exists {
                continue;
            }
        }
        let src_id = read_id(&row, AideonGraphProjectionEdges::SrcEntityId)?;
        let dst_id = read_id(&row, AideonGraphProjectionEdges::DstEntityId)?;
        let edge_type_id = read_opt_id(&row, AideonGraphProjectionEdges::EdgeTypeId)?;
        let weight: f64 = row.try_get("", &col_name(AideonGraphProjectionEdges::Weight))?;
        edges.push((
            edge_id,
            ProjectionEdge {
                edge_id,
                src_id,
                dst_id,
                edge_type_id,
                weight,
            },
        ));
    }
    Ok(edges)
}

fn parse_cursor_id(value: &str) -> MnemeResult<Id> {
    Id::from_uuid_str(value).or_else(|_| Id::from_ulid_str(value))
}

async fn list_entities_without_filters(
    conn: &DatabaseConnection,
    input: ListEntitiesInput,
    backend: DatabaseBackend,
) -> MnemeResult<Vec<ListEntitiesResultItem>> {
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let scenarios = if let Some(scenario_id) = input.scenario_id {
        vec![Some(scenario_id), None]
    } else {
        vec![None]
    };
    for scenario_id in scenarios {
        let mut select = Query::select()
            .from(AideonEntities::Table)
            .columns([
                AideonEntities::EntityId,
                AideonEntities::EntityKind,
                AideonEntities::TypeId,
                AideonEntities::IsDeleted,
            ])
            .and_where(
                Expr::col(AideonEntities::PartitionId).eq(id_value(backend, input.partition.0)),
            )
            .to_owned();
        if let Some(scenario_id) = scenario_id {
            select.and_where(
                Expr::col(AideonEntities::ScenarioId).eq(id_value(backend, scenario_id.0)),
            );
        } else {
            select.and_where(Expr::col(AideonEntities::ScenarioId).is_null());
        }
        if let Some(kind) = input.kind {
            select.and_where(
                Expr::col(AideonEntities::EntityKind).eq(kind.as_i16() as i64),
            );
        }
        if let Some(type_id) = input.type_id {
            select
                .and_where(Expr::col(AideonEntities::TypeId).eq(id_value(backend, type_id)));
        }
        let rows = query_all(conn, &select).await?;
        for row in rows {
            let entity_id = read_id(&row, AideonEntities::EntityId)?;
            if seen.contains(&entity_id) {
                continue;
            }
            let kind_raw: i16 = row.try_get("", &col_name(AideonEntities::EntityKind))?;
            let kind = EntityKind::from_i16(kind_raw)
                .ok_or_else(|| MnemeError::storage("invalid entity kind".to_string()))?;
            let type_id = read_opt_id(&row, AideonEntities::TypeId)?;
            let is_deleted: bool = row.try_get("", &col_name(AideonEntities::IsDeleted))?;
            if is_deleted {
                continue;
            }
            seen.insert(entity_id);
            results.push(ListEntitiesResultItem {
                entity_id,
                kind,
                type_id,
            });
        }
    }
    results.sort_by(|a, b| a.entity_id.as_bytes().cmp(&b.entity_id.as_bytes()));
    if let Some(cursor) = input.cursor.as_deref() {
        let cursor_id = parse_cursor_id(cursor)?;
        results.retain(|item| item.entity_id.as_bytes() > cursor_id.as_bytes());
    }
    if results.len() > input.limit as usize {
        results.truncate(input.limit as usize);
    }
    Ok(results)
}

async fn fetch_index_candidates(
    conn: &DatabaseConnection,
    partition: PartitionId,
    scenario_id: Option<ScenarioId>,
    at_valid_time: ValidTime,
    as_of_asserted_at: Option<Hlc>,
    filter: &FieldFilter,
    backend: DatabaseBackend,
) -> MnemeResult<std::collections::HashSet<Id>> {
    let mut ids = std::collections::HashSet::new();
    match &filter.value {
        Value::Str(value) => {
            let norm = normalize_index_text(value);
            let mut select = Query::select()
                .from(AideonIdxFieldStr::Table)
                .column(AideonIdxFieldStr::EntityId)
                .and_where(
                    Expr::col(AideonIdxFieldStr::PartitionId).eq(id_value(backend, partition.0)),
                )
                .and_where(
                    Expr::col(AideonIdxFieldStr::FieldId).eq(id_value(backend, filter.field_id)),
                )
                .and_where(Expr::col(AideonIdxFieldStr::ValidFrom).lte(at_valid_time.0))
                .and_where(
                    Expr::col(AideonIdxFieldStr::ValidTo)
                        .gt(at_valid_time.0)
                        .or(Expr::col(AideonIdxFieldStr::ValidTo).is_null()),
                )
                .to_owned();
            if let Some(as_of_asserted_at) = as_of_asserted_at {
                select.and_where(
                    Expr::col(AideonIdxFieldStr::AssertedAtHlc).lte(as_of_asserted_at.as_i64()),
                );
            }
            if let Some(scenario_id) = scenario_id {
                select.and_where(
                    Expr::col(AideonIdxFieldStr::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                select.and_where(Expr::col(AideonIdxFieldStr::ScenarioId).is_null());
            }
            match filter.op {
                CompareOp::Eq => {
                    select.and_where(Expr::col(AideonIdxFieldStr::ValueTextNorm).eq(norm));
                }
                CompareOp::Ne => {
                    select.and_where(Expr::col(AideonIdxFieldStr::ValueTextNorm).ne(norm));
                }
                CompareOp::Lt => {
                    select.and_where(Expr::col(AideonIdxFieldStr::ValueTextNorm).lt(norm));
                }
                CompareOp::Lte => {
                    select.and_where(Expr::col(AideonIdxFieldStr::ValueTextNorm).lte(norm));
                }
                CompareOp::Gt => {
                    select.and_where(Expr::col(AideonIdxFieldStr::ValueTextNorm).gt(norm));
                }
                CompareOp::Gte => {
                    select.and_where(Expr::col(AideonIdxFieldStr::ValueTextNorm).gte(norm));
                }
                CompareOp::Prefix => {
                    select.and_where(
                        Expr::col(AideonIdxFieldStr::ValueTextNorm)
                            .like(format!("{norm}%")),
                    );
                }
                CompareOp::Contains => {
                    select.and_where(
                        Expr::col(AideonIdxFieldStr::ValueTextNorm)
                            .like(format!("%{norm}%")),
                    );
                }
            }
            let rows = query_all(conn, &select).await?;
            for row in rows {
                let entity_id = read_id(&row, AideonIdxFieldStr::EntityId)?;
                ids.insert(entity_id);
            }
        }
        Value::I64(value) => {
            let mut select = Query::select()
                .from(AideonIdxFieldI64::Table)
                .column(AideonIdxFieldI64::EntityId)
                .and_where(
                    Expr::col(AideonIdxFieldI64::PartitionId).eq(id_value(backend, partition.0)),
                )
                .and_where(
                    Expr::col(AideonIdxFieldI64::FieldId).eq(id_value(backend, filter.field_id)),
                )
                .and_where(Expr::col(AideonIdxFieldI64::ValidFrom).lte(at_valid_time.0))
                .and_where(
                    Expr::col(AideonIdxFieldI64::ValidTo)
                        .gt(at_valid_time.0)
                        .or(Expr::col(AideonIdxFieldI64::ValidTo).is_null()),
                )
                .to_owned();
            if let Some(as_of_asserted_at) = as_of_asserted_at {
                select.and_where(
                    Expr::col(AideonIdxFieldI64::AssertedAtHlc).lte(as_of_asserted_at.as_i64()),
                );
            }
            if let Some(scenario_id) = scenario_id {
                select.and_where(
                    Expr::col(AideonIdxFieldI64::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                select.and_where(Expr::col(AideonIdxFieldI64::ScenarioId).is_null());
            }
            match filter.op {
                CompareOp::Eq => {
                    select.and_where(Expr::col(AideonIdxFieldI64::ValueI64).eq(*value));
                }
                CompareOp::Ne => {
                    select.and_where(Expr::col(AideonIdxFieldI64::ValueI64).ne(*value));
                }
                CompareOp::Lt => {
                    select.and_where(Expr::col(AideonIdxFieldI64::ValueI64).lt(*value));
                }
                CompareOp::Lte => {
                    select.and_where(Expr::col(AideonIdxFieldI64::ValueI64).lte(*value));
                }
                CompareOp::Gt => {
                    select.and_where(Expr::col(AideonIdxFieldI64::ValueI64).gt(*value));
                }
                CompareOp::Gte => {
                    select.and_where(Expr::col(AideonIdxFieldI64::ValueI64).gte(*value));
                }
                CompareOp::Prefix | CompareOp::Contains => {
                    return Err(MnemeError::invalid("string compare op on i64 field"));
                }
            }
            let rows = query_all(conn, &select).await?;
            for row in rows {
                let entity_id = read_id(&row, AideonIdxFieldI64::EntityId)?;
                ids.insert(entity_id);
            }
        }
        Value::F64(value) => {
            let mut select = Query::select()
                .from(AideonIdxFieldF64::Table)
                .column(AideonIdxFieldF64::EntityId)
                .and_where(
                    Expr::col(AideonIdxFieldF64::PartitionId).eq(id_value(backend, partition.0)),
                )
                .and_where(
                    Expr::col(AideonIdxFieldF64::FieldId).eq(id_value(backend, filter.field_id)),
                )
                .and_where(Expr::col(AideonIdxFieldF64::ValidFrom).lte(at_valid_time.0))
                .and_where(
                    Expr::col(AideonIdxFieldF64::ValidTo)
                        .gt(at_valid_time.0)
                        .or(Expr::col(AideonIdxFieldF64::ValidTo).is_null()),
                )
                .to_owned();
            if let Some(as_of_asserted_at) = as_of_asserted_at {
                select.and_where(
                    Expr::col(AideonIdxFieldF64::AssertedAtHlc).lte(as_of_asserted_at.as_i64()),
                );
            }
            if let Some(scenario_id) = scenario_id {
                select.and_where(
                    Expr::col(AideonIdxFieldF64::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                select.and_where(Expr::col(AideonIdxFieldF64::ScenarioId).is_null());
            }
            match filter.op {
                CompareOp::Eq => {
                    select.and_where(Expr::col(AideonIdxFieldF64::ValueF64).eq(*value));
                }
                CompareOp::Ne => {
                    select.and_where(Expr::col(AideonIdxFieldF64::ValueF64).ne(*value));
                }
                CompareOp::Lt => {
                    select.and_where(Expr::col(AideonIdxFieldF64::ValueF64).lt(*value));
                }
                CompareOp::Lte => {
                    select.and_where(Expr::col(AideonIdxFieldF64::ValueF64).lte(*value));
                }
                CompareOp::Gt => {
                    select.and_where(Expr::col(AideonIdxFieldF64::ValueF64).gt(*value));
                }
                CompareOp::Gte => {
                    select.and_where(Expr::col(AideonIdxFieldF64::ValueF64).gte(*value));
                }
                CompareOp::Prefix | CompareOp::Contains => {
                    return Err(MnemeError::invalid("string compare op on f64 field"));
                }
            }
            let rows = query_all(conn, &select).await?;
            for row in rows {
                let entity_id = read_id(&row, AideonIdxFieldF64::EntityId)?;
                ids.insert(entity_id);
            }
        }
        Value::Bool(value) => {
            let mut select = Query::select()
                .from(AideonIdxFieldBool::Table)
                .column(AideonIdxFieldBool::EntityId)
                .and_where(
                    Expr::col(AideonIdxFieldBool::PartitionId).eq(id_value(backend, partition.0)),
                )
                .and_where(
                    Expr::col(AideonIdxFieldBool::FieldId).eq(id_value(backend, filter.field_id)),
                )
                .and_where(Expr::col(AideonIdxFieldBool::ValidFrom).lte(at_valid_time.0))
                .and_where(
                    Expr::col(AideonIdxFieldBool::ValidTo)
                        .gt(at_valid_time.0)
                        .or(Expr::col(AideonIdxFieldBool::ValidTo).is_null()),
                )
                .to_owned();
            if let Some(as_of_asserted_at) = as_of_asserted_at {
                select.and_where(
                    Expr::col(AideonIdxFieldBool::AssertedAtHlc).lte(as_of_asserted_at.as_i64()),
                );
            }
            if let Some(scenario_id) = scenario_id {
                select.and_where(
                    Expr::col(AideonIdxFieldBool::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                select.and_where(Expr::col(AideonIdxFieldBool::ScenarioId).is_null());
            }
            match filter.op {
                CompareOp::Eq => {
                    select.and_where(Expr::col(AideonIdxFieldBool::ValueBool).eq(*value));
                }
                CompareOp::Ne => {
                    select.and_where(Expr::col(AideonIdxFieldBool::ValueBool).ne(*value));
                }
                _ => {
                    return Err(MnemeError::invalid("unsupported compare op for bool field"));
                }
            }
            let rows = query_all(conn, &select).await?;
            for row in rows {
                let entity_id = read_id(&row, AideonIdxFieldBool::EntityId)?;
                ids.insert(entity_id);
            }
        }
        Value::Time(value) => {
            let mut select = Query::select()
                .from(AideonIdxFieldTime::Table)
                .column(AideonIdxFieldTime::EntityId)
                .and_where(
                    Expr::col(AideonIdxFieldTime::PartitionId).eq(id_value(backend, partition.0)),
                )
                .and_where(
                    Expr::col(AideonIdxFieldTime::FieldId).eq(id_value(backend, filter.field_id)),
                )
                .and_where(Expr::col(AideonIdxFieldTime::ValidFrom).lte(at_valid_time.0))
                .and_where(
                    Expr::col(AideonIdxFieldTime::ValidTo)
                        .gt(at_valid_time.0)
                        .or(Expr::col(AideonIdxFieldTime::ValidTo).is_null()),
                )
                .to_owned();
            if let Some(as_of_asserted_at) = as_of_asserted_at {
                select.and_where(
                    Expr::col(AideonIdxFieldTime::AssertedAtHlc).lte(as_of_asserted_at.as_i64()),
                );
            }
            if let Some(scenario_id) = scenario_id {
                select.and_where(
                    Expr::col(AideonIdxFieldTime::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                select.and_where(Expr::col(AideonIdxFieldTime::ScenarioId).is_null());
            }
            match filter.op {
                CompareOp::Eq => {
                    select.and_where(Expr::col(AideonIdxFieldTime::ValueTime).eq(value.0));
                }
                CompareOp::Ne => {
                    select.and_where(Expr::col(AideonIdxFieldTime::ValueTime).ne(value.0));
                }
                CompareOp::Lt => {
                    select.and_where(Expr::col(AideonIdxFieldTime::ValueTime).lt(value.0));
                }
                CompareOp::Lte => {
                    select.and_where(Expr::col(AideonIdxFieldTime::ValueTime).lte(value.0));
                }
                CompareOp::Gt => {
                    select.and_where(Expr::col(AideonIdxFieldTime::ValueTime).gt(value.0));
                }
                CompareOp::Gte => {
                    select.and_where(Expr::col(AideonIdxFieldTime::ValueTime).gte(value.0));
                }
                CompareOp::Prefix | CompareOp::Contains => {
                    return Err(MnemeError::invalid("string compare op on time field"));
                }
            }
            let rows = query_all(conn, &select).await?;
            for row in rows {
                let entity_id = read_id(&row, AideonIdxFieldTime::EntityId)?;
                ids.insert(entity_id);
            }
        }
        Value::Ref(value) => {
            let mut select = Query::select()
                .from(AideonIdxFieldRef::Table)
                .column(AideonIdxFieldRef::EntityId)
                .and_where(
                    Expr::col(AideonIdxFieldRef::PartitionId).eq(id_value(backend, partition.0)),
                )
                .and_where(
                    Expr::col(AideonIdxFieldRef::FieldId).eq(id_value(backend, filter.field_id)),
                )
                .and_where(Expr::col(AideonIdxFieldRef::ValidFrom).lte(at_valid_time.0))
                .and_where(
                    Expr::col(AideonIdxFieldRef::ValidTo)
                        .gt(at_valid_time.0)
                        .or(Expr::col(AideonIdxFieldRef::ValidTo).is_null()),
                )
                .to_owned();
            if let Some(as_of_asserted_at) = as_of_asserted_at {
                select.and_where(
                    Expr::col(AideonIdxFieldRef::AssertedAtHlc).lte(as_of_asserted_at.as_i64()),
                );
            }
            if let Some(scenario_id) = scenario_id {
                select.and_where(
                    Expr::col(AideonIdxFieldRef::ScenarioId).eq(id_value(backend, scenario_id.0)),
                );
            } else {
                select.and_where(Expr::col(AideonIdxFieldRef::ScenarioId).is_null());
            }
            match filter.op {
                CompareOp::Eq => {
                    select.and_where(
                        Expr::col(AideonIdxFieldRef::ValueRefEntityId)
                            .eq(id_value(backend, *value)),
                    );
                }
                CompareOp::Ne => {
                    select.and_where(
                        Expr::col(AideonIdxFieldRef::ValueRefEntityId)
                            .ne(id_value(backend, *value)),
                    );
                }
                _ => {
                    return Err(MnemeError::invalid("unsupported compare op for ref field"));
                }
            }
            let rows = query_all(conn, &select).await?;
            for row in rows {
                let entity_id = read_id(&row, AideonIdxFieldRef::EntityId)?;
                ids.insert(entity_id);
            }
        }
        Value::Blob(_) | Value::Json(_) => {
            return Err(MnemeError::invalid("indexed filters not supported for blob/json"));
        }
    }
    Ok(ids)
}

fn filter_matches(resolved: Option<ReadValue>, filter: &FieldFilter) -> MnemeResult<bool> {
    let Some(resolved) = resolved else {
        return Ok(false);
    };
    match resolved {
        ReadValue::Single(value) => compare_value(&value, filter),
        ReadValue::Multi(values) => {
            if filter.op == CompareOp::Ne {
                for value in values {
                    if !compare_value(&value, filter)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            } else {
                for value in values {
                    if compare_value(&value, filter)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }
}

fn compare_value(value: &Value, filter: &FieldFilter) -> MnemeResult<bool> {
    match (value, &filter.value) {
        (Value::Str(actual), Value::Str(expected)) => {
            let actual_norm = normalize_index_text(actual);
            let expected_norm = normalize_index_text(expected);
            match filter.op {
                CompareOp::Eq => Ok(actual_norm == expected_norm),
                CompareOp::Ne => Ok(actual_norm != expected_norm),
                CompareOp::Lt => Ok(actual_norm < expected_norm),
                CompareOp::Lte => Ok(actual_norm <= expected_norm),
                CompareOp::Gt => Ok(actual_norm > expected_norm),
                CompareOp::Gte => Ok(actual_norm >= expected_norm),
                CompareOp::Prefix => Ok(actual_norm.starts_with(&expected_norm)),
                CompareOp::Contains => Ok(actual_norm.contains(&expected_norm)),
            }
        }
        (Value::I64(actual), Value::I64(expected)) => match filter.op {
            CompareOp::Eq => Ok(actual == expected),
            CompareOp::Ne => Ok(actual != expected),
            CompareOp::Lt => Ok(actual < expected),
            CompareOp::Lte => Ok(actual <= expected),
            CompareOp::Gt => Ok(actual > expected),
            CompareOp::Gte => Ok(actual >= expected),
            _ => Err(MnemeError::invalid("unsupported compare op for i64 field")),
        },
        (Value::F64(actual), Value::F64(expected)) => match filter.op {
            CompareOp::Eq => Ok(actual == expected),
            CompareOp::Ne => Ok(actual != expected),
            CompareOp::Lt => Ok(actual < expected),
            CompareOp::Lte => Ok(actual <= expected),
            CompareOp::Gt => Ok(actual > expected),
            CompareOp::Gte => Ok(actual >= expected),
            _ => Err(MnemeError::invalid("unsupported compare op for f64 field")),
        },
        (Value::Bool(actual), Value::Bool(expected)) => match filter.op {
            CompareOp::Eq => Ok(actual == expected),
            CompareOp::Ne => Ok(actual != expected),
            _ => Err(MnemeError::invalid("unsupported compare op for bool field")),
        },
        (Value::Time(actual), Value::Time(expected)) => match filter.op {
            CompareOp::Eq => Ok(actual == expected),
            CompareOp::Ne => Ok(actual != expected),
            CompareOp::Lt => Ok(actual < expected),
            CompareOp::Lte => Ok(actual <= expected),
            CompareOp::Gt => Ok(actual > expected),
            CompareOp::Gte => Ok(actual >= expected),
            _ => Err(MnemeError::invalid("unsupported compare op for time field")),
        },
        (Value::Ref(actual), Value::Ref(expected)) => match filter.op {
            CompareOp::Eq => Ok(actual == expected),
            CompareOp::Ne => Ok(actual != expected),
            _ => Err(MnemeError::invalid("unsupported compare op for ref field")),
        },
        _ => Err(MnemeError::invalid("filter value type mismatch")),
    }
}

fn read_value(value_type: ValueType, row: &QueryResult, column: &str) -> MnemeResult<Value> {
    Ok(match value_type {
        ValueType::Str => Value::Str(row.try_get("", column)?),
        ValueType::I64 => Value::I64(row.try_get("", column)?),
        ValueType::F64 => Value::F64(row.try_get("", column)?),
        ValueType::Bool => Value::Bool(row.try_get("", column)?),
        ValueType::Time => Value::Time(ValidTime(row.try_get("", column)?)),
        ValueType::Ref => {
            let id = read_id_by_name(row, column)?;
            Value::Ref(id)
        }
        ValueType::Blob => Value::Blob(row.try_get("", column)?),
        ValueType::Json => {
            let raw: String = row.try_get("", column)?;
            let json =
                serde_json::from_str(&raw).map_err(|err| MnemeError::storage(err.to_string()))?;
            Value::Json(json)
        }
    })
}

fn read_id_by_name(row: &QueryResult, column: &str) -> MnemeResult<Id> {
    if let Ok(value) = row.try_get::<String>("", column) {
        return Id::from_uuid_str(&value);
    }
    if let Ok(value) = row.try_get::<Uuid>("", column) {
        return Ok(Id::from_bytes(*value.as_bytes()));
    }
    if let Ok(value) = row.try_get::<Vec<u8>>("", column) {
        return bytes_to_id(value).ok_or_else(|| MnemeError::storage("invalid id length"));
    }
    Err(MnemeError::storage("unsupported id format"))
}

fn read_hlc_by_name(row: &QueryResult, column: &str) -> MnemeResult<Hlc> {
    let value: i64 = row.try_get("", column)?;
    Ok(Hlc::from_i64(value))
}

async fn default_value_for_field<C: ConnectionTrait>(
    conn: &C,
    partition: PartitionId,
    type_id: Option<Id>,
    field_id: Id,
) -> MnemeResult<Option<Value>> {
    let Some(type_id) = type_id else {
        return Ok(None);
    };
        let select = Query::select()
        .from(AideonTypeFields::Table)
        .inner_join(
            AideonFields::Table,
            Expr::col((AideonTypeFields::Table, AideonTypeFields::PartitionId))
                .equals((AideonFields::Table, AideonFields::PartitionId))
                .and(
                    Expr::col((AideonTypeFields::Table, AideonTypeFields::FieldId))
                        .equals((AideonFields::Table, AideonFields::FieldId)),
                ),
        )
        .column((AideonFields::Table, AideonFields::ValueType))
        .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueStr))
        .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueI64))
        .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueF64))
        .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueBool))
        .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueTime))
        .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueRef))
        .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueBlob))
        .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueJson))
        .and_where(
            Expr::col((AideonTypeFields::Table, AideonTypeFields::PartitionId))
                .eq(id_value(conn.get_database_backend(), partition.0)),
        )
        .and_where(
            Expr::col((AideonTypeFields::Table, AideonTypeFields::TypeId))
                .eq(id_value(conn.get_database_backend(), type_id)),
        )
        .and_where(
            Expr::col((AideonTypeFields::Table, AideonTypeFields::FieldId))
                .eq(id_value(conn.get_database_backend(), field_id)),
        )
        .to_owned();
    let row = query_one(conn, &select).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let value_type_raw: i16 = row.try_get("", &col_name(AideonFields::ValueType))?;
    let value_type = ValueType::from_i16(value_type_raw)
        .ok_or_else(|| MnemeError::storage("invalid value type"))?;
    let value = match value_type {
        ValueType::Str => row
            .try_get::<Option<String>>("", &col_name(AideonTypeFields::DefaultValueStr))?
            .map(Value::Str),
        ValueType::I64 => row
            .try_get::<Option<i64>>("", &col_name(AideonTypeFields::DefaultValueI64))?
            .map(Value::I64),
        ValueType::F64 => row
            .try_get::<Option<f64>>("", &col_name(AideonTypeFields::DefaultValueF64))?
            .map(Value::F64),
        ValueType::Bool => row
            .try_get::<Option<bool>>("", &col_name(AideonTypeFields::DefaultValueBool))?
            .map(Value::Bool),
        ValueType::Time => row
            .try_get::<Option<i64>>("", &col_name(AideonTypeFields::DefaultValueTime))?
            .map(|v| Value::Time(ValidTime(v))),
        ValueType::Ref => read_opt_id(&row, AideonTypeFields::DefaultValueRef)?.map(Value::Ref),
        ValueType::Blob => row
            .try_get::<Option<Vec<u8>>>("", &col_name(AideonTypeFields::DefaultValueBlob))?
            .map(Value::Blob),
        ValueType::Json => {
            let raw: Option<String> =
                row.try_get("", &col_name(AideonTypeFields::DefaultValueJson))?;
            raw.map(|raw| {
                serde_json::from_str(&raw)
                    .map(Value::Json)
                    .map_err(|err| MnemeError::storage(err.to_string()))
            })
            .transpose()?
        }
    };
    Ok(value)
}

impl MnemeStore {
    async fn build_effective_schema(
        &self,
        partition: PartitionId,
        type_id: Id,
    ) -> MnemeResult<EffectiveSchema> {
        let select_type = Query::select()
            .from(AideonTypes::Table)
            .column(AideonTypes::AppliesTo)
            .and_where(Expr::col(AideonTypes::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(Expr::col(AideonTypes::TypeId).eq(id_value(self.backend, type_id)))
            .and_where(Expr::col(AideonTypes::IsDeleted).eq(false))
            .to_owned();
        let row = query_one(&self.conn, &select_type).await?;
        let Some(row) = row else {
            return Err(MnemeError::not_found("type not found"));
        };
        let applies_to_raw: i16 = row.try_get("", &col_name(AideonTypes::AppliesTo))?;
        let applies_to = EntityKind::from_i16(applies_to_raw)
            .ok_or_else(|| MnemeError::storage("unknown applies_to"))?;

        let select_fields = Query::select()
            .from(AideonTypeFields::Table)
            .inner_join(
                AideonFields::Table,
                Expr::col((AideonTypeFields::Table, AideonTypeFields::PartitionId))
                    .equals((AideonFields::Table, AideonFields::PartitionId))
                    .and(
                        Expr::col((AideonTypeFields::Table, AideonTypeFields::FieldId))
                            .equals((AideonFields::Table, AideonFields::FieldId)),
                    ),
            )
            .column((AideonTypeFields::Table, AideonTypeFields::FieldId))
            .column((AideonFields::Table, AideonFields::ValueType))
            .column((AideonFields::Table, AideonFields::Cardinality))
            .column((AideonFields::Table, AideonFields::MergePolicy))
            .column((AideonFields::Table, AideonFields::IsIndexed))
            .column((AideonTypeFields::Table, AideonTypeFields::IsRequired))
            .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueStr))
            .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueI64))
            .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueF64))
            .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueBool))
            .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueTime))
            .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueRef))
            .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueBlob))
            .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueJson))
            .and_where(
                Expr::col((AideonTypeFields::Table, AideonTypeFields::PartitionId))
                    .eq(id_value(self.backend, partition.0)),
            )
            .and_where(
                Expr::col((AideonTypeFields::Table, AideonTypeFields::TypeId))
                    .eq(id_value(self.backend, type_id)),
            )
            .and_where(Expr::col((AideonFields::Table, AideonFields::IsDeleted)).eq(false))
            .to_owned();
        let rows = query_all(&self.conn, &select_fields).await?;
        let mut fields = Vec::new();
        for row in rows {
            let field_id = read_id(&row, AideonTypeFields::FieldId)?;
            let value_type_raw: i16 = row.try_get("", &col_name(AideonFields::ValueType))?;
            let merge_policy_raw: i16 = row.try_get("", &col_name(AideonFields::MergePolicy))?;
            let cardinality_raw: i16 = row.try_get("", &col_name(AideonFields::Cardinality))?;
            let value_type = ValueType::from_i16(value_type_raw)
                .ok_or_else(|| MnemeError::storage("invalid value type"))?;
            let merge_policy = MergePolicy::from_i16(merge_policy_raw)
                .ok_or_else(|| MnemeError::storage("invalid merge policy"))?;
            let cardinality_multi = cardinality_raw == 2;
            let is_indexed: bool = row.try_get("", &col_name(AideonFields::IsIndexed))?;
            let is_required: bool = row.try_get("", &col_name(AideonTypeFields::IsRequired))?;
            let default_value = match value_type {
                ValueType::Str => row
                    .try_get::<Option<String>>("", &col_name(AideonTypeFields::DefaultValueStr))?
                    .map(Value::Str),
                ValueType::I64 => row
                    .try_get::<Option<i64>>("", &col_name(AideonTypeFields::DefaultValueI64))?
                    .map(Value::I64),
                ValueType::F64 => row
                    .try_get::<Option<f64>>("", &col_name(AideonTypeFields::DefaultValueF64))?
                    .map(Value::F64),
                ValueType::Bool => row
                    .try_get::<Option<bool>>("", &col_name(AideonTypeFields::DefaultValueBool))?
                    .map(Value::Bool),
                ValueType::Time => row
                    .try_get::<Option<i64>>("", &col_name(AideonTypeFields::DefaultValueTime))?
                    .map(|v| Value::Time(ValidTime(v))),
                ValueType::Ref => read_opt_id(&row, AideonTypeFields::DefaultValueRef)?
                    .map(Value::Ref),
                ValueType::Blob => row
                    .try_get::<Option<Vec<u8>>>("", &col_name(AideonTypeFields::DefaultValueBlob))?
                    .map(Value::Blob),
                ValueType::Json => {
                    let raw: Option<String> =
                        row.try_get("", &col_name(AideonTypeFields::DefaultValueJson))?;
                    raw.map(|raw| {
                        serde_json::from_str(&raw)
                            .map(Value::Json)
                            .map_err(|err| MnemeError::storage(err.to_string()))
                    })
                    .transpose()?
                }
            };
            fields.push(EffectiveField {
                field_id,
                value_type,
                cardinality_multi,
                merge_policy,
                is_required,
                default_value,
                is_indexed,
            });
        }

        Ok(EffectiveSchema {
            type_id,
            applies_to,
            fields,
        })
    }
}

fn resolve_property(
    merge_policy: MergePolicy,
    mut facts: Vec<PropertyFact>,
) -> MnemeResult<Option<ReadValue>> {
    if facts.is_empty() {
        return Ok(None);
    }
    facts.sort_by(|a, b| {
        b.layer
            .cmp(&a.layer)
            .then_with(|| interval_width(a).cmp(&interval_width(b)))
            .then_with(|| b.asserted_at.cmp(&a.asserted_at))
            .then_with(|| a.op_id.as_bytes().cmp(&b.op_id.as_bytes()))
    });
    match merge_policy {
        MergePolicy::Lww | MergePolicy::Text => {
            let top = &facts[0];
            if top.is_tombstone {
                return Ok(None);
            }
            Ok(Some(ReadValue::Single(top.value.clone())))
        }
        MergePolicy::Mv => {
            let top = &facts[0];
            if top.is_tombstone {
                return Ok(None);
            }
            let top_layer = top.layer;
            let top_width = interval_width(top);
            let values = facts
                .into_iter()
                .filter(|fact| fact.layer == top_layer && interval_width(fact) == top_width)
                .filter(|fact| !fact.is_tombstone)
                .map(|fact| fact.value)
                .collect::<Vec<_>>();
            if values.is_empty() {
                return Ok(None);
            }
            Ok(Some(ReadValue::Multi(values)))
        }
        MergePolicy::OrSet => {
            let mut values = Vec::new();
            for fact in facts {
                if fact.is_tombstone {
                    values.retain(|value| value != &fact.value);
                } else {
                    values.push(fact.value);
                }
            }
            if values.is_empty() {
                return Ok(None);
            }
            Ok(Some(ReadValue::Multi(values)))
        }
        MergePolicy::Counter => {
            let mut total = 0;
            for fact in facts {
                if fact.is_tombstone {
                    continue;
                }
                if let Value::I64(value) = fact.value {
                    total += value;
                }
            }
            Ok(Some(ReadValue::Single(Value::I64(total))))
        }
    }
}

fn resolve_edge(facts: Vec<EdgeFact>) -> MnemeResult<Option<TraverseEdgeItem>> {
    if facts.is_empty() {
        return Ok(None);
    }
    let mut facts = facts;
    facts.sort_by(|a, b| {
        b.layer
            .cmp(&a.layer)
            .then_with(|| interval_width_edge(a).cmp(&interval_width_edge(b)))
            .then_with(|| b.asserted_at.cmp(&a.asserted_at))
            .then_with(|| a.op_id.as_bytes().cmp(&b.op_id.as_bytes()))
    });
    let top = &facts[0];
    if top.is_tombstone {
        return Ok(None);
    }
    Ok(Some(TraverseEdgeItem {
        edge_id: top.edge_id,
        src_id: top.src_id,
        dst_id: top.dst_id,
        type_id: top.edge_type_id,
    }))
}

fn interval_width(fact: &PropertyFact) -> i64 {
    fact.valid_to.unwrap_or(i64::MAX) - fact.valid_from
}

fn interval_width_edge(fact: &EdgeFact) -> i64 {
    fact.valid_to.unwrap_or(i64::MAX) - fact.valid_from
}

async fn edge_exists_at(
    conn: &DatabaseConnection,
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    edge_id: Id,
    at: ValidTime,
    as_of_asserted_at: Option<Hlc>,
    backend: DatabaseBackend,
) -> MnemeResult<bool> {
    let mut select = Query::select()
        .from(AideonEdgeExistsFacts::Table)
        .expr_as(
            Func::count(Expr::col(AideonEdgeExistsFacts::EdgeId)),
            Alias::new("edge_count"),
        )
        .and_where(Expr::col(AideonEdgeExistsFacts::PartitionId).eq(id_value(backend, partition.0)))
        .and_where(Expr::col(AideonEdgeExistsFacts::EdgeId).eq(id_value(backend, edge_id)))
        .and_where(Expr::col(AideonEdgeExistsFacts::ValidFrom).lte(at.0))
        .and_where(
            Expr::col(AideonEdgeExistsFacts::ValidTo)
                .gt(at.0)
                .or(Expr::col(AideonEdgeExistsFacts::ValidTo).is_null()),
        )
        .and_where(Expr::col(AideonEdgeExistsFacts::IsTombstone).eq(false))
        .to_owned();
    if let Some(scenario_id) = scenario_id {
        select.and_where(
            Expr::col(AideonEdgeExistsFacts::ScenarioId).eq(id_value(backend, scenario_id.0)),
        );
    } else {
        select.and_where(Expr::col(AideonEdgeExistsFacts::ScenarioId).is_null());
    }
    if let Some(as_of) = as_of_asserted_at {
        select.and_where(Expr::col(AideonEdgeExistsFacts::AssertedAtHlc).lte(as_of.as_i64()));
    }
    let row = query_one(conn, &select).await?;
    let Some(row) = row else {
        return Ok(false);
    };
    let count: i64 = row.try_get("", "edge_count")?;
    Ok(count > 0)
}

async fn fetch_op_deps(
    conn: &DatabaseConnection,
    partition: PartitionId,
    op_id: Id,
) -> MnemeResult<Vec<Id>> {
    let select = Query::select()
        .from(AideonOpDeps::Table)
        .column(AideonOpDeps::DepOpId)
        .and_where(
            Expr::col(AideonOpDeps::PartitionId).eq(id_value(conn.get_database_backend(), partition.0)),
        )
        .and_where(Expr::col(AideonOpDeps::OpId).eq(id_value(conn.get_database_backend(), op_id)))
        .to_owned();
    let rows = query_all(conn, &select).await?;
    rows.into_iter()
        .map(|row| read_id(&row, AideonOpDeps::DepOpId))
        .collect()
}
