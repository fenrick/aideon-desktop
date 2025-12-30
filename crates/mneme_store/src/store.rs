use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use sea_orm::sea_query;
use sea_orm::sea_query::{
    Alias, Expr, ExprTrait, Func, MysqlQueryBuilder, OnConflict, Order, PostgresQueryBuilder,
    Query, QueryStatementWriter, SqliteQueryBuilder, Value as SeaValue,
};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, QueryResult,
    Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::sleep;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::db::*;
use crate::migration::Migrator;
use crate::{MnemeConfig, ValidationMode, load_schema_manifest};
use aideon_mneme_core::api::{
    AnalyticsApi, AnalyticsResultsApi, ChangeEvent, ChangeFeedApi, ComputedCacheApi,
    ComputedCacheEntry, ComputedRule, ComputedRulesApi, CreateScenarioInput, DiagnosticsApi,
    Direction, ExportOpsInput, ExportOptions, ExportRecord, GraphReadApi, GraphWriteApi,
    ImportOptions, ImportReport, JobRecord, JobSummary, ListComputedCacheInput, MetamodelApi,
    MnemeExportApi, MnemeImportApi, MnemeProcessingApi, MnemeSnapshotApi, ProjectionEdge,
    PropertyWriteApi, RunWorkerInput, ScenarioApi, SnapshotOptions, SyncApi,
    TriggerCompactionInput, TriggerProcessingInput, TriggerRetentionInput, ValidationRule,
    ValidationRulesApi,
};
use aideon_mneme_core::ops::{
    ClearPropIntervalInput, CounterUpdateInput, CreateEdgeInput, CreateNodeInput, OpPayload,
    OrSetUpdateInput, SetOp, SetPropIntervalInput,
};
use aideon_mneme_core::schema::{EffectiveField, EffectiveSchema, MetamodelBatch, SchemaVersion};
use aideon_mneme_core::value::{EntityKind, Layer, MergePolicy, Value, ValueType};
use aideon_mneme_core::{
    ActorId, CompareOp, FieldFilter, Hlc, Id, ListEntitiesInput, ListEntitiesResultItem,
    MnemeError, MnemeResult, OpEnvelope, OpId, PartitionId, ReadEntityAtTimeInput,
    ReadEntityAtTimeResult, ReadValue, RetentionPolicy, ScenarioId, SecurityContext,
    TraverseAtTimeInput, TraverseEdgeItem, ValidTime,
};
use sea_orm_migration::MigratorTrait;
use sea_orm_migration::prelude::Iden;

#[derive(Clone)]
pub struct MnemeStore {
    conn: DatabaseConnection,
    backend: DatabaseBackend,
    limits: MnemeLimits,
    record_overlap_warnings: bool,
    validation_mode: ValidationMode,
    failpoints: HashSet<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct BackendCapabilities {
    pub transactional_ddl: bool,
    pub partial_indexes: bool,
    pub computed_columns: bool,
    pub json_types: bool,
}

#[derive(Clone, Copy, Debug)]
struct MnemeLimits {
    max_op_payload_bytes: usize,
    max_blob_bytes: usize,
    max_mv_values: usize,
    max_pending_jobs: u32,
    max_ingest_batch: usize,
}

impl MnemeLimits {
    fn from_config(config: &MnemeConfig) -> Self {
        let defaults = crate::LimitsConfig::with_defaults();
        let limits = config.limits.clone().unwrap_or(defaults);
        Self {
            max_op_payload_bytes: limits.max_op_payload_bytes.unwrap_or(1_048_576),
            max_blob_bytes: limits.max_blob_bytes.unwrap_or(4_194_304),
            max_mv_values: limits.max_mv_values.unwrap_or(100),
            max_pending_jobs: limits.max_pending_jobs.unwrap_or(10_000),
            max_ingest_batch: limits.max_ingest_batch.unwrap_or(5_000),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FieldConstraints {
    value_type: ValueType,
    merge_policy: MergePolicy,
    cardinality_multi: bool,
    is_indexed: bool,
    disallow_overlap: bool,
}

#[derive(Clone, Copy, Debug)]
struct TypeMeta {
    applies_to: EntityKind,
    is_abstract: bool,
}

#[derive(Clone, Debug)]
struct EdgeRow {
    scenario_id: Option<ScenarioId>,
    src_id: Id,
    dst_id: Id,
    edge_type_id: Option<Id>,
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
        let limits = MnemeLimits::from_config(config);
        let record_overlap_warnings = config
            .integrity
            .as_ref()
            .and_then(|cfg| cfg.record_overlap_warnings)
            .unwrap_or(true);
        let validation_mode = config.validation_mode.unwrap_or(ValidationMode::Error);
        let failpoints = config
            .failpoints
            .clone()
            .unwrap_or_default()
            .into_iter()
            .collect::<HashSet<_>>();
        let store = Self {
            conn,
            backend,
            limits,
            record_overlap_warnings,
            validation_mode,
            failpoints,
        };
        Migrator::up(&store.conn, None)
            .await
            .map_err(MnemeError::from)?;
        Ok(store)
    }

    async fn op_exists<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        op_id: OpId,
    ) -> MnemeResult<bool> {
        let select = Query::select()
            .from(AideonOps::Table)
            .column(AideonOps::OpId)
            .and_where(Expr::col(AideonOps::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(Expr::col(AideonOps::OpId).eq(id_value(self.backend, op_id.0)))
            .limit(1)
            .to_owned();
        Ok(query_one(conn, &select).await?.is_some())
    }

    async fn fetch_type_meta<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        type_id: Id,
    ) -> MnemeResult<Option<TypeMeta>> {
        let select = Query::select()
            .from(AideonTypes::Table)
            .columns([AideonTypes::AppliesTo, AideonTypes::IsAbstract])
            .and_where(Expr::col(AideonTypes::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(Expr::col(AideonTypes::TypeId).eq(id_value(self.backend, type_id)))
            .and_where(Expr::col(AideonTypes::IsDeleted).eq(false))
            .limit(1)
            .to_owned();
        let row = query_one(conn, &select).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let applies_to_raw: i16 = row.try_get("", &col_name(AideonTypes::AppliesTo))?;
        let applies_to = EntityKind::from_i16(applies_to_raw)
            .ok_or_else(|| MnemeError::storage("invalid applies_to"))?;
        let is_abstract: bool = row.try_get("", &col_name(AideonTypes::IsAbstract))?;
        Ok(Some(TypeMeta {
            applies_to,
            is_abstract,
        }))
    }

    async fn type_lineage<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        type_id: Id,
    ) -> MnemeResult<Vec<Id>> {
        let mut lineage = Vec::new();
        let mut seen = HashSet::new();
        let mut current = type_id;
        loop {
            if self
                .fetch_type_meta(conn, partition, current)
                .await?
                .is_none()
            {
                return Err(MnemeError::validation("type not found"));
            }
            if !seen.insert(current) {
                return Err(MnemeError::validation("type inheritance cycle detected"));
            }
            lineage.push(current);
            let select = Query::select()
                .from(AideonTypeExtends::Table)
                .column(AideonTypeExtends::ParentTypeId)
                .and_where(
                    Expr::col(AideonTypeExtends::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonTypeExtends::TypeId).eq(id_value(self.backend, current)))
                .limit(1)
                .to_owned();
            let row = query_one(conn, &select).await?;
            let Some(row) = row else {
                break;
            };
            let parent_id = read_id(&row, AideonTypeExtends::ParentTypeId)?;
            if self
                .fetch_type_meta(conn, partition, parent_id)
                .await?
                .is_none()
            {
                return Err(MnemeError::validation("parent type not found"));
            }
            current = parent_id;
        }
        lineage.reverse();
        Ok(lineage)
    }

    async fn read_entity_row_with_fallback_conn<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        scenario_id: Option<ScenarioId>,
        entity_id: Id,
    ) -> MnemeResult<Option<(EntityKind, Option<Id>, bool)>> {
        if let Some(scenario_id) = scenario_id
            && let Some(row) = self
                .read_entity_row_in_partition_conn(conn, partition, Some(scenario_id), entity_id)
                .await?
        {
            return Ok(Some(row));
        }
        self.read_entity_row_in_partition_conn(conn, partition, None, entity_id)
            .await
    }

    async fn fetch_edge_row<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        edge_id: Id,
    ) -> MnemeResult<Option<EdgeRow>> {
        let select = Query::select()
            .from(AideonEdges::Table)
            .columns([
                AideonEdges::ScenarioId,
                AideonEdges::SrcEntityId,
                AideonEdges::DstEntityId,
                AideonEdges::EdgeTypeId,
            ])
            .and_where(Expr::col(AideonEdges::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(Expr::col(AideonEdges::EdgeId).eq(id_value(self.backend, edge_id)))
            .limit(1)
            .to_owned();
        let row = query_one(conn, &select).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let scenario_id = read_opt_id(&row, AideonEdges::ScenarioId)?.map(ScenarioId);
        let src_id = read_id(&row, AideonEdges::SrcEntityId)?;
        let dst_id = read_id(&row, AideonEdges::DstEntityId)?;
        let edge_type_id = read_opt_id(&row, AideonEdges::EdgeTypeId)?;
        Ok(Some(EdgeRow {
            scenario_id,
            src_id,
            dst_id,
            edge_type_id,
        }))
    }

    async fn fetch_edge_type_rule<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        edge_type_id: Id,
    ) -> MnemeResult<Option<crate::schema::EdgeTypeRule>> {
        let select = Query::select()
            .from(AideonEdgeTypeRules::Table)
            .columns([
                AideonEdgeTypeRules::AllowedSrcTypeIdsJson,
                AideonEdgeTypeRules::AllowedDstTypeIdsJson,
                AideonEdgeTypeRules::SemanticDirection,
            ])
            .and_where(
                Expr::col(AideonEdgeTypeRules::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .and_where(
                Expr::col(AideonEdgeTypeRules::EdgeTypeId).eq(id_value(self.backend, edge_type_id)),
            )
            .limit(1)
            .to_owned();
        let row = query_one(conn, &select).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let src_json: String =
            row.try_get("", &col_name(AideonEdgeTypeRules::AllowedSrcTypeIdsJson))?;
        let dst_json: String =
            row.try_get("", &col_name(AideonEdgeTypeRules::AllowedDstTypeIdsJson))?;
        let semantic_direction: Option<String> =
            row.try_get("", &col_name(AideonEdgeTypeRules::SemanticDirection))?;
        let allowed_src_type_ids: Vec<Id> =
            serde_json::from_str(&src_json).map_err(|err| MnemeError::storage(err.to_string()))?;
        let allowed_dst_type_ids: Vec<Id> =
            serde_json::from_str(&dst_json).map_err(|err| MnemeError::storage(err.to_string()))?;
        Ok(Some(crate::schema::EdgeTypeRule {
            edge_type_id,
            allowed_src_type_ids,
            allowed_dst_type_ids,
            semantic_direction,
        }))
    }

    async fn validate_create_edge<C: ConnectionTrait>(
        &self,
        conn: &C,
        input: &CreateEdgeInput,
    ) -> MnemeResult<()> {
        if let Some(type_id) = input.type_id {
            let meta = self.fetch_type_meta(conn, input.partition, type_id).await?;
            if let Some(meta) = meta {
                self.validate_or_warn(
                    !meta.is_abstract,
                    "abstract edge types cannot be instantiated",
                )?;
                self.validate_or_warn(
                    meta.applies_to == EntityKind::Edge,
                    "type does not apply to edges",
                )?;
            } else {
                self.validate_or_warn(false, "edge type not found")?;
            }
        }

        let src = self
            .read_entity_row_with_fallback_conn(
                conn,
                input.partition,
                input.scenario_id,
                input.src_id,
            )
            .await?
            .ok_or_else(|| MnemeError::validation("edge src entity not found"))?;
        let dst = self
            .read_entity_row_with_fallback_conn(
                conn,
                input.partition,
                input.scenario_id,
                input.dst_id,
            )
            .await?
            .ok_or_else(|| MnemeError::validation("edge dst entity not found"))?;
        if src.2 || dst.2 {
            return Err(MnemeError::validation("edge endpoints are deleted"));
        }
        self.validate_or_warn(src.0 == EntityKind::Node, "edge src is not a node")?;
        self.validate_or_warn(dst.0 == EntityKind::Node, "edge dst is not a node")?;

        if let Some(edge_type_id) = input.type_id
            && let Some(rule) = self
                .fetch_edge_type_rule(conn, input.partition, edge_type_id)
                .await?
        {
            if !rule.allowed_src_type_ids.is_empty() {
                if let Some(src_type) = src.1 {
                    self.validate_or_warn(
                        rule.allowed_src_type_ids.contains(&src_type),
                        "edge src type not allowed by rule",
                    )?;
                } else {
                    self.validate_or_warn(false, "edge src type is required by rule")?;
                }
            }
            if !rule.allowed_dst_type_ids.is_empty() {
                if let Some(dst_type) = dst.1 {
                    self.validate_or_warn(
                        rule.allowed_dst_type_ids.contains(&dst_type),
                        "edge dst type not allowed by rule",
                    )?;
                } else {
                    self.validate_or_warn(false, "edge dst type is required by rule")?;
                }
            }
        }

        if let Some(existing) = self
            .fetch_edge_row(conn, input.partition, input.edge_id)
            .await?
        {
            self.validate_or_warn(
                existing.scenario_id == input.scenario_id,
                "edge scenario mismatch",
            )?;
            self.validate_or_warn(
                existing.src_id == input.src_id && existing.dst_id == input.dst_id,
                "edge endpoints are immutable",
            )?;
            if existing.edge_type_id != input.type_id {
                self.validate_or_warn(false, "edge type is immutable")?;
            }
        }
        Ok(())
    }

    async fn validate_create_node<C: ConnectionTrait>(
        &self,
        conn: &C,
        input: &CreateNodeInput,
    ) -> MnemeResult<()> {
        if let Some(type_id) = input.type_id {
            let meta = self.fetch_type_meta(conn, input.partition, type_id).await?;
            if let Some(meta) = meta {
                self.validate_or_warn(!meta.is_abstract, "abstract types cannot be instantiated")?;
                self.validate_or_warn(
                    meta.applies_to == EntityKind::Node,
                    "type does not apply to nodes",
                )?;
            } else {
                self.validate_or_warn(false, "type not found")?;
            }
        }
        Ok(())
    }

    async fn validate_edge_exists<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        scenario_id: Option<ScenarioId>,
        edge_id: Id,
    ) -> MnemeResult<()> {
        let edge = self
            .read_entity_row_with_fallback_conn(conn, partition, scenario_id, edge_id)
            .await?
            .ok_or_else(|| MnemeError::validation("edge entity not found"))?;
        if edge.2 {
            return Err(MnemeError::validation("edge entity is deleted"));
        }
        self.validate_or_warn(edge.0 == EntityKind::Edge, "entity is not an edge")?;
        Ok(())
    }

    pub async fn connect_sqlite(path: &Path) -> MnemeResult<Self> {
        let config = MnemeConfig::default_sqlite(path.to_string_lossy());
        Self::connect(&config, path.parent().unwrap_or_else(|| Path::new("."))).await
    }

    fn validation_failure(&self, message: impl Into<String>) -> MnemeResult<()> {
        let message = message.into();
        match self.validation_mode {
            ValidationMode::Error => Err(MnemeError::validation(message)),
            ValidationMode::Warn => {
                log::warn!("mneme validation warning: {message}");
                Ok(())
            }
            ValidationMode::Off => Ok(()),
        }
    }

    fn validate_or_warn(&self, condition: bool, message: impl Into<String>) -> MnemeResult<()> {
        if condition {
            Ok(())
        } else {
            self.validation_failure(message)
        }
    }

    fn maybe_failpoint(&self, key: &str) -> MnemeResult<()> {
        if self.failpoints.contains(key) {
            Err(MnemeError::storage(format!("failpoint {key}")))
        } else {
            Ok(())
        }
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
            .and_where(
                Expr::col(AideonHlcState::PartitionId).eq(id_value(self.backend, partition.0)),
            )
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
        let payload_scenario = payload_scenario_id(payload);
        if payload_bytes.len() > self.limits.max_op_payload_bytes {
            return Err(MnemeError::invalid("op payload exceeds limit"));
        }
        self.bump_hlc_state(tx, partition, asserted_at).await?;
        let insert = Query::insert()
            .into_table(AideonOps::Table)
            .columns([
                AideonOps::PartitionId,
                AideonOps::ScenarioId,
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
                opt_id_value(self.backend, payload_scenario.map(|s| s.0)).into(),
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
        self.maybe_failpoint("after_op_insert")?;
        Ok((op_id, asserted_at, payload_bytes, op_type))
    }

    async fn apply_op_payload(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        op_id: OpId,
        asserted_at: Hlc,
        payload: &OpPayload,
    ) -> MnemeResult<()> {
        match payload {
            OpPayload::CreateNode(input) => {
                self.validate_create_node(tx, input).await?;
                let insert_entity = Query::insert()
                    .into_table(AideonEntities::Table)
                    .columns([
                        AideonEntities::PartitionId,
                        AideonEntities::ScenarioId,
                        AideonEntities::EntityId,
                        AideonEntities::EntityKind,
                        AideonEntities::TypeId,
                        AideonEntities::IsDeleted,
                        AideonEntities::AclGroupId,
                        AideonEntities::OwnerActorId,
                        AideonEntities::Visibility,
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
                        input.acl_group_id.clone().into(),
                        opt_id_value(self.backend, input.owner_actor_id.map(|actor| actor.0))
                            .into(),
                        input.visibility.map(|v| v as i64).into(),
                        id_value(self.backend, op_id.0).into(),
                        id_value(self.backend, op_id.0).into(),
                        asserted_at.as_i64().into(),
                        asserted_at.as_i64().into(),
                    ])
                    .on_conflict(
                        OnConflict::columns([
                            AideonEntities::PartitionId,
                            AideonEntities::EntityId,
                        ])
                        .do_nothing()
                        .to_owned(),
                    )
                    .to_owned();
                exec(tx, &insert_entity).await?;
            }
            OpPayload::CreateEdge(input) => {
                self.validate_create_edge(tx, input).await?;
                let insert_entity = Query::insert()
                    .into_table(AideonEntities::Table)
                    .columns([
                        AideonEntities::PartitionId,
                        AideonEntities::ScenarioId,
                        AideonEntities::EntityId,
                        AideonEntities::EntityKind,
                        AideonEntities::TypeId,
                        AideonEntities::IsDeleted,
                        AideonEntities::AclGroupId,
                        AideonEntities::OwnerActorId,
                        AideonEntities::Visibility,
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
                        input.acl_group_id.clone().into(),
                        opt_id_value(self.backend, input.owner_actor_id.map(|actor| actor.0))
                            .into(),
                        input.visibility.map(|v| v as i64).into(),
                        id_value(self.backend, op_id.0).into(),
                        id_value(self.backend, op_id.0).into(),
                        asserted_at.as_i64().into(),
                        asserted_at.as_i64().into(),
                    ])
                    .on_conflict(
                        OnConflict::columns([
                            AideonEntities::PartitionId,
                            AideonEntities::EntityId,
                        ])
                        .do_nothing()
                        .to_owned(),
                    )
                    .to_owned();
                exec(tx, &insert_entity).await?;

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
                            .do_nothing()
                            .to_owned(),
                    )
                    .to_owned();
                exec(tx, &insert_edge).await?;

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
                    .on_conflict(
                        OnConflict::columns([
                            AideonEdgeExistsFacts::PartitionId,
                            AideonEdgeExistsFacts::EdgeId,
                            AideonEdgeExistsFacts::ValidFrom,
                            AideonEdgeExistsFacts::AssertedAtHlc,
                            AideonEdgeExistsFacts::OpId,
                        ])
                        .do_nothing()
                        .to_owned(),
                    )
                    .to_owned();
                exec(tx, &insert_exists).await?;

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
                        .do_nothing()
                        .to_owned(),
                    )
                    .to_owned();
                exec(tx, &insert_projection).await?;
            }
            OpPayload::SetEdgeExistenceInterval(input) => {
                self.validate_edge_exists(tx, input.partition, input.scenario_id, input.edge_id)
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
                    .on_conflict(
                        OnConflict::columns([
                            AideonEdgeExistsFacts::PartitionId,
                            AideonEdgeExistsFacts::EdgeId,
                            AideonEdgeExistsFacts::ValidFrom,
                            AideonEdgeExistsFacts::AssertedAtHlc,
                            AideonEdgeExistsFacts::OpId,
                        ])
                        .do_nothing()
                        .to_owned(),
                    )
                    .to_owned();
                exec(tx, &insert_exists).await?;
            }
            OpPayload::TombstoneEntity {
                partition: payload_partition,
                scenario_id,
                entity_id,
                ..
            } => {
                let update = Query::update()
                    .table(AideonEntities::Table)
                    .values([
                        (AideonEntities::IsDeleted, true.into()),
                        (
                            AideonEntities::UpdatedOpId,
                            id_value(self.backend, op_id.0).into(),
                        ),
                        (
                            AideonEntities::UpdatedAssertedAtHlc,
                            asserted_at.as_i64().into(),
                        ),
                    ])
                    .and_where(
                        Expr::col(AideonEntities::PartitionId)
                            .eq(id_value(self.backend, payload_partition.0)),
                    )
                    .and_where(
                        Expr::col(AideonEntities::EntityId).eq(id_value(self.backend, *entity_id)),
                    )
                    .to_owned();
                exec(tx, &update).await?;

                if let Some((kind, _type_id, _is_deleted)) = self
                    .read_entity_row_in_partition_conn(tx, *payload_partition, None, *entity_id)
                    .await?
                    && kind == EntityKind::Edge
                {
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
                            id_value(self.backend, payload_partition.0).into(),
                            opt_id_value(self.backend, scenario_id.map(|s| s.0)).into(),
                            id_value(self.backend, *entity_id).into(),
                            0i64.into(),
                            SeaValue::BigInt(None).into(),
                            valid_bucket(ValidTime(0)).into(),
                            Self::layer_value(Layer::Actual).into(),
                            asserted_at.as_i64().into(),
                            id_value(self.backend, op_id.0).into(),
                            true.into(),
                        ])
                        .on_conflict(
                            OnConflict::columns([
                                AideonEdgeExistsFacts::PartitionId,
                                AideonEdgeExistsFacts::EdgeId,
                                AideonEdgeExistsFacts::ValidFrom,
                                AideonEdgeExistsFacts::AssertedAtHlc,
                                AideonEdgeExistsFacts::OpId,
                            ])
                            .do_nothing()
                            .to_owned(),
                        )
                        .to_owned();
                    exec(tx, &insert_exists).await?;
                }

                let delete_projection = Query::delete()
                    .from_table(AideonGraphProjectionEdges::Table)
                    .and_where(
                        Expr::col(AideonGraphProjectionEdges::PartitionId)
                            .eq(id_value(self.backend, payload_partition.0)),
                    )
                    .and_where(
                        Expr::col(AideonGraphProjectionEdges::EdgeId)
                            .eq(id_value(self.backend, *entity_id)),
                    )
                    .to_owned();
                exec(tx, &delete_projection).await?;
            }
            OpPayload::SetProperty(input) => {
                let constraints = self
                    .fetch_field_def_with_entity(
                        tx,
                        input.partition,
                        input.scenario_id,
                        input.field_id,
                        input.entity_id,
                    )
                    .await?;
                if constraints.value_type != input.value.value_type() {
                    return Err(MnemeError::invalid("value type mismatch"));
                }
                if matches!(
                    constraints.merge_policy,
                    MergePolicy::OrSet | MergePolicy::Counter
                ) {
                    return Err(MnemeError::validation(
                        "use or_set_update/counter_update for this field",
                    ));
                }
                insert_property_fact(
                    tx,
                    self.backend,
                    PropertyFactInsertInput {
                        partition: input.partition,
                        scenario_id: input.scenario_id,
                        entity_id: input.entity_id,
                        field_id: input.field_id,
                        value: &input.value,
                        valid_from: input.valid_from,
                        valid_to: input.valid_to,
                        layer: input.layer,
                        asserted_at,
                        op_id,
                        is_tombstone: false,
                    },
                )
                .await?;
                if constraints.is_indexed {
                    insert_index_row(
                        tx,
                        self.backend,
                        IndexInsertInput {
                            partition: input.partition,
                            scenario_id: input.scenario_id,
                            field_id: input.field_id,
                            entity_id: input.entity_id,
                            value: &input.value,
                            valid_from: input.valid_from,
                            valid_to: input.valid_to,
                            asserted_at,
                            layer: input.layer,
                        },
                    )
                    .await?;
                }
            }
            OpPayload::ClearProperty(input) => {
                let constraints = self
                    .fetch_field_def_with_entity(
                        tx,
                        input.partition,
                        input.scenario_id,
                        input.field_id,
                        input.entity_id,
                    )
                    .await?;
                if matches!(
                    constraints.merge_policy,
                    MergePolicy::OrSet | MergePolicy::Counter
                ) {
                    return Err(MnemeError::validation(
                        "use or_set_update/counter_update for this field",
                    ));
                }
                let value = default_value_for_type(constraints.value_type);
                insert_property_fact(
                    tx,
                    self.backend,
                    PropertyFactInsertInput {
                        partition: input.partition,
                        scenario_id: input.scenario_id,
                        entity_id: input.entity_id,
                        field_id: input.field_id,
                        value: &value,
                        valid_from: input.valid_from,
                        valid_to: input.valid_to,
                        layer: input.layer,
                        asserted_at,
                        op_id,
                        is_tombstone: true,
                    },
                )
                .await?;
                if constraints.is_indexed {
                    delete_index_row(
                        tx,
                        input.partition,
                        input.scenario_id,
                        input.field_id,
                        input.entity_id,
                        &value,
                        self.backend,
                    )
                    .await?;
                }
            }
            OpPayload::OrSetUpdate(input) => {
                let constraints = self
                    .fetch_field_def_with_entity(
                        tx,
                        input.partition,
                        input.scenario_id,
                        input.field_id,
                        input.entity_id,
                    )
                    .await?;
                if constraints.value_type != input.element.value_type() {
                    return Err(MnemeError::invalid("value type mismatch"));
                }
                if constraints.merge_policy != MergePolicy::OrSet {
                    return Err(MnemeError::invalid(
                        "field is not configured for OR_SET merge policy",
                    ));
                }
                let is_tombstone = matches!(input.op, SetOp::Remove);
                insert_property_fact(
                    tx,
                    self.backend,
                    PropertyFactInsertInput {
                        partition: input.partition,
                        scenario_id: input.scenario_id,
                        entity_id: input.entity_id,
                        field_id: input.field_id,
                        value: &input.element,
                        valid_from: input.valid_from,
                        valid_to: input.valid_to,
                        layer: input.layer,
                        asserted_at,
                        op_id,
                        is_tombstone,
                    },
                )
                .await?;
                if constraints.is_indexed {
                    if is_tombstone {
                        delete_index_row(
                            tx,
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
                            tx,
                            self.backend,
                            IndexInsertInput {
                                partition: input.partition,
                                scenario_id: input.scenario_id,
                                field_id: input.field_id,
                                entity_id: input.entity_id,
                                value: &input.element,
                                valid_from: input.valid_from,
                                valid_to: input.valid_to,
                                asserted_at,
                                layer: input.layer,
                            },
                        )
                        .await?;
                    }
                }
            }
            OpPayload::CounterUpdate(input) => {
                let constraints = self
                    .fetch_field_def_with_entity(
                        tx,
                        input.partition,
                        input.scenario_id,
                        input.field_id,
                        input.entity_id,
                    )
                    .await?;
                if constraints.merge_policy != MergePolicy::Counter {
                    return Err(MnemeError::invalid(
                        "field is not configured for COUNTER merge policy",
                    ));
                }
                let value = Value::I64(input.delta);
                insert_property_fact(
                    tx,
                    self.backend,
                    PropertyFactInsertInput {
                        partition: input.partition,
                        scenario_id: input.scenario_id,
                        entity_id: input.entity_id,
                        field_id: input.field_id,
                        value: &value,
                        valid_from: input.valid_from,
                        valid_to: input.valid_to,
                        layer: input.layer,
                        asserted_at,
                        op_id,
                        is_tombstone: false,
                    },
                )
                .await?;
                if constraints.is_indexed {
                    insert_index_row(
                        tx,
                        self.backend,
                        IndexInsertInput {
                            partition: input.partition,
                            scenario_id: input.scenario_id,
                            field_id: input.field_id,
                            entity_id: input.entity_id,
                            value: &value,
                            valid_from: input.valid_from,
                            valid_to: input.valid_to,
                            asserted_at,
                            layer: input.layer,
                        },
                    )
                    .await?;
                }
            }
            OpPayload::UpsertMetamodelBatch(batch) => {
                self.apply_metamodel_batch(tx, partition, op_id, asserted_at, batch)
                    .await?;
            }
            OpPayload::CreateScenario { .. } | OpPayload::DeleteScenario { .. } => {}
        }
        Ok(())
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
            .expr_as(
                Func::max(Expr::col(AideonChangeFeed::Sequence)),
                Alias::new("max_seq"),
            )
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
        if payload_bulk_mode(payload) {
            return Ok(());
        }
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
            OpPayload::SetEdgeExistenceInterval(input) => {
                let job_payload = TriggerJobPayload {
                    partition_id: input.partition,
                    scenario_id: input.scenario_id,
                    type_id: None,
                    schema_version_hint: None,
                    reason: "edge_interval_change".to_string(),
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
            OpPayload::TombstoneEntity { scenario_id, .. } => {
                let job_payload = TriggerJobPayload {
                    partition_id: partition,
                    scenario_id: *scenario_id,
                    type_id: None,
                    schema_version_hint: None,
                    reason: "edge_tombstone".to_string(),
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
                    tx,
                    partition,
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

    async fn export_fact_table<
        TTable: sea_query::Iden + Clone,
        TEntity: sea_query::Iden + Clone,
        TField: sea_query::Iden + Clone,
        TValidFrom: sea_query::Iden + Clone,
        TValidTo: sea_query::Iden + Clone,
        TLayer: sea_query::Iden + Clone,
        TAsserted: sea_query::Iden + Clone,
        TOpId: sea_query::Iden + Clone,
        TTombstone: sea_query::Iden + Clone,
        TValue: sea_query::Iden + Clone,
    >(
        &self,
        records: &mut Vec<ExportRecord>,
        opts: &SnapshotOptions,
        spec: ExportFactTableSpec<
            '_,
            TTable,
            TEntity,
            TField,
            TValidFrom,
            TValidTo,
            TLayer,
            TAsserted,
            TOpId,
            TTombstone,
            TValue,
        >,
    ) -> MnemeResult<()> {
        let table = spec.table;
        let entity_col = spec.entity_col;
        let field_col = spec.field_col;
        let valid_from_col = spec.valid_from_col;
        let valid_to_col = spec.valid_to_col;
        let layer_col = spec.layer_col;
        let asserted_col = spec.asserted_col;
        let op_id_col = spec.op_id_col;
        let tombstone_col = spec.tombstone_col;
        let value_col = spec.value_col;
        let mut select = Query::select()
            .from(table.clone())
            .column(entity_col.clone())
            .column(field_col.clone())
            .column(valid_from_col.clone())
            .column(valid_to_col.clone())
            .column(layer_col.clone())
            .column(asserted_col.clone())
            .column(op_id_col.clone())
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
            let op_id = read_id_by_name(&row, &col_name(op_id_col.clone()))?;
            let is_tombstone: bool = row.try_get("", &col_name(tombstone_col.clone()))?;
            let value = match spec.record_type {
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
                record_type: spec.record_type.to_string(),
                data: serde_json::json!({
                    "entity_id": entity_id,
                    "field_id": field_id,
                    "valid_from": valid_from,
                    "valid_to": valid_to,
                    "layer": layer,
                    "asserted_at": asserted_at.as_i64(),
                    "op_id": op_id,
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
        let op_id = Id::from_uuid_str(
            record
                .data
                .get("op_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MnemeError::invalid("missing op_id"))?,
        )?;
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
                        id_value(self.backend, op_id).into(),
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
                        id_value(self.backend, op_id).into(),
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
                        id_value(self.backend, op_id).into(),
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
                        id_value(self.backend, op_id).into(),
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
                        id_value(self.backend, op_id).into(),
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
                        id_value(self.backend, op_id).into(),
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
                        id_value(self.backend, op_id).into(),
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
                        id_value(self.backend, op_id).into(),
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
        self.read_entity_row_in_partition_conn(&self.conn, partition, scenario_id, entity_id)
            .await
    }

    async fn read_entity_security_in_partition(
        &self,
        partition: PartitionId,
        scenario_id: Option<crate::ScenarioId>,
        entity_id: Id,
    ) -> MnemeResult<Option<EntitySecurity>> {
        let mut select = Query::select()
            .from(AideonEntities::Table)
            .columns([
                AideonEntities::AclGroupId,
                AideonEntities::OwnerActorId,
                AideonEntities::Visibility,
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
        let acl_group_id: Option<String> =
            row.try_get("", &col_name(AideonEntities::AclGroupId))?;
        let owner_actor_id = read_opt_id(&row, AideonEntities::OwnerActorId)?.map(ActorId);
        let visibility: Option<i64> = row.try_get("", &col_name(AideonEntities::Visibility))?;
        let is_deleted: bool = row.try_get("", &col_name(AideonEntities::IsDeleted))?;
        Ok(Some(EntitySecurity {
            acl_group_id,
            owner_actor_id,
            visibility: visibility.map(|v| v as u8),
            is_deleted,
        }))
    }

    async fn read_entity_security_with_fallback(
        &self,
        partition: PartitionId,
        scenario_id: Option<ScenarioId>,
        entity_id: Id,
    ) -> MnemeResult<Option<EntitySecurity>> {
        if let Some(security) = self
            .read_entity_security_in_partition(partition, scenario_id, entity_id)
            .await?
        {
            return Ok(Some(security));
        }
        if scenario_id.is_some() {
            return self
                .read_entity_security_in_partition(partition, None, entity_id)
                .await;
        }
        Ok(None)
    }

    fn is_entity_visible(ctx: &SecurityContext, entity: &EntitySecurity) -> bool {
        if entity.is_deleted {
            return false;
        }
        if let Some(min_visibility) = ctx.min_visibility {
            let value = entity.visibility.unwrap_or(0);
            if value < min_visibility {
                return false;
            }
        }
        if let Some(groups) = &ctx.allowed_acl_groups
            && let Some(group_id) = &entity.acl_group_id
            && !groups.iter().any(|allowed| allowed == group_id)
        {
            return false;
        }
        if let Some(owners) = &ctx.allowed_owner_ids
            && let Some(owner) = entity.owner_actor_id
            && !owners.iter().any(|allowed| allowed == &owner)
        {
            return false;
        }
        true
    }

    async fn read_entity_row_with_fallback(
        &self,
        partition: PartitionId,
        scenario_id: Option<crate::ScenarioId>,
        entity_id: Id,
    ) -> MnemeResult<(PartitionId, EntityKind, Option<Id>, bool)> {
        if let Some(scenario_id) = scenario_id
            && let Some((kind, type_id, is_deleted)) = self
                .read_entity_row_in_partition(partition, Some(scenario_id), entity_id)
                .await?
        {
            return Ok((partition, kind, type_id, is_deleted));
        }
        if let Some((kind, type_id, is_deleted)) = self
            .read_entity_row_in_partition(partition, None, entity_id)
            .await?
        {
            return Ok((partition, kind, type_id, is_deleted));
        }
        Err(MnemeError::not_found("entity not found"))
    }

    async fn read_entity_row_in_partition_conn<C: ConnectionTrait>(
        &self,
        conn: &C,
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
        let row = query_one(conn, &select).await?;
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

    async fn fetch_field_def<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        field_id: Id,
    ) -> MnemeResult<FieldConstraints> {
        self.fetch_field_def_in_partition(conn, partition, field_id)
            .await?
            .ok_or_else(|| MnemeError::not_found("field not found"))
    }

    async fn fetch_field_def_with_entity<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        scenario_id: Option<ScenarioId>,
        field_id: Id,
        entity_id: Id,
    ) -> MnemeResult<FieldConstraints> {
        let mut constraints = self.fetch_field_def(conn, partition, field_id).await?;
        if let Some(type_id) = self
            .read_entity_row_with_fallback_conn(conn, partition, scenario_id, entity_id)
            .await?
            .and_then(|(_, type_id, _)| type_id)
        {
            let lineage = self.type_lineage(conn, partition, type_id).await?;
            let mut found_attachment = false;
            for lineage_type in lineage {
                let select = Query::select()
                    .from(AideonTypeFields::Table)
                    .column(AideonTypeFields::DisallowOverlap)
                    .and_where(
                        Expr::col(AideonTypeFields::PartitionId)
                            .eq(id_value(self.backend, partition.0)),
                    )
                    .and_where(
                        Expr::col(AideonTypeFields::TypeId)
                            .eq(id_value(self.backend, lineage_type)),
                    )
                    .and_where(
                        Expr::col(AideonTypeFields::FieldId).eq(id_value(self.backend, field_id)),
                    )
                    .to_owned();
                if let Some(row) = query_one(conn, &select).await? {
                    found_attachment = true;
                    if let Ok(Some(value)) = row
                        .try_get::<Option<bool>>("", &col_name(AideonTypeFields::DisallowOverlap))
                    {
                        constraints.disallow_overlap = value;
                    }
                }
            }
            if !found_attachment {
                self.validate_or_warn(false, "field not attached to type")?;
            }
        }
        Ok(constraints)
    }

    async fn fetch_field_def_in_partition<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        field_id: Id,
    ) -> MnemeResult<Option<FieldConstraints>> {
        let select = Query::select()
            .from(AideonFields::Table)
            .columns([
                AideonFields::ValueType,
                AideonFields::MergePolicy,
                AideonFields::Cardinality,
                AideonFields::IsIndexed,
                AideonFields::DisallowOverlap,
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
        let disallow_overlap: Option<bool> =
            row.try_get("", &col_name(AideonFields::DisallowOverlap))?;
        let value_type = ValueType::from_i16(value_type_raw)
            .ok_or_else(|| MnemeError::storage("invalid value type"))?;
        let merge_policy = MergePolicy::from_i16(merge_policy_raw)
            .ok_or_else(|| MnemeError::storage("invalid merge policy"))?;
        Ok(Some(FieldConstraints {
            value_type,
            merge_policy,
            cardinality_multi: cardinality_raw == 2,
            is_indexed,
            disallow_overlap: disallow_overlap.unwrap_or(false),
        }))
    }

    async fn property_overlap_exists<C: ConnectionTrait>(
        &self,
        conn: &C,
        input: PropertyOverlapInput,
        value_type: ValueType,
    ) -> MnemeResult<bool> {
        match value_type {
            ValueType::Str => {
                self.property_overlap_exists_for(
                    conn,
                    &input,
                    PropertyOverlapTableSpec {
                        table: AideonPropFactStr::Table,
                        partition_col: AideonPropFactStr::PartitionId,
                        scenario_col: AideonPropFactStr::ScenarioId,
                        entity_col: AideonPropFactStr::EntityId,
                        field_col: AideonPropFactStr::FieldId,
                        valid_from_col: AideonPropFactStr::ValidFrom,
                        valid_to_col: AideonPropFactStr::ValidTo,
                    },
                )
                .await
            }
            ValueType::I64 => {
                self.property_overlap_exists_for(
                    conn,
                    &input,
                    PropertyOverlapTableSpec {
                        table: AideonPropFactI64::Table,
                        partition_col: AideonPropFactI64::PartitionId,
                        scenario_col: AideonPropFactI64::ScenarioId,
                        entity_col: AideonPropFactI64::EntityId,
                        field_col: AideonPropFactI64::FieldId,
                        valid_from_col: AideonPropFactI64::ValidFrom,
                        valid_to_col: AideonPropFactI64::ValidTo,
                    },
                )
                .await
            }
            ValueType::F64 => {
                self.property_overlap_exists_for(
                    conn,
                    &input,
                    PropertyOverlapTableSpec {
                        table: AideonPropFactF64::Table,
                        partition_col: AideonPropFactF64::PartitionId,
                        scenario_col: AideonPropFactF64::ScenarioId,
                        entity_col: AideonPropFactF64::EntityId,
                        field_col: AideonPropFactF64::FieldId,
                        valid_from_col: AideonPropFactF64::ValidFrom,
                        valid_to_col: AideonPropFactF64::ValidTo,
                    },
                )
                .await
            }
            ValueType::Bool => {
                self.property_overlap_exists_for(
                    conn,
                    &input,
                    PropertyOverlapTableSpec {
                        table: AideonPropFactBool::Table,
                        partition_col: AideonPropFactBool::PartitionId,
                        scenario_col: AideonPropFactBool::ScenarioId,
                        entity_col: AideonPropFactBool::EntityId,
                        field_col: AideonPropFactBool::FieldId,
                        valid_from_col: AideonPropFactBool::ValidFrom,
                        valid_to_col: AideonPropFactBool::ValidTo,
                    },
                )
                .await
            }
            ValueType::Time => {
                self.property_overlap_exists_for(
                    conn,
                    &input,
                    PropertyOverlapTableSpec {
                        table: AideonPropFactTime::Table,
                        partition_col: AideonPropFactTime::PartitionId,
                        scenario_col: AideonPropFactTime::ScenarioId,
                        entity_col: AideonPropFactTime::EntityId,
                        field_col: AideonPropFactTime::FieldId,
                        valid_from_col: AideonPropFactTime::ValidFrom,
                        valid_to_col: AideonPropFactTime::ValidTo,
                    },
                )
                .await
            }
            ValueType::Ref => {
                self.property_overlap_exists_for(
                    conn,
                    &input,
                    PropertyOverlapTableSpec {
                        table: AideonPropFactRef::Table,
                        partition_col: AideonPropFactRef::PartitionId,
                        scenario_col: AideonPropFactRef::ScenarioId,
                        entity_col: AideonPropFactRef::EntityId,
                        field_col: AideonPropFactRef::FieldId,
                        valid_from_col: AideonPropFactRef::ValidFrom,
                        valid_to_col: AideonPropFactRef::ValidTo,
                    },
                )
                .await
            }
            ValueType::Blob => {
                self.property_overlap_exists_for(
                    conn,
                    &input,
                    PropertyOverlapTableSpec {
                        table: AideonPropFactBlob::Table,
                        partition_col: AideonPropFactBlob::PartitionId,
                        scenario_col: AideonPropFactBlob::ScenarioId,
                        entity_col: AideonPropFactBlob::EntityId,
                        field_col: AideonPropFactBlob::FieldId,
                        valid_from_col: AideonPropFactBlob::ValidFrom,
                        valid_to_col: AideonPropFactBlob::ValidTo,
                    },
                )
                .await
            }
            ValueType::Json => {
                self.property_overlap_exists_for(
                    conn,
                    &input,
                    PropertyOverlapTableSpec {
                        table: AideonPropFactJson::Table,
                        partition_col: AideonPropFactJson::PartitionId,
                        scenario_col: AideonPropFactJson::ScenarioId,
                        entity_col: AideonPropFactJson::EntityId,
                        field_col: AideonPropFactJson::FieldId,
                        valid_from_col: AideonPropFactJson::ValidFrom,
                        valid_to_col: AideonPropFactJson::ValidTo,
                    },
                )
                .await
            }
        }
    }

    async fn property_overlap_exists_for<
        C,
        TTable,
        TPartition,
        TScenario,
        TEntity,
        TField,
        TValidFrom,
        TValidTo,
    >(
        &self,
        conn: &C,
        input: &PropertyOverlapInput,
        spec: PropertyOverlapTableSpec<
            TTable,
            TPartition,
            TScenario,
            TEntity,
            TField,
            TValidFrom,
            TValidTo,
        >,
    ) -> MnemeResult<bool>
    where
        C: ConnectionTrait,
        TTable: Iden + Copy,
        TPartition: Iden + Copy,
        TScenario: Iden + Copy,
        TEntity: Iden + Copy,
        TField: Iden + Copy,
        TValidFrom: Iden + Copy,
        TValidTo: Iden + Copy,
    {
        let mut select = Query::select()
            .from(spec.table)
            .column(spec.valid_from_col)
            .and_where(
                Expr::col((spec.table, spec.partition_col))
                    .eq(id_value(self.backend, input.partition.0)),
            )
            .and_where(
                Expr::col((spec.table, spec.entity_col))
                    .eq(id_value(self.backend, input.entity_id)),
            )
            .and_where(
                Expr::col((spec.table, spec.field_col)).eq(id_value(self.backend, input.field_id)),
            )
            .limit(1)
            .to_owned();
        if let Some(scenario_id) = input.scenario_id {
            select.and_where(
                Expr::col((spec.table, spec.scenario_col))
                    .eq(id_value(self.backend, scenario_id.0)),
            );
        } else {
            select.and_where(Expr::col((spec.table, spec.scenario_col)).is_null());
        }
        let overlap_expr = match input.valid_to {
            Some(valid_to) => Expr::col((spec.table, spec.valid_from_col))
                .lt(valid_to.0)
                .and(
                    Expr::col((spec.table, spec.valid_to_col))
                        .is_null()
                        .or(Expr::col((spec.table, spec.valid_to_col)).gt(input.valid_from.0)),
                ),
            None => Expr::col((spec.table, spec.valid_to_col))
                .is_null()
                .or(Expr::col((spec.table, spec.valid_to_col)).gt(input.valid_from.0)),
        };
        select.and_where(overlap_expr);
        Ok(query_one(conn, &select).await?.is_some())
    }

    async fn ensure_no_overlap(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        input: &SetPropIntervalInput,
        value_type: ValueType,
    ) -> MnemeResult<()> {
        let overlap_input = PropertyOverlapInput {
            partition: input.partition,
            scenario_id: input.scenario_id,
            entity_id: input.entity_id,
            field_id: input.field_id,
            valid_from: input.valid_from,
            valid_to: input.valid_to,
        };
        if self
            .property_overlap_exists(tx, overlap_input, value_type)
            .await?
        {
            return Err(MnemeError::conflict("overlapping intervals disallowed"));
        }
        Ok(())
    }

    async fn ensure_no_overlap_clear(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        input: &ClearPropIntervalInput,
        value_type: ValueType,
    ) -> MnemeResult<()> {
        let overlap_input = PropertyOverlapInput {
            partition: input.partition,
            scenario_id: input.scenario_id,
            entity_id: input.entity_id,
            field_id: input.field_id,
            valid_from: input.valid_from,
            valid_to: input.valid_to,
        };
        if self
            .property_overlap_exists(tx, overlap_input, value_type)
            .await?
        {
            return Err(MnemeError::conflict("overlapping intervals disallowed"));
        }
        Ok(())
    }

    async fn record_overlap_warning(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        input: &SetPropIntervalInput,
        value_type: ValueType,
    ) -> MnemeResult<()> {
        let overlap_input = PropertyOverlapInput {
            partition: input.partition,
            scenario_id: input.scenario_id,
            entity_id: input.entity_id,
            field_id: input.field_id,
            valid_from: input.valid_from,
            valid_to: input.valid_to,
        };
        if !self
            .property_overlap_exists(tx, overlap_input, value_type)
            .await?
        {
            return Ok(());
        }
        self.insert_overlap_finding(
            tx,
            OverlapFindingInput {
                partition: input.partition,
                scenario_id: input.scenario_id,
                entity_id: input.entity_id,
                field_id: input.field_id,
                valid_from: input.valid_from,
                valid_to: input.valid_to,
                op_kind: "set_property",
            },
        )
        .await
    }

    async fn record_overlap_warning_clear(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        input: &ClearPropIntervalInput,
        value_type: ValueType,
    ) -> MnemeResult<()> {
        let overlap_input = PropertyOverlapInput {
            partition: input.partition,
            scenario_id: input.scenario_id,
            entity_id: input.entity_id,
            field_id: input.field_id,
            valid_from: input.valid_from,
            valid_to: input.valid_to,
        };
        if !self
            .property_overlap_exists(tx, overlap_input, value_type)
            .await?
        {
            return Ok(());
        }
        self.insert_overlap_finding(
            tx,
            OverlapFindingInput {
                partition: input.partition,
                scenario_id: input.scenario_id,
                entity_id: input.entity_id,
                field_id: input.field_id,
                valid_from: input.valid_from,
                valid_to: input.valid_to,
                op_kind: "clear_property",
            },
        )
        .await
    }

    async fn insert_overlap_finding(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        input: OverlapFindingInput<'_>,
    ) -> MnemeResult<()> {
        const SEVERITY_WARN: i64 = 1;
        let run_id = Id::new();
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
                id_value(self.backend, input.partition.0).into(),
                id_value(self.backend, run_id).into(),
                opt_id_value(self.backend, input.scenario_id.map(|s| s.0)).into(),
                input.valid_from.0.into(),
                now.into(),
                serde_json::json!({
                    "reason": "overlap_warning",
                    "op_kind": input.op_kind,
                })
                .to_string()
                .into(),
                now.into(),
            ])
            .to_owned();
        exec(tx, &insert_run).await?;

        let insert_finding = Query::insert()
            .into_table(AideonIntegrityFindings::Table)
            .columns([
                AideonIntegrityFindings::PartitionId,
                AideonIntegrityFindings::RunId,
                AideonIntegrityFindings::FindingType,
                AideonIntegrityFindings::Severity,
                AideonIntegrityFindings::SubjectEntityId,
                AideonIntegrityFindings::DetailsJson,
            ])
            .values_panic([
                id_value(self.backend, input.partition.0).into(),
                id_value(self.backend, run_id).into(),
                "overlap_warning".into(),
                SEVERITY_WARN.into(),
                id_value(self.backend, input.entity_id).into(),
                serde_json::json!({
                    "field_id": input.field_id,
                    "valid_from": input.valid_from.0,
                    "valid_to": input.valid_to.map(|v| v.0),
                })
                .to_string()
                .into(),
            ])
            .to_owned();
        exec(tx, &insert_finding).await?;

        let insert_head = Query::insert()
            .into_table(AideonIntegrityHead::Table)
            .columns([
                AideonIntegrityHead::PartitionId,
                AideonIntegrityHead::ScenarioId,
                AideonIntegrityHead::RunId,
                AideonIntegrityHead::UpdatedAssertedAtHlc,
            ])
            .values_panic([
                id_value(self.backend, input.partition.0).into(),
                opt_id_value(self.backend, input.scenario_id.map(|s| s.0)).into(),
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
}

const MICROS_PER_DAY: i64 = 86_400_000_000;
const INDEX_REBUILD_BATCH: u64 = 1_000;
const SUBSCRIPTION_POLL_INTERVAL_MS: u64 = 250;
const SUBSCRIPTION_POLL_LIMIT: u32 = 250;

fn valid_bucket(valid_from: ValidTime) -> i64 {
    valid_from.0 / MICROS_PER_DAY
}

fn retention_cutoff_hlc(days: u32) -> i64 {
    let now = Hlc::now();
    let cutoff = now
        .physical_micros()
        .saturating_sub(days as i64 * MICROS_PER_DAY);
    Hlc::from_physical_micros(cutoff).as_i64()
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

fn check_value_limits(value: &Value, limits: &MnemeLimits) -> MnemeResult<()> {
    if let Value::Blob(payload) = value
        && payload.len() > limits.max_blob_bytes
    {
        return Err(MnemeError::validation("blob value exceeds limit"));
    }
    Ok(())
}

impl MnemeStore {
    async fn apply_metamodel_batch(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        op_id: OpId,
        asserted_at: Hlc,
        batch: &MetamodelBatch,
    ) -> MnemeResult<()> {
        if batch.metamodel_version.is_some() || batch.metamodel_source.is_some() {
            let insert_version = Query::insert()
                .into_table(AideonMetamodelVersions::Table)
                .columns([
                    AideonMetamodelVersions::PartitionId,
                    AideonMetamodelVersions::Version,
                    AideonMetamodelVersions::Source,
                    AideonMetamodelVersions::OpId,
                    AideonMetamodelVersions::CreatedAssertedAtHlc,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    batch
                        .metamodel_version
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string())
                        .into(),
                    batch.metamodel_source.clone().into(),
                    id_value(self.backend, op_id.0).into(),
                    asserted_at.as_i64().into(),
                ])
                .on_conflict(
                    OnConflict::columns([
                        AideonMetamodelVersions::PartitionId,
                        AideonMetamodelVersions::Version,
                    ])
                    .update_columns([
                        AideonMetamodelVersions::Source,
                        AideonMetamodelVersions::OpId,
                        AideonMetamodelVersions::CreatedAssertedAtHlc,
                    ])
                    .to_owned(),
                )
                .to_owned();
            exec(tx, &insert_version).await?;
        }

        for t in &batch.types {
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
            exec(tx, &insert).await?;

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
                exec(tx, &insert_extends).await?;
            }
        }

        for f in &batch.fields {
            if !f.cardinality_multi
                && matches!(f.merge_policy, MergePolicy::Mv | MergePolicy::OrSet)
            {
                return Err(MnemeError::invalid(
                    "cardinality single not compatible with merge policy",
                ));
            }
            if f.is_indexed && matches!(f.value_type, ValueType::Blob | ValueType::Json) {
                self.validate_or_warn(false, "blob/json fields cannot be indexed")?;
            }
            let insert = Query::insert()
                .into_table(AideonFields::Table)
                .columns([
                    AideonFields::PartitionId,
                    AideonFields::FieldId,
                    AideonFields::ValueType,
                    AideonFields::Cardinality,
                    AideonFields::MergePolicy,
                    AideonFields::IsIndexed,
                    AideonFields::DisallowOverlap,
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
                    f.disallow_overlap.into(),
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
                            AideonFields::DisallowOverlap,
                            AideonFields::Label,
                            AideonFields::IsDeleted,
                            AideonFields::UpdatedAssertedAtHlc,
                        ])
                        .to_owned(),
                )
                .to_owned();
            exec(tx, &insert).await?;
        }

        for tf in &batch.type_fields {
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
                    AideonTypeFields::DisallowOverlap,
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
                    tf.disallow_overlap.into(),
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
                        AideonTypeFields::DisallowOverlap,
                        AideonTypeFields::UpdatedAssertedAtHlc,
                    ])
                    .to_owned(),
                )
                .to_owned();
            exec(tx, &insert).await?;
        }

        for rule in &batch.edge_type_rules {
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
            exec(tx, &insert).await?;
        }
        Ok(())
    }
}
#[async_trait]
impl MetamodelApi for MnemeStore {
    async fn upsert_metamodel_batch(
        &self,
        partition: PartitionId,
        actor: ActorId,
        asserted_at: Hlc,
        batch: MetamodelBatch,
    ) -> MnemeResult<OpId> {
        let tx = self.conn.begin().await?;
        self.ensure_partition(&tx, partition, actor).await?;
        let payload = OpPayload::UpsertMetamodelBatch(batch.clone());
        let (op_id, _asserted_at, _payload, _op_type) = self
            .insert_op(&tx, partition, actor, asserted_at, &payload)
            .await?;
        self.apply_metamodel_batch(&tx, partition, op_id, asserted_at, &batch)
            .await?;
        tx.commit().await?;
        Ok(op_id)
    }

    async fn compile_effective_schema(
        &self,
        partition: PartitionId,
        _actor: ActorId,
        asserted_at: Hlc,
        type_id: Id,
    ) -> MnemeResult<SchemaVersion> {
        self.compile_effective_schema_with_conn(&self.conn, partition, asserted_at, type_id)
            .await
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
                Expr::col(AideonEdgeTypeRules::EdgeTypeId).eq(id_value(self.backend, edge_type_id)),
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
        self.validate_create_node(&tx, &input).await?;
        let payload = OpPayload::CreateNode(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(
                &tx,
                input.partition,
                input.actor,
                input.asserted_at,
                &payload,
            )
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
                AideonEntities::AclGroupId,
                AideonEntities::OwnerActorId,
                AideonEntities::Visibility,
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
                input.acl_group_id.clone().into(),
                opt_id_value(self.backend, input.owner_actor_id.map(|actor| actor.0)).into(),
                input.visibility.map(|v| v as i64).into(),
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
        self.validate_create_edge(&tx, &input).await?;
        let payload = OpPayload::CreateEdge(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(
                &tx,
                input.partition,
                input.actor,
                input.asserted_at,
                &payload,
            )
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
                AideonEntities::AclGroupId,
                AideonEntities::OwnerActorId,
                AideonEntities::Visibility,
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
                input.acl_group_id.clone().into(),
                opt_id_value(self.backend, input.owner_actor_id.map(|actor| actor.0)).into(),
                input.visibility.map(|v| v as i64).into(),
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
                    .do_nothing()
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
                .do_nothing()
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
        self.validate_edge_exists(&tx, input.partition, input.scenario_id, input.edge_id)
            .await?;
        let payload = OpPayload::SetEdgeExistenceInterval(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(
                &tx,
                input.partition,
                input.actor,
                input.asserted_at,
                &payload,
            )
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
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(&tx, partition, actor, asserted_at, &payload)
            .await?;

        let update = Query::update()
            .table(AideonEntities::Table)
            .values([
                (AideonEntities::IsDeleted, true.into()),
                (
                    AideonEntities::UpdatedOpId,
                    id_value(self.backend, op_id.0).into(),
                ),
                (
                    AideonEntities::UpdatedAssertedAtHlc,
                    asserted_at.as_i64().into(),
                ),
            ])
            .and_where(
                Expr::col(AideonEntities::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .and_where(Expr::col(AideonEntities::EntityId).eq(id_value(self.backend, entity_id)))
            .to_owned();
        exec(&tx, &update).await?;

        let select_kind = Query::select()
            .from(AideonEntities::Table)
            .column(AideonEntities::EntityKind)
            .and_where(
                Expr::col(AideonEntities::PartitionId).eq(id_value(self.backend, partition.0)),
            )
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
            .and_where(
                Expr::col(AideonGraphProjectionEdges::EdgeId).eq(id_value(self.backend, entity_id)),
            )
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
        let constraints = self
            .fetch_field_def_with_entity(
                &tx,
                input.partition,
                input.scenario_id,
                input.field_id,
                input.entity_id,
            )
            .await?;
        if !constraints.cardinality_multi && constraints.merge_policy == MergePolicy::Mv {
            return Err(MnemeError::validation(
                "cardinality single is incompatible with MV merge policy",
            ));
        }
        if constraints.value_type != input.value.value_type() {
            return Err(MnemeError::invalid("value type mismatch"));
        }
        check_value_limits(&input.value, &self.limits)?;
        if matches!(
            constraints.merge_policy,
            MergePolicy::OrSet | MergePolicy::Counter
        ) {
            return Err(MnemeError::validation(
                "use or_set_update/counter_update for this field",
            ));
        }
        if constraints.disallow_overlap {
            self.ensure_no_overlap(&tx, &input, constraints.value_type)
                .await?;
        } else if self.record_overlap_warnings
            && matches!(
                constraints.merge_policy,
                MergePolicy::Lww | MergePolicy::Text
            )
        {
            self.record_overlap_warning(&tx, &input, constraints.value_type)
                .await?;
        }
        let payload = OpPayload::SetProperty(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(
                &tx,
                input.partition,
                input.actor,
                input.asserted_at,
                &payload,
            )
            .await?;
        insert_property_fact(
            &tx,
            self.backend,
            PropertyFactInsertInput {
                partition: input.partition,
                scenario_id: input.scenario_id,
                entity_id: input.entity_id,
                field_id: input.field_id,
                value: &input.value,
                valid_from: input.valid_from,
                valid_to: input.valid_to,
                layer: input.layer,
                asserted_at,
                op_id,
                is_tombstone: false,
            },
        )
        .await?;
        self.maybe_failpoint("after_property_fact_insert")?;
        if constraints.is_indexed {
            insert_index_row(
                &tx,
                self.backend,
                IndexInsertInput {
                    partition: input.partition,
                    scenario_id: input.scenario_id,
                    field_id: input.field_id,
                    entity_id: input.entity_id,
                    value: &input.value,
                    valid_from: input.valid_from,
                    valid_to: input.valid_to,
                    asserted_at,
                    layer: input.layer,
                },
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
        let constraints = self
            .fetch_field_def_with_entity(
                &tx,
                input.partition,
                input.scenario_id,
                input.field_id,
                input.entity_id,
            )
            .await?;
        if matches!(
            constraints.merge_policy,
            MergePolicy::OrSet | MergePolicy::Counter
        ) {
            return Err(MnemeError::validation(
                "use or_set_update/counter_update for this field",
            ));
        }
        if constraints.disallow_overlap {
            self.ensure_no_overlap_clear(&tx, &input, constraints.value_type)
                .await?;
        } else if self.record_overlap_warnings {
            self.record_overlap_warning_clear(&tx, &input, constraints.value_type)
                .await?;
        }
        let payload = OpPayload::ClearProperty(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(
                &tx,
                input.partition,
                input.actor,
                input.asserted_at,
                &payload,
            )
            .await?;
        let value = match constraints.value_type {
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
            self.backend,
            PropertyFactInsertInput {
                partition: input.partition,
                scenario_id: input.scenario_id,
                entity_id: input.entity_id,
                field_id: input.field_id,
                value: &value,
                valid_from: input.valid_from,
                valid_to: input.valid_to,
                layer: input.layer,
                asserted_at,
                op_id,
                is_tombstone: true,
            },
        )
        .await?;
        if constraints.is_indexed {
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
        let constraints = self
            .fetch_field_def_with_entity(
                &tx,
                input.partition,
                input.scenario_id,
                input.field_id,
                input.entity_id,
            )
            .await?;
        if constraints.value_type != input.element.value_type() {
            return Err(MnemeError::invalid("value type mismatch"));
        }
        check_value_limits(&input.element, &self.limits)?;
        if constraints.merge_policy != MergePolicy::OrSet {
            return Err(MnemeError::invalid(
                "field is not configured for OR_SET merge policy",
            ));
        }
        let payload = OpPayload::OrSetUpdate(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(
                &tx,
                input.partition,
                input.actor,
                input.asserted_at,
                &payload,
            )
            .await?;
        let is_tombstone = matches!(input.op, SetOp::Remove);
        insert_property_fact(
            &tx,
            self.backend,
            PropertyFactInsertInput {
                partition: input.partition,
                scenario_id: input.scenario_id,
                entity_id: input.entity_id,
                field_id: input.field_id,
                value: &input.element,
                valid_from: input.valid_from,
                valid_to: input.valid_to,
                layer: input.layer,
                asserted_at,
                op_id,
                is_tombstone,
            },
        )
        .await?;
        if constraints.is_indexed {
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
                    self.backend,
                    IndexInsertInput {
                        partition: input.partition,
                        scenario_id: input.scenario_id,
                        field_id: input.field_id,
                        entity_id: input.entity_id,
                        value: &input.element,
                        valid_from: input.valid_from,
                        valid_to: input.valid_to,
                        asserted_at,
                        layer: input.layer,
                    },
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
        let constraints = self
            .fetch_field_def_with_entity(
                &tx,
                input.partition,
                input.scenario_id,
                input.field_id,
                input.entity_id,
            )
            .await?;
        if constraints.merge_policy != MergePolicy::Counter {
            return Err(MnemeError::invalid(
                "field is not configured for COUNTER merge policy",
            ));
        }
        let payload = OpPayload::CounterUpdate(input.clone());
        let (op_id, asserted_at, _payload, _op_type) = self
            .insert_op(
                &tx,
                input.partition,
                input.actor,
                input.asserted_at,
                &payload,
            )
            .await?;
        let value = Value::I64(input.delta);
        insert_property_fact(
            &tx,
            self.backend,
            PropertyFactInsertInput {
                partition: input.partition,
                scenario_id: input.scenario_id,
                entity_id: input.entity_id,
                field_id: input.field_id,
                value: &value,
                valid_from: input.valid_from,
                valid_to: input.valid_to,
                layer: input.layer,
                asserted_at,
                op_id,
                is_tombstone: false,
            },
        )
        .await?;
        if constraints.is_indexed {
            insert_index_row(
                &tx,
                self.backend,
                IndexInsertInput {
                    partition: input.partition,
                    scenario_id: input.scenario_id,
                    field_id: input.field_id,
                    entity_id: input.entity_id,
                    value: &value,
                    valid_from: input.valid_from,
                    valid_to: input.valid_to,
                    asserted_at,
                    layer: input.layer,
                },
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
        if let Some(ctx) = &input.security_context
            && let Some(security) = self
                .read_entity_security_with_fallback(
                    input.partition,
                    input.scenario_id,
                    input.entity_id,
                )
                .await?
            && !Self::is_entity_visible(ctx, &security)
        {
            return Err(MnemeError::not_found("entity not visible"));
        }
        let (resolved_partition, kind, type_id, is_deleted) = self
            .read_entity_row_with_fallback(input.partition, input.scenario_id, input.entity_id)
            .await?;

        let effective_schema = if let Some(type_id) = type_id {
            if input.field_ids.is_none() || input.include_defaults {
                match self
                    .get_effective_schema(resolved_partition, type_id)
                    .await?
                {
                    Some(schema) => Some(schema),
                    None => {
                        let _ = self
                            .compile_effective_schema(
                                resolved_partition,
                                ActorId(Id::new()),
                                Hlc::now(),
                                type_id,
                            )
                            .await;
                        match self
                            .get_effective_schema(resolved_partition, type_id)
                            .await?
                        {
                            Some(schema) => Some(schema),
                            None => Some(
                                self.build_effective_schema(resolved_partition, type_id)
                                    .await?,
                            ),
                        }
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        let field_ids = if let Some(field_ids) = input.field_ids.clone() {
            field_ids
        } else if let Some(schema) = &effective_schema {
            schema.fields.iter().map(|field| field.field_id).collect()
        } else {
            Vec::new()
        };

        let defaults_by_field = effective_schema.as_ref().map(|schema| {
            schema
                .fields
                .iter()
                .map(|field| (field.field_id, field.default_value.clone()))
                .collect::<HashMap<Id, Option<Value>>>()
        });

        let mut properties = HashMap::new();
        for field_id in field_ids {
            let constraints = self
                .fetch_field_def(&self.conn, resolved_partition, field_id)
                .await?;
            let facts = fetch_property_facts_with_fallback(
                &self.conn,
                PropertyFactQueryInput {
                    partition: resolved_partition,
                    scenario_id: input.scenario_id,
                    entity_id: input.entity_id,
                    field_id,
                    at_valid_time: input.at_valid_time,
                    as_of_asserted_at: input.as_of_asserted_at,
                    value_type: constraints.value_type,
                },
                self.backend,
            )
            .await?;
            if facts.is_empty() {
                if input.include_defaults
                    && let Some(default) = defaults_by_field
                        .as_ref()
                        .and_then(|defaults| defaults.get(&field_id))
                        .and_then(|value| value.clone())
                {
                    properties.insert(field_id, ReadValue::Single(default));
                }
                continue;
            }
            let resolved = resolve_property(constraints.merge_policy, facts, &self.limits)?;
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
        if let Some(ctx) = &input.security_context
            && let Some(security) = self
                .read_entity_security_with_fallback(
                    input.partition,
                    input.scenario_id,
                    input.from_entity_id,
                )
                .await?
            && !Self::is_entity_visible(ctx, &security)
        {
            return Ok(Vec::new());
        }
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
                if let Some(ctx) = &input.security_context {
                    if let Some(edge_security) = self
                        .read_entity_security_with_fallback(
                            input.partition,
                            input.scenario_id,
                            edge.edge_id,
                        )
                        .await?
                        && !Self::is_entity_visible(ctx, &edge_security)
                    {
                        continue;
                    }
                    if let Some(src_security) = self
                        .read_entity_security_with_fallback(
                            input.partition,
                            input.scenario_id,
                            edge.src_id,
                        )
                        .await?
                        && !Self::is_entity_visible(ctx, &src_security)
                    {
                        continue;
                    }
                    if let Some(dst_security) = self
                        .read_entity_security_with_fallback(
                            input.partition,
                            input.scenario_id,
                            edge.dst_id,
                        )
                        .await?
                        && !Self::is_entity_visible(ctx, &dst_security)
                    {
                        continue;
                    }
                }
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
        let mut force_full_scan = false;
        for filter in &input.filters {
            let constraints = self
                .fetch_field_def(&self.conn, input.partition, filter.field_id)
                .await?;
            if !constraints.is_indexed {
                self.validate_or_warn(false, "field is not indexed")?;
                force_full_scan = true;
                continue;
            }
            if filter.value.value_type() != constraints.value_type {
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

        let candidates: Vec<Id> = if force_full_scan {
            list_entities_without_filters(self, input.clone(), self.backend)
                .await?
                .into_iter()
                .map(|item| item.entity_id)
                .collect()
        } else {
            match candidate_ids {
                Some(ids) => ids.into_iter().collect(),
                None => return list_entities_without_filters(self, input, self.backend).await,
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
            if let Some(ctx) = &input.security_context
                && let Some(security) = self
                    .read_entity_security_with_fallback(
                        input.partition,
                        input.scenario_id,
                        entity_id,
                    )
                    .await?
                && !Self::is_entity_visible(ctx, &security)
            {
                continue;
            }
            if let Some(kind_filter) = input.kind
                && kind != kind_filter
            {
                continue;
            }
            if let Some(type_filter) = input.type_id
                && type_id != Some(type_filter)
            {
                continue;
            }
            let mut matches = true;
            for filter in &input.filters {
                let constraints = self
                    .fetch_field_def(&self.conn, input.partition, filter.field_id)
                    .await?;
                let facts = fetch_property_facts_with_fallback(
                    &self.conn,
                    PropertyFactQueryInput {
                        partition: input.partition,
                        scenario_id: input.scenario_id,
                        entity_id,
                        field_id: filter.field_id,
                        at_valid_time: input.at_valid_time,
                        as_of_asserted_at: input.as_of_asserted_at,
                        value_type: constraints.value_type,
                    },
                    self.backend,
                )
                .await?;
                let resolved = resolve_property(constraints.merge_policy, facts, &self.limits)?;
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
            let cursor_id = crate::decode_entity_cursor(cursor)?;
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
            if let Some(ctx) = &input.security_context {
                if let Some(edge_security) = self
                    .read_entity_security_with_fallback(
                        input.partition,
                        input.scenario_id,
                        edge.edge_id,
                    )
                    .await?
                    && !Self::is_entity_visible(ctx, &edge_security)
                {
                    continue;
                }
                if let Some(src_security) = self
                    .read_entity_security_with_fallback(
                        input.partition,
                        input.scenario_id,
                        edge.src_id,
                    )
                    .await?
                    && !Self::is_entity_visible(ctx, &src_security)
                {
                    continue;
                }
                if let Some(dst_security) = self
                    .read_entity_security_with_fallback(
                        input.partition,
                        input.scenario_id,
                        edge.dst_id,
                    )
                    .await?
                    && !Self::is_entity_visible(ctx, &dst_security)
                {
                    continue;
                }
            }
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
                if let Some(ctx) = &input.security_context {
                    if let Some(edge_security) = self
                        .read_entity_security_with_fallback(input.partition, None, edge.edge_id)
                        .await?
                        && !Self::is_entity_visible(ctx, &edge_security)
                    {
                        continue;
                    }
                    if let Some(src_security) = self
                        .read_entity_security_with_fallback(input.partition, None, edge.src_id)
                        .await?
                        && !Self::is_entity_visible(ctx, &src_security)
                    {
                        continue;
                    }
                    if let Some(dst_security) = self
                        .read_entity_security_with_fallback(input.partition, None, edge.dst_id)
                        .await?
                        && !Self::is_entity_visible(ctx, &dst_security)
                    {
                        continue;
                    }
                }
                edges.push(edge);
                if edges.len() >= limit {
                    break;
                }
            }
        }

        Ok(edges)
    }

    async fn get_graph_degree_stats(
        &self,
        input: crate::GetGraphDegreeStatsInput,
    ) -> MnemeResult<Vec<crate::GraphDegreeStat>> {
        let mut select = Query::select()
            .from(AideonGraphDegreeStats::Table)
            .columns([
                (
                    AideonGraphDegreeStats::Table,
                    AideonGraphDegreeStats::EntityId,
                ),
                (
                    AideonGraphDegreeStats::Table,
                    AideonGraphDegreeStats::OutDegree,
                ),
                (
                    AideonGraphDegreeStats::Table,
                    AideonGraphDegreeStats::InDegree,
                ),
                (
                    AideonGraphDegreeStats::Table,
                    AideonGraphDegreeStats::AsOfValidTime,
                ),
                (
                    AideonGraphDegreeStats::Table,
                    AideonGraphDegreeStats::ComputedAssertedAtHlc,
                ),
            ])
            .and_where(
                Expr::col(AideonGraphDegreeStats::PartitionId)
                    .eq(id_value(self.backend, input.partition.0)),
            )
            .to_owned();
        if let Some(scenario_id) = input.scenario_id {
            select.and_where(
                Expr::col(AideonGraphDegreeStats::ScenarioId)
                    .eq(id_value(self.backend, scenario_id.0)),
            );
        } else {
            select.and_where(Expr::col(AideonGraphDegreeStats::ScenarioId).is_null());
        }
        if let Some(as_of_valid_time) = input.as_of_valid_time {
            select
                .and_where(Expr::col(AideonGraphDegreeStats::AsOfValidTime).eq(as_of_valid_time.0));
        } else {
            select.and_where(Expr::col(AideonGraphDegreeStats::AsOfValidTime).is_null());
        }
        if let Some(entity_ids) = input.entity_ids {
            select.and_where(
                Expr::col(AideonGraphDegreeStats::EntityId)
                    .is_in(entity_ids.into_iter().map(|id| id_value(self.backend, id))),
            );
        }
        select.order_by(AideonGraphDegreeStats::EntityId, Order::Asc);
        if let Some(limit) = input.limit {
            select.limit(limit as u64);
        }

        let rows = query_all(&self.conn, &select).await?;
        let mut stats = Vec::new();
        for row in rows {
            let entity_id = read_id(&row, AideonGraphDegreeStats::EntityId)?;
            let out_degree: i64 = row.try_get("", &col_name(AideonGraphDegreeStats::OutDegree))?;
            let in_degree: i64 = row.try_get("", &col_name(AideonGraphDegreeStats::InDegree))?;
            let as_of: Option<i64> =
                row.try_get("", &col_name(AideonGraphDegreeStats::AsOfValidTime))?;
            let computed: i64 =
                row.try_get("", &col_name(AideonGraphDegreeStats::ComputedAssertedAtHlc))?;
            stats.push(crate::GraphDegreeStat {
                entity_id,
                out_degree: out_degree as i32,
                in_degree: in_degree as i32,
                as_of_valid_time: as_of.map(ValidTime),
                computed_asserted_at: Hlc::from_i64(computed),
            });
        }
        Ok(stats)
    }

    async fn get_graph_edge_type_counts(
        &self,
        input: crate::GetGraphEdgeTypeCountsInput,
    ) -> MnemeResult<Vec<crate::GraphEdgeTypeCount>> {
        let mut select = Query::select()
            .from(AideonGraphEdgeTypeCounts::Table)
            .columns([
                (
                    AideonGraphEdgeTypeCounts::Table,
                    AideonGraphEdgeTypeCounts::EdgeTypeId,
                ),
                (
                    AideonGraphEdgeTypeCounts::Table,
                    AideonGraphEdgeTypeCounts::Count,
                ),
                (
                    AideonGraphEdgeTypeCounts::Table,
                    AideonGraphEdgeTypeCounts::ComputedAssertedAtHlc,
                ),
            ])
            .and_where(
                Expr::col(AideonGraphEdgeTypeCounts::PartitionId)
                    .eq(id_value(self.backend, input.partition.0)),
            )
            .to_owned();
        if let Some(scenario_id) = input.scenario_id {
            select.and_where(
                Expr::col(AideonGraphEdgeTypeCounts::ScenarioId)
                    .eq(id_value(self.backend, scenario_id.0)),
            );
        } else {
            select.and_where(Expr::col(AideonGraphEdgeTypeCounts::ScenarioId).is_null());
        }
        if let Some(edge_type_ids) = input.edge_type_ids {
            select.and_where(
                Expr::col(AideonGraphEdgeTypeCounts::EdgeTypeId).is_in(
                    edge_type_ids
                        .into_iter()
                        .map(|id| id_value(self.backend, id)),
                ),
            );
        }
        select.order_by(AideonGraphEdgeTypeCounts::EdgeTypeId, Order::Asc);
        if let Some(limit) = input.limit {
            select.limit(limit as u64);
        }

        let rows = query_all(&self.conn, &select).await?;
        let mut counts = Vec::new();
        for row in rows {
            let edge_type_id = read_opt_id(&row, AideonGraphEdgeTypeCounts::EdgeTypeId)?;
            let count: i64 = row.try_get("", &col_name(AideonGraphEdgeTypeCounts::Count))?;
            let computed: i64 = row.try_get(
                "",
                &col_name(AideonGraphEdgeTypeCounts::ComputedAssertedAtHlc),
            )?;
            counts.push(crate::GraphEdgeTypeCount {
                edge_type_id,
                count: count as i32,
                computed_asserted_at: Hlc::from_i64(computed),
            });
        }
        Ok(counts)
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
                Expr::col(AideonPagerankScores::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
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
                (AideonOps::Table, AideonOps::ScenarioId),
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
            let scenario_from_row = read_opt_id(&row, AideonOps::ScenarioId)?;
            let payload: Vec<u8> = row.try_get("", &col_name(AideonOps::Payload))?;
            let payload_scenario = match scenario_from_row {
                Some(id) => Some(ScenarioId(id)),
                None => scenario_id_from_payload(&payload)?,
            };
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
        let mut bulk_scenarios: HashSet<Option<ScenarioId>> = HashSet::new();
        if ops.len() > self.limits.max_ingest_batch {
            return Err(MnemeError::validation("op batch exceeds limit"));
        }
        for op in ops {
            if self.op_exists(&tx, partition, op.op_id).await? {
                continue;
            }
            if op.payload.len() > self.limits.max_op_payload_bytes {
                return Err(MnemeError::invalid("op payload exceeds limit"));
            }
            let payload: OpPayload = serde_json::from_slice(&op.payload)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let payload_scenario = payload_scenario_id(&payload);
            if payload_bulk_mode(&payload) {
                bulk_scenarios.insert(payload_scenario);
            }
            self.bump_hlc_state(&tx, partition, op.asserted_at).await?;
            let insert = Query::insert()
                .into_table(AideonOps::Table)
                .columns([
                    AideonOps::PartitionId,
                    AideonOps::ScenarioId,
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
                    opt_id_value(self.backend, payload_scenario.map(|s| s.0)).into(),
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
            self.apply_op_payload(&tx, partition, op.op_id, op.asserted_at, &payload)
                .await?;
            self.enqueue_jobs_for_op(&tx, partition, &payload).await?;
        }
        if !bulk_scenarios.is_empty() {
            let schema_payload = TriggerJobPayload {
                partition_id: partition,
                scenario_id: None,
                type_id: None,
                schema_version_hint: None,
                reason: "bulk_mode".to_string(),
            };
            let payload = serde_json::to_vec(&schema_payload)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let dedupe_key = format!("schema:{}:all", partition.0.to_uuid_string());
            self.enqueue_job(
                &tx,
                partition,
                JOB_TYPE_SCHEMA_REBUILD,
                payload,
                0,
                Some(dedupe_key),
            )
            .await?;

            for scenario_id in bulk_scenarios {
                let scenario_key = scenario_id
                    .map(|s| s.0.to_uuid_string())
                    .unwrap_or_else(|| "baseline".to_string());
                let integrity_payload = TriggerJobPayload {
                    partition_id: partition,
                    scenario_id,
                    type_id: None,
                    schema_version_hint: None,
                    reason: "bulk_mode".to_string(),
                };
                let payload = serde_json::to_vec(&integrity_payload)
                    .map_err(|err| MnemeError::storage(err.to_string()))?;
                let dedupe_key = format!(
                    "integrity:{}:{}",
                    partition.0.to_uuid_string(),
                    scenario_key
                );
                self.enqueue_job(
                    &tx,
                    partition,
                    JOB_TYPE_INTEGRITY_REFRESH,
                    payload.clone(),
                    0,
                    Some(dedupe_key),
                )
                .await?;
                let analytics_key = format!(
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
                    Some(analytics_key),
                )
                .await?;
            }
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
            .insert_op(
                &tx,
                input.partition,
                input.actor,
                input.asserted_at,
                &payload,
            )
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
const JOB_TYPE_RETENTION: &str = "retention";
const JOB_TYPE_COMPACTION: &str = "compaction";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TriggerJobPayload {
    partition_id: PartitionId,
    scenario_id: Option<ScenarioId>,
    type_id: Option<Id>,
    schema_version_hint: Option<String>,
    reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RetentionJobPayload {
    partition_id: PartitionId,
    scenario_id: Option<ScenarioId>,
    policy: RetentionPolicy,
    reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CompactionJobPayload {
    partition_id: PartitionId,
    scenario_id: Option<ScenarioId>,
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
        if let Some(key) = dedupe_key.as_deref() {
            let select = Query::select()
                .from(AideonJobs::Table)
                .column(AideonJobs::JobId)
                .and_where(
                    Expr::col(AideonJobs::PartitionId).eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonJobs::JobType).eq(job_type))
                .and_where(Expr::col(AideonJobs::DedupeKey).eq(key))
                .and_where(
                    Expr::col(AideonJobs::Status)
                        .is_in([JOB_STATUS_PENDING as i64, JOB_STATUS_RUNNING as i64]),
                )
                .limit(1)
                .to_owned();
            if let Some(row) = query_one(tx, &select).await? {
                let job_id = read_id(&row, AideonJobs::JobId)?;
                return Ok(job_id);
            }
        }

        if self.limits.max_pending_jobs > 0 {
            let select = Query::select()
                .from(AideonJobs::Table)
                .expr_as(
                    Func::count(Expr::col(AideonJobs::JobId)),
                    Alias::new("job_count"),
                )
                .and_where(
                    Expr::col(AideonJobs::PartitionId).eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonJobs::Status).eq(JOB_STATUS_PENDING as i64))
                .to_owned();
            if let Some(row) = query_one(tx, &select).await? {
                let count: i64 = row.try_get("", "job_count")?;
                if count >= self.limits.max_pending_jobs as i64 {
                    return Err(MnemeError::validation("pending job limit exceeded"));
                }
            }
        }

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
            let dedupe_key: Option<String> = row.try_get("", &col_name(AideonJobs::DedupeKey))?;
            let last_error: Option<String> = row.try_get("", &col_name(AideonJobs::LastError))?;
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
                    .or(Expr::col(AideonJobs::Status)
                        .eq(JOB_STATUS_RUNNING as i64)
                        .and(Expr::col(AideonJobs::LeaseExpiresAt).lt(now))),
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
                .and_where(
                    Expr::col(AideonJobs::PartitionId).eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonJobs::JobId).eq(id_value(self.backend, job_id)))
                .and_where(
                    Expr::col(AideonJobs::Status)
                        .eq(JOB_STATUS_PENDING as i64)
                        .or(Expr::col(AideonJobs::Status)
                            .eq(JOB_STATUS_RUNNING as i64)
                            .and(Expr::col(AideonJobs::LeaseExpiresAt).lt(now))),
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
            let dedupe_key: Option<String> = row.try_get("", &col_name(AideonJobs::DedupeKey))?;
            let last_error: Option<String> = row.try_get("", &col_name(AideonJobs::LastError))?;
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
            .and_where(
                Expr::col(AideonGraphDegreeStats::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
            )
            .and_where(match scenario_id {
                Some(scenario_id) => Expr::col(AideonGraphDegreeStats::ScenarioId)
                    .eq(id_value(self.backend, scenario_id.0)),
                None => Expr::col(AideonGraphDegreeStats::ScenarioId).is_null(),
            })
            .to_owned();
        exec(tx, &delete_degree).await?;

        let delete_counts = Query::delete()
            .from_table(AideonGraphEdgeTypeCounts::Table)
            .and_where(
                Expr::col(AideonGraphEdgeTypeCounts::PartitionId)
                    .eq(id_value(self.backend, partition.0)),
            )
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
            .expr_as(
                Func::count(Expr::col(AideonGraphProjectionEdges::EdgeId)),
                Alias::new("cnt"),
            )
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
            .expr_as(
                Func::count(Expr::col(AideonGraphProjectionEdges::EdgeId)),
                Alias::new("cnt"),
            )
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
            let entity_id =
                read_id_by_name(&row, &col_name(AideonGraphProjectionEdges::SrcEntityId))?;
            let count: i64 = row.try_get("", "cnt")?;
            let entry = degrees.entry(entity_id).or_insert((0, 0));
            entry.0 = count as i32;
        }
        for row in in_rows {
            let entity_id =
                read_id_by_name(&row, &col_name(AideonGraphProjectionEdges::DstEntityId))?;
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
            .expr_as(
                Func::count(Expr::col(AideonGraphProjectionEdges::EdgeId)),
                Alias::new("cnt"),
            )
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
                    opt_id_value(self.backend, edge_type_id).into(),
                    count.into(),
                    now.into(),
                ])
                .to_owned();
            exec(tx, &insert).await?;
        }

        Ok(())
    }

    async fn apply_retention_policy(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
        policy: &RetentionPolicy,
    ) -> MnemeResult<()> {
        if let Some(days) = policy.keep_ops_days {
            let cutoff = retention_cutoff_hlc(days);
            let op_subquery = Query::select()
                .from(AideonOps::Table)
                .column(AideonOps::OpId)
                .and_where(
                    Expr::col(AideonOps::PartitionId).eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonOps::AssertedAtHlc).lt(cutoff))
                .to_owned();
            let delete_op_deps = Query::delete()
                .from_table(AideonOpDeps::Table)
                .and_where(
                    Expr::col(AideonOpDeps::PartitionId).eq(id_value(self.backend, partition.0)),
                )
                .and_where(
                    Expr::col(AideonOpDeps::OpId)
                        .in_subquery(op_subquery.clone())
                        .or(Expr::col(AideonOpDeps::DepOpId).in_subquery(op_subquery)),
                )
                .to_owned();
            exec(tx, &delete_op_deps).await?;

            let delete_ops = Query::delete()
                .from_table(AideonOps::Table)
                .and_where(
                    Expr::col(AideonOps::PartitionId).eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonOps::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_ops).await?;

            let delete_change_feed = Query::delete()
                .from_table(AideonChangeFeed::Table)
                .and_where(
                    Expr::col(AideonChangeFeed::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonChangeFeed::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_change_feed).await?;
        }

        if let Some(days) = policy.keep_facts_days {
            let cutoff = retention_cutoff_hlc(days);
            let delete_edge_exists = Query::delete()
                .from_table(AideonEdgeExistsFacts::Table)
                .and_where(
                    Expr::col(AideonEdgeExistsFacts::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonEdgeExistsFacts::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_edge_exists).await?;

            let delete_fact_str = Query::delete()
                .from_table(AideonPropFactStr::Table)
                .and_where(
                    Expr::col(AideonPropFactStr::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPropFactStr::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_fact_str).await?;

            let delete_fact_i64 = Query::delete()
                .from_table(AideonPropFactI64::Table)
                .and_where(
                    Expr::col(AideonPropFactI64::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPropFactI64::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_fact_i64).await?;

            let delete_fact_f64 = Query::delete()
                .from_table(AideonPropFactF64::Table)
                .and_where(
                    Expr::col(AideonPropFactF64::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPropFactF64::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_fact_f64).await?;

            let delete_fact_bool = Query::delete()
                .from_table(AideonPropFactBool::Table)
                .and_where(
                    Expr::col(AideonPropFactBool::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPropFactBool::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_fact_bool).await?;

            let delete_fact_time = Query::delete()
                .from_table(AideonPropFactTime::Table)
                .and_where(
                    Expr::col(AideonPropFactTime::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPropFactTime::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_fact_time).await?;

            let delete_fact_ref = Query::delete()
                .from_table(AideonPropFactRef::Table)
                .and_where(
                    Expr::col(AideonPropFactRef::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPropFactRef::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_fact_ref).await?;

            let delete_fact_blob = Query::delete()
                .from_table(AideonPropFactBlob::Table)
                .and_where(
                    Expr::col(AideonPropFactBlob::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPropFactBlob::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_fact_blob).await?;

            let delete_fact_json = Query::delete()
                .from_table(AideonPropFactJson::Table)
                .and_where(
                    Expr::col(AideonPropFactJson::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPropFactJson::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_fact_json).await?;

            let delete_idx_str = Query::delete()
                .from_table(AideonIdxFieldStr::Table)
                .and_where(
                    Expr::col(AideonIdxFieldStr::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonIdxFieldStr::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_idx_str).await?;

            let delete_idx_i64 = Query::delete()
                .from_table(AideonIdxFieldI64::Table)
                .and_where(
                    Expr::col(AideonIdxFieldI64::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonIdxFieldI64::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_idx_i64).await?;

            let delete_idx_f64 = Query::delete()
                .from_table(AideonIdxFieldF64::Table)
                .and_where(
                    Expr::col(AideonIdxFieldF64::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonIdxFieldF64::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_idx_f64).await?;

            let delete_idx_bool = Query::delete()
                .from_table(AideonIdxFieldBool::Table)
                .and_where(
                    Expr::col(AideonIdxFieldBool::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonIdxFieldBool::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_idx_bool).await?;

            let delete_idx_time = Query::delete()
                .from_table(AideonIdxFieldTime::Table)
                .and_where(
                    Expr::col(AideonIdxFieldTime::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonIdxFieldTime::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_idx_time).await?;

            let delete_idx_ref = Query::delete()
                .from_table(AideonIdxFieldRef::Table)
                .and_where(
                    Expr::col(AideonIdxFieldRef::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonIdxFieldRef::AssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_idx_ref).await?;

            let delete_schema_cache = Query::delete()
                .from_table(AideonEffectiveSchemaCache::Table)
                .and_where(
                    Expr::col(AideonEffectiveSchemaCache::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonEffectiveSchemaCache::BuiltAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_schema_cache).await?;

            let integrity_runs = Query::select()
                .from(AideonIntegrityRuns::Table)
                .column(AideonIntegrityRuns::RunId)
                .and_where(
                    Expr::col(AideonIntegrityRuns::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonIntegrityRuns::CreatedAssertedAtHlc).lt(cutoff))
                .to_owned();
            let delete_findings = Query::delete()
                .from_table(AideonIntegrityFindings::Table)
                .and_where(
                    Expr::col(AideonIntegrityFindings::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(
                    Expr::col(AideonIntegrityFindings::RunId).in_subquery(integrity_runs.clone()),
                )
                .to_owned();
            exec(tx, &delete_findings).await?;

            let delete_runs = Query::delete()
                .from_table(AideonIntegrityRuns::Table)
                .and_where(
                    Expr::col(AideonIntegrityRuns::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonIntegrityRuns::CreatedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_runs).await?;

            let delete_degree_stats = Query::delete()
                .from_table(AideonGraphDegreeStats::Table)
                .and_where(
                    Expr::col(AideonGraphDegreeStats::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonGraphDegreeStats::ComputedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_degree_stats).await?;

            let delete_edge_counts = Query::delete()
                .from_table(AideonGraphEdgeTypeCounts::Table)
                .and_where(
                    Expr::col(AideonGraphEdgeTypeCounts::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonGraphEdgeTypeCounts::ComputedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_edge_counts).await?;

            let delete_cache_str = Query::delete()
                .from_table(AideonComputedCacheStr::Table)
                .and_where(
                    Expr::col(AideonComputedCacheStr::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonComputedCacheStr::ComputedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_cache_str).await?;

            let delete_cache_i64 = Query::delete()
                .from_table(AideonComputedCacheI64::Table)
                .and_where(
                    Expr::col(AideonComputedCacheI64::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonComputedCacheI64::ComputedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_cache_i64).await?;

            let delete_cache_f64 = Query::delete()
                .from_table(AideonComputedCacheF64::Table)
                .and_where(
                    Expr::col(AideonComputedCacheF64::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonComputedCacheF64::ComputedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_cache_f64).await?;

            let delete_cache_bool = Query::delete()
                .from_table(AideonComputedCacheBool::Table)
                .and_where(
                    Expr::col(AideonComputedCacheBool::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonComputedCacheBool::ComputedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_cache_bool).await?;

            let delete_cache_time = Query::delete()
                .from_table(AideonComputedCacheTime::Table)
                .and_where(
                    Expr::col(AideonComputedCacheTime::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonComputedCacheTime::ComputedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_cache_time).await?;

            let delete_cache_ref = Query::delete()
                .from_table(AideonComputedCacheRef::Table)
                .and_where(
                    Expr::col(AideonComputedCacheRef::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonComputedCacheRef::ComputedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_cache_ref).await?;

            let delete_cache_blob = Query::delete()
                .from_table(AideonComputedCacheBlob::Table)
                .and_where(
                    Expr::col(AideonComputedCacheBlob::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonComputedCacheBlob::ComputedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_cache_blob).await?;

            let delete_cache_json = Query::delete()
                .from_table(AideonComputedCacheJson::Table)
                .and_where(
                    Expr::col(AideonComputedCacheJson::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonComputedCacheJson::ComputedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_cache_json).await?;
        }

        if let Some(days) = policy.keep_failed_jobs_days {
            let cutoff = retention_cutoff_hlc(days);
            let failed_jobs = Query::select()
                .from(AideonJobs::Table)
                .column(AideonJobs::JobId)
                .and_where(
                    Expr::col(AideonJobs::PartitionId).eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonJobs::Status).eq(JOB_STATUS_FAILED as i64))
                .and_where(Expr::col(AideonJobs::UpdatedAssertedAtHlc).lt(cutoff))
                .to_owned();
            let delete_events = Query::delete()
                .from_table(AideonJobEvents::Table)
                .and_where(
                    Expr::col(AideonJobEvents::PartitionId).eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonJobEvents::JobId).in_subquery(failed_jobs.clone()))
                .to_owned();
            exec(tx, &delete_events).await?;

            let delete_jobs = Query::delete()
                .from_table(AideonJobs::Table)
                .and_where(
                    Expr::col(AideonJobs::PartitionId).eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonJobs::Status).eq(JOB_STATUS_FAILED as i64))
                .and_where(Expr::col(AideonJobs::UpdatedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_jobs).await?;
        }

        if let Some(days) = policy.keep_pagerank_runs_days {
            let cutoff = retention_cutoff_hlc(days);
            let old_runs = Query::select()
                .from(AideonPagerankRuns::Table)
                .column(AideonPagerankRuns::RunId)
                .and_where(
                    Expr::col(AideonPagerankRuns::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPagerankRuns::CreatedAssertedAtHlc).lt(cutoff))
                .to_owned();
            let delete_scores = Query::delete()
                .from_table(AideonPagerankScores::Table)
                .and_where(
                    Expr::col(AideonPagerankScores::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPagerankScores::RunId).in_subquery(old_runs.clone()))
                .to_owned();
            exec(tx, &delete_scores).await?;

            let delete_runs = Query::delete()
                .from_table(AideonPagerankRuns::Table)
                .and_where(
                    Expr::col(AideonPagerankRuns::PartitionId)
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col(AideonPagerankRuns::CreatedAssertedAtHlc).lt(cutoff))
                .to_owned();
            exec(tx, &delete_runs).await?;
        }

        Ok(())
    }

    async fn rebuild_index_tables(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
    ) -> MnemeResult<()> {
        self.rebuild_index_str(tx, partition).await?;
        self.rebuild_index_i64(tx, partition).await?;
        self.rebuild_index_f64(tx, partition).await?;
        self.rebuild_index_bool(tx, partition).await?;
        self.rebuild_index_time(tx, partition).await?;
        self.rebuild_index_ref(tx, partition).await?;
        Ok(())
    }

    async fn compact_partition(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
    ) -> MnemeResult<()> {
        let lww_fields = self.list_lww_fields(tx, partition).await?;
        if !lww_fields.str_fields.is_empty() {
            compact_prop_fact_table(
                tx,
                PropFactCompactionSpec {
                    table_info: PropFactTableInfo {
                        backend: self.backend,
                        partition,
                        table: AideonPropFactStr::Table,
                        scenario_col: &col_name(AideonPropFactStr::ScenarioId),
                        entity_col: &col_name(AideonPropFactStr::EntityId),
                        field_col: &col_name(AideonPropFactStr::FieldId),
                        valid_from_col: &col_name(AideonPropFactStr::ValidFrom),
                        valid_to_col: &col_name(AideonPropFactStr::ValidTo),
                        asserted_col: &col_name(AideonPropFactStr::AssertedAtHlc),
                        op_id_col: &col_name(AideonPropFactStr::OpId),
                    },
                    field_ids: &lww_fields.str_fields,
                    layer_col: &col_name(AideonPropFactStr::Layer),
                    is_tombstone_col: &col_name(AideonPropFactStr::IsTombstone),
                    value_col: &col_name(AideonPropFactStr::ValueText),
                    value_type: ValueType::Str,
                },
            )
            .await?;
        }
        if !lww_fields.i64_fields.is_empty() {
            compact_prop_fact_table(
                tx,
                PropFactCompactionSpec {
                    table_info: PropFactTableInfo {
                        backend: self.backend,
                        partition,
                        table: AideonPropFactI64::Table,
                        scenario_col: &col_name(AideonPropFactI64::ScenarioId),
                        entity_col: &col_name(AideonPropFactI64::EntityId),
                        field_col: &col_name(AideonPropFactI64::FieldId),
                        valid_from_col: &col_name(AideonPropFactI64::ValidFrom),
                        valid_to_col: &col_name(AideonPropFactI64::ValidTo),
                        asserted_col: &col_name(AideonPropFactI64::AssertedAtHlc),
                        op_id_col: &col_name(AideonPropFactI64::OpId),
                    },
                    field_ids: &lww_fields.i64_fields,
                    layer_col: &col_name(AideonPropFactI64::Layer),
                    is_tombstone_col: &col_name(AideonPropFactI64::IsTombstone),
                    value_col: &col_name(AideonPropFactI64::ValueI64),
                    value_type: ValueType::I64,
                },
            )
            .await?;
        }
        if !lww_fields.f64_fields.is_empty() {
            compact_prop_fact_table(
                tx,
                PropFactCompactionSpec {
                    table_info: PropFactTableInfo {
                        backend: self.backend,
                        partition,
                        table: AideonPropFactF64::Table,
                        scenario_col: &col_name(AideonPropFactF64::ScenarioId),
                        entity_col: &col_name(AideonPropFactF64::EntityId),
                        field_col: &col_name(AideonPropFactF64::FieldId),
                        valid_from_col: &col_name(AideonPropFactF64::ValidFrom),
                        valid_to_col: &col_name(AideonPropFactF64::ValidTo),
                        asserted_col: &col_name(AideonPropFactF64::AssertedAtHlc),
                        op_id_col: &col_name(AideonPropFactF64::OpId),
                    },
                    field_ids: &lww_fields.f64_fields,
                    layer_col: &col_name(AideonPropFactF64::Layer),
                    is_tombstone_col: &col_name(AideonPropFactF64::IsTombstone),
                    value_col: &col_name(AideonPropFactF64::ValueF64),
                    value_type: ValueType::F64,
                },
            )
            .await?;
        }
        if !lww_fields.bool_fields.is_empty() {
            compact_prop_fact_table(
                tx,
                PropFactCompactionSpec {
                    table_info: PropFactTableInfo {
                        backend: self.backend,
                        partition,
                        table: AideonPropFactBool::Table,
                        scenario_col: &col_name(AideonPropFactBool::ScenarioId),
                        entity_col: &col_name(AideonPropFactBool::EntityId),
                        field_col: &col_name(AideonPropFactBool::FieldId),
                        valid_from_col: &col_name(AideonPropFactBool::ValidFrom),
                        valid_to_col: &col_name(AideonPropFactBool::ValidTo),
                        asserted_col: &col_name(AideonPropFactBool::AssertedAtHlc),
                        op_id_col: &col_name(AideonPropFactBool::OpId),
                    },
                    field_ids: &lww_fields.bool_fields,
                    layer_col: &col_name(AideonPropFactBool::Layer),
                    is_tombstone_col: &col_name(AideonPropFactBool::IsTombstone),
                    value_col: &col_name(AideonPropFactBool::ValueBool),
                    value_type: ValueType::Bool,
                },
            )
            .await?;
        }
        if !lww_fields.time_fields.is_empty() {
            compact_prop_fact_table(
                tx,
                PropFactCompactionSpec {
                    table_info: PropFactTableInfo {
                        backend: self.backend,
                        partition,
                        table: AideonPropFactTime::Table,
                        scenario_col: &col_name(AideonPropFactTime::ScenarioId),
                        entity_col: &col_name(AideonPropFactTime::EntityId),
                        field_col: &col_name(AideonPropFactTime::FieldId),
                        valid_from_col: &col_name(AideonPropFactTime::ValidFrom),
                        valid_to_col: &col_name(AideonPropFactTime::ValidTo),
                        asserted_col: &col_name(AideonPropFactTime::AssertedAtHlc),
                        op_id_col: &col_name(AideonPropFactTime::OpId),
                    },
                    field_ids: &lww_fields.time_fields,
                    layer_col: &col_name(AideonPropFactTime::Layer),
                    is_tombstone_col: &col_name(AideonPropFactTime::IsTombstone),
                    value_col: &col_name(AideonPropFactTime::ValueTime),
                    value_type: ValueType::Time,
                },
            )
            .await?;
        }
        if !lww_fields.ref_fields.is_empty() {
            compact_prop_fact_table(
                tx,
                PropFactCompactionSpec {
                    table_info: PropFactTableInfo {
                        backend: self.backend,
                        partition,
                        table: AideonPropFactRef::Table,
                        scenario_col: &col_name(AideonPropFactRef::ScenarioId),
                        entity_col: &col_name(AideonPropFactRef::EntityId),
                        field_col: &col_name(AideonPropFactRef::FieldId),
                        valid_from_col: &col_name(AideonPropFactRef::ValidFrom),
                        valid_to_col: &col_name(AideonPropFactRef::ValidTo),
                        asserted_col: &col_name(AideonPropFactRef::AssertedAtHlc),
                        op_id_col: &col_name(AideonPropFactRef::OpId),
                    },
                    field_ids: &lww_fields.ref_fields,
                    layer_col: &col_name(AideonPropFactRef::Layer),
                    is_tombstone_col: &col_name(AideonPropFactRef::IsTombstone),
                    value_col: &col_name(AideonPropFactRef::ValueRefEntityId),
                    value_type: ValueType::Ref,
                },
            )
            .await?;
        }
        if !lww_fields.blob_fields.is_empty() {
            compact_prop_fact_table(
                tx,
                PropFactCompactionSpec {
                    table_info: PropFactTableInfo {
                        backend: self.backend,
                        partition,
                        table: AideonPropFactBlob::Table,
                        scenario_col: &col_name(AideonPropFactBlob::ScenarioId),
                        entity_col: &col_name(AideonPropFactBlob::EntityId),
                        field_col: &col_name(AideonPropFactBlob::FieldId),
                        valid_from_col: &col_name(AideonPropFactBlob::ValidFrom),
                        valid_to_col: &col_name(AideonPropFactBlob::ValidTo),
                        asserted_col: &col_name(AideonPropFactBlob::AssertedAtHlc),
                        op_id_col: &col_name(AideonPropFactBlob::OpId),
                    },
                    field_ids: &lww_fields.blob_fields,
                    layer_col: &col_name(AideonPropFactBlob::Layer),
                    is_tombstone_col: &col_name(AideonPropFactBlob::IsTombstone),
                    value_col: &col_name(AideonPropFactBlob::ValueBlob),
                    value_type: ValueType::Blob,
                },
            )
            .await?;
        }
        if !lww_fields.json_fields.is_empty() {
            compact_prop_fact_table(
                tx,
                PropFactCompactionSpec {
                    table_info: PropFactTableInfo {
                        backend: self.backend,
                        partition,
                        table: AideonPropFactJson::Table,
                        scenario_col: &col_name(AideonPropFactJson::ScenarioId),
                        entity_col: &col_name(AideonPropFactJson::EntityId),
                        field_col: &col_name(AideonPropFactJson::FieldId),
                        valid_from_col: &col_name(AideonPropFactJson::ValidFrom),
                        valid_to_col: &col_name(AideonPropFactJson::ValidTo),
                        asserted_col: &col_name(AideonPropFactJson::AssertedAtHlc),
                        op_id_col: &col_name(AideonPropFactJson::OpId),
                    },
                    field_ids: &lww_fields.json_fields,
                    layer_col: &col_name(AideonPropFactJson::Layer),
                    is_tombstone_col: &col_name(AideonPropFactJson::IsTombstone),
                    value_col: &col_name(AideonPropFactJson::ValueJson),
                    value_type: ValueType::Json,
                },
            )
            .await?;
        }
        compact_edge_exists_facts(tx, self.backend, partition).await?;
        self.rebuild_index_tables(tx, partition).await?;
        Ok(())
    }

    async fn list_lww_fields(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
    ) -> MnemeResult<LwwFields> {
        let select = Query::select()
            .from(AideonFields::Table)
            .columns([AideonFields::FieldId, AideonFields::ValueType])
            .and_where(Expr::col(AideonFields::PartitionId).eq(id_value(self.backend, partition.0)))
            .and_where(
                Expr::col(AideonFields::MergePolicy)
                    .is_in([MergePolicy::Lww.as_i16(), MergePolicy::Text.as_i16()]),
            )
            .to_owned();
        let rows = query_all(tx, &select).await?;
        let mut by_type = LwwFields::default();
        for row in rows {
            let field_id = read_id(&row, AideonFields::FieldId)?;
            let value_type_raw: i64 = row.try_get("", &col_name(AideonFields::ValueType))?;
            let value_type = ValueType::from_i16(value_type_raw as i16)
                .ok_or_else(|| MnemeError::storage("invalid value type".to_string()))?;
            by_type.push(value_type, field_id);
        }
        Ok(by_type)
    }

    async fn rebuild_index_str(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
    ) -> MnemeResult<()> {
        let delete = Query::delete()
            .from_table(AideonIdxFieldStr::Table)
            .and_where(
                Expr::col(AideonIdxFieldStr::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        exec(tx, &delete).await?;

        let mut offset = 0u64;
        loop {
            let select = Query::select()
                .from(AideonPropFactStr::Table)
                .inner_join(
                    AideonFields::Table,
                    Expr::col((AideonPropFactStr::Table, AideonPropFactStr::PartitionId))
                        .equals((AideonFields::Table, AideonFields::PartitionId))
                        .and(
                            Expr::col((AideonPropFactStr::Table, AideonPropFactStr::FieldId))
                                .equals((AideonFields::Table, AideonFields::FieldId)),
                        ),
                )
                .columns([
                    (AideonPropFactStr::Table, AideonPropFactStr::ScenarioId),
                    (AideonPropFactStr::Table, AideonPropFactStr::FieldId),
                    (AideonPropFactStr::Table, AideonPropFactStr::EntityId),
                    (AideonPropFactStr::Table, AideonPropFactStr::ValidFrom),
                    (AideonPropFactStr::Table, AideonPropFactStr::ValidTo),
                    (AideonPropFactStr::Table, AideonPropFactStr::ValidBucket),
                    (AideonPropFactStr::Table, AideonPropFactStr::AssertedAtHlc),
                    (AideonPropFactStr::Table, AideonPropFactStr::Layer),
                    (AideonPropFactStr::Table, AideonPropFactStr::ValueText),
                ])
                .and_where(
                    Expr::col((AideonPropFactStr::Table, AideonPropFactStr::PartitionId))
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col((AideonFields::Table, AideonFields::IsIndexed)).eq(true))
                .order_by(
                    (AideonPropFactStr::Table, AideonPropFactStr::AssertedAtHlc),
                    Order::Asc,
                )
                .limit(INDEX_REBUILD_BATCH)
                .offset(offset)
                .to_owned();

            let rows = query_all(tx, &select).await?;
            if rows.is_empty() {
                break;
            }

            let mut insert = Query::insert()
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
                .to_owned();

            for row in rows {
                let scenario_id =
                    read_opt_id_by_name(&row, &col_name(AideonPropFactStr::ScenarioId))?;
                let field_id = read_id_by_name(&row, &col_name(AideonPropFactStr::FieldId))?;
                let entity_id = read_id_by_name(&row, &col_name(AideonPropFactStr::EntityId))?;
                let valid_from: i64 = row.try_get("", &col_name(AideonPropFactStr::ValidFrom))?;
                let valid_to: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactStr::ValidTo))?;
                let valid_bucket: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactStr::ValidBucket))?;
                let asserted_at: i64 =
                    row.try_get("", &col_name(AideonPropFactStr::AssertedAtHlc))?;
                let layer: i64 = row.try_get("", &col_name(AideonPropFactStr::Layer))?;
                let value: String = row.try_get("", &col_name(AideonPropFactStr::ValueText))?;
                let value_norm = normalize_index_text(&value);
                insert.values_panic([
                    id_value(self.backend, partition.0).into(),
                    opt_id_value(self.backend, scenario_id).into(),
                    id_value(self.backend, field_id).into(),
                    value_norm.into(),
                    id_value(self.backend, entity_id).into(),
                    valid_from.into(),
                    valid_to.into(),
                    valid_bucket.into(),
                    asserted_at.into(),
                    layer.into(),
                ]);
            }
            exec(tx, &insert).await?;
            offset += INDEX_REBUILD_BATCH;
        }

        Ok(())
    }

    async fn rebuild_index_i64(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
    ) -> MnemeResult<()> {
        let delete = Query::delete()
            .from_table(AideonIdxFieldI64::Table)
            .and_where(
                Expr::col(AideonIdxFieldI64::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        exec(tx, &delete).await?;

        let mut offset = 0u64;
        loop {
            let select = Query::select()
                .from(AideonPropFactI64::Table)
                .inner_join(
                    AideonFields::Table,
                    Expr::col((AideonPropFactI64::Table, AideonPropFactI64::PartitionId))
                        .equals((AideonFields::Table, AideonFields::PartitionId))
                        .and(
                            Expr::col((AideonPropFactI64::Table, AideonPropFactI64::FieldId))
                                .equals((AideonFields::Table, AideonFields::FieldId)),
                        ),
                )
                .columns([
                    (AideonPropFactI64::Table, AideonPropFactI64::ScenarioId),
                    (AideonPropFactI64::Table, AideonPropFactI64::FieldId),
                    (AideonPropFactI64::Table, AideonPropFactI64::EntityId),
                    (AideonPropFactI64::Table, AideonPropFactI64::ValidFrom),
                    (AideonPropFactI64::Table, AideonPropFactI64::ValidTo),
                    (AideonPropFactI64::Table, AideonPropFactI64::ValidBucket),
                    (AideonPropFactI64::Table, AideonPropFactI64::AssertedAtHlc),
                    (AideonPropFactI64::Table, AideonPropFactI64::Layer),
                    (AideonPropFactI64::Table, AideonPropFactI64::ValueI64),
                ])
                .and_where(
                    Expr::col((AideonPropFactI64::Table, AideonPropFactI64::PartitionId))
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col((AideonFields::Table, AideonFields::IsIndexed)).eq(true))
                .order_by(
                    (AideonPropFactI64::Table, AideonPropFactI64::AssertedAtHlc),
                    Order::Asc,
                )
                .limit(INDEX_REBUILD_BATCH)
                .offset(offset)
                .to_owned();

            let rows = query_all(tx, &select).await?;
            if rows.is_empty() {
                break;
            }

            let mut insert = Query::insert()
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
                .to_owned();

            for row in rows {
                let scenario_id =
                    read_opt_id_by_name(&row, &col_name(AideonPropFactI64::ScenarioId))?;
                let field_id = read_id_by_name(&row, &col_name(AideonPropFactI64::FieldId))?;
                let entity_id = read_id_by_name(&row, &col_name(AideonPropFactI64::EntityId))?;
                let valid_from: i64 = row.try_get("", &col_name(AideonPropFactI64::ValidFrom))?;
                let valid_to: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactI64::ValidTo))?;
                let valid_bucket: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactI64::ValidBucket))?;
                let asserted_at: i64 =
                    row.try_get("", &col_name(AideonPropFactI64::AssertedAtHlc))?;
                let layer: i64 = row.try_get("", &col_name(AideonPropFactI64::Layer))?;
                let value: i64 = row.try_get("", &col_name(AideonPropFactI64::ValueI64))?;
                insert.values_panic([
                    id_value(self.backend, partition.0).into(),
                    opt_id_value(self.backend, scenario_id).into(),
                    id_value(self.backend, field_id).into(),
                    value.into(),
                    id_value(self.backend, entity_id).into(),
                    valid_from.into(),
                    valid_to.into(),
                    valid_bucket.into(),
                    asserted_at.into(),
                    layer.into(),
                ]);
            }
            exec(tx, &insert).await?;
            offset += INDEX_REBUILD_BATCH;
        }

        Ok(())
    }

    async fn rebuild_index_f64(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
    ) -> MnemeResult<()> {
        let delete = Query::delete()
            .from_table(AideonIdxFieldF64::Table)
            .and_where(
                Expr::col(AideonIdxFieldF64::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        exec(tx, &delete).await?;

        let mut offset = 0u64;
        loop {
            let select = Query::select()
                .from(AideonPropFactF64::Table)
                .inner_join(
                    AideonFields::Table,
                    Expr::col((AideonPropFactF64::Table, AideonPropFactF64::PartitionId))
                        .equals((AideonFields::Table, AideonFields::PartitionId))
                        .and(
                            Expr::col((AideonPropFactF64::Table, AideonPropFactF64::FieldId))
                                .equals((AideonFields::Table, AideonFields::FieldId)),
                        ),
                )
                .columns([
                    (AideonPropFactF64::Table, AideonPropFactF64::ScenarioId),
                    (AideonPropFactF64::Table, AideonPropFactF64::FieldId),
                    (AideonPropFactF64::Table, AideonPropFactF64::EntityId),
                    (AideonPropFactF64::Table, AideonPropFactF64::ValidFrom),
                    (AideonPropFactF64::Table, AideonPropFactF64::ValidTo),
                    (AideonPropFactF64::Table, AideonPropFactF64::ValidBucket),
                    (AideonPropFactF64::Table, AideonPropFactF64::AssertedAtHlc),
                    (AideonPropFactF64::Table, AideonPropFactF64::Layer),
                    (AideonPropFactF64::Table, AideonPropFactF64::ValueF64),
                ])
                .and_where(
                    Expr::col((AideonPropFactF64::Table, AideonPropFactF64::PartitionId))
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col((AideonFields::Table, AideonFields::IsIndexed)).eq(true))
                .order_by(
                    (AideonPropFactF64::Table, AideonPropFactF64::AssertedAtHlc),
                    Order::Asc,
                )
                .limit(INDEX_REBUILD_BATCH)
                .offset(offset)
                .to_owned();

            let rows = query_all(tx, &select).await?;
            if rows.is_empty() {
                break;
            }

            let mut insert = Query::insert()
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
                .to_owned();

            for row in rows {
                let scenario_id =
                    read_opt_id_by_name(&row, &col_name(AideonPropFactF64::ScenarioId))?;
                let field_id = read_id_by_name(&row, &col_name(AideonPropFactF64::FieldId))?;
                let entity_id = read_id_by_name(&row, &col_name(AideonPropFactF64::EntityId))?;
                let valid_from: i64 = row.try_get("", &col_name(AideonPropFactF64::ValidFrom))?;
                let valid_to: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactF64::ValidTo))?;
                let valid_bucket: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactF64::ValidBucket))?;
                let asserted_at: i64 =
                    row.try_get("", &col_name(AideonPropFactF64::AssertedAtHlc))?;
                let layer: i64 = row.try_get("", &col_name(AideonPropFactF64::Layer))?;
                let value: f64 = row.try_get("", &col_name(AideonPropFactF64::ValueF64))?;
                insert.values_panic([
                    id_value(self.backend, partition.0).into(),
                    opt_id_value(self.backend, scenario_id).into(),
                    id_value(self.backend, field_id).into(),
                    value.into(),
                    id_value(self.backend, entity_id).into(),
                    valid_from.into(),
                    valid_to.into(),
                    valid_bucket.into(),
                    asserted_at.into(),
                    layer.into(),
                ]);
            }
            exec(tx, &insert).await?;
            offset += INDEX_REBUILD_BATCH;
        }

        Ok(())
    }

    async fn rebuild_index_bool(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
    ) -> MnemeResult<()> {
        let delete = Query::delete()
            .from_table(AideonIdxFieldBool::Table)
            .and_where(
                Expr::col(AideonIdxFieldBool::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        exec(tx, &delete).await?;

        let mut offset = 0u64;
        loop {
            let select = Query::select()
                .from(AideonPropFactBool::Table)
                .inner_join(
                    AideonFields::Table,
                    Expr::col((AideonPropFactBool::Table, AideonPropFactBool::PartitionId))
                        .equals((AideonFields::Table, AideonFields::PartitionId))
                        .and(
                            Expr::col((AideonPropFactBool::Table, AideonPropFactBool::FieldId))
                                .equals((AideonFields::Table, AideonFields::FieldId)),
                        ),
                )
                .columns([
                    (AideonPropFactBool::Table, AideonPropFactBool::ScenarioId),
                    (AideonPropFactBool::Table, AideonPropFactBool::FieldId),
                    (AideonPropFactBool::Table, AideonPropFactBool::EntityId),
                    (AideonPropFactBool::Table, AideonPropFactBool::ValidFrom),
                    (AideonPropFactBool::Table, AideonPropFactBool::ValidTo),
                    (AideonPropFactBool::Table, AideonPropFactBool::ValidBucket),
                    (AideonPropFactBool::Table, AideonPropFactBool::AssertedAtHlc),
                    (AideonPropFactBool::Table, AideonPropFactBool::Layer),
                    (AideonPropFactBool::Table, AideonPropFactBool::ValueBool),
                ])
                .and_where(
                    Expr::col((AideonPropFactBool::Table, AideonPropFactBool::PartitionId))
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col((AideonFields::Table, AideonFields::IsIndexed)).eq(true))
                .order_by(
                    (AideonPropFactBool::Table, AideonPropFactBool::AssertedAtHlc),
                    Order::Asc,
                )
                .limit(INDEX_REBUILD_BATCH)
                .offset(offset)
                .to_owned();

            let rows = query_all(tx, &select).await?;
            if rows.is_empty() {
                break;
            }

            let mut insert = Query::insert()
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
                .to_owned();

            for row in rows {
                let scenario_id =
                    read_opt_id_by_name(&row, &col_name(AideonPropFactBool::ScenarioId))?;
                let field_id = read_id_by_name(&row, &col_name(AideonPropFactBool::FieldId))?;
                let entity_id = read_id_by_name(&row, &col_name(AideonPropFactBool::EntityId))?;
                let valid_from: i64 = row.try_get("", &col_name(AideonPropFactBool::ValidFrom))?;
                let valid_to: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactBool::ValidTo))?;
                let valid_bucket: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactBool::ValidBucket))?;
                let asserted_at: i64 =
                    row.try_get("", &col_name(AideonPropFactBool::AssertedAtHlc))?;
                let layer: i64 = row.try_get("", &col_name(AideonPropFactBool::Layer))?;
                let value: bool = row.try_get("", &col_name(AideonPropFactBool::ValueBool))?;
                insert.values_panic([
                    id_value(self.backend, partition.0).into(),
                    opt_id_value(self.backend, scenario_id).into(),
                    id_value(self.backend, field_id).into(),
                    value.into(),
                    id_value(self.backend, entity_id).into(),
                    valid_from.into(),
                    valid_to.into(),
                    valid_bucket.into(),
                    asserted_at.into(),
                    layer.into(),
                ]);
            }
            exec(tx, &insert).await?;
            offset += INDEX_REBUILD_BATCH;
        }

        Ok(())
    }

    async fn rebuild_index_time(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
    ) -> MnemeResult<()> {
        let delete = Query::delete()
            .from_table(AideonIdxFieldTime::Table)
            .and_where(
                Expr::col(AideonIdxFieldTime::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        exec(tx, &delete).await?;

        let mut offset = 0u64;
        loop {
            let select = Query::select()
                .from(AideonPropFactTime::Table)
                .inner_join(
                    AideonFields::Table,
                    Expr::col((AideonPropFactTime::Table, AideonPropFactTime::PartitionId))
                        .equals((AideonFields::Table, AideonFields::PartitionId))
                        .and(
                            Expr::col((AideonPropFactTime::Table, AideonPropFactTime::FieldId))
                                .equals((AideonFields::Table, AideonFields::FieldId)),
                        ),
                )
                .columns([
                    (AideonPropFactTime::Table, AideonPropFactTime::ScenarioId),
                    (AideonPropFactTime::Table, AideonPropFactTime::FieldId),
                    (AideonPropFactTime::Table, AideonPropFactTime::EntityId),
                    (AideonPropFactTime::Table, AideonPropFactTime::ValidFrom),
                    (AideonPropFactTime::Table, AideonPropFactTime::ValidTo),
                    (AideonPropFactTime::Table, AideonPropFactTime::ValidBucket),
                    (AideonPropFactTime::Table, AideonPropFactTime::AssertedAtHlc),
                    (AideonPropFactTime::Table, AideonPropFactTime::Layer),
                    (AideonPropFactTime::Table, AideonPropFactTime::ValueTime),
                ])
                .and_where(
                    Expr::col((AideonPropFactTime::Table, AideonPropFactTime::PartitionId))
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col((AideonFields::Table, AideonFields::IsIndexed)).eq(true))
                .order_by(
                    (AideonPropFactTime::Table, AideonPropFactTime::AssertedAtHlc),
                    Order::Asc,
                )
                .limit(INDEX_REBUILD_BATCH)
                .offset(offset)
                .to_owned();

            let rows = query_all(tx, &select).await?;
            if rows.is_empty() {
                break;
            }

            let mut insert = Query::insert()
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
                .to_owned();

            for row in rows {
                let scenario_id =
                    read_opt_id_by_name(&row, &col_name(AideonPropFactTime::ScenarioId))?;
                let field_id = read_id_by_name(&row, &col_name(AideonPropFactTime::FieldId))?;
                let entity_id = read_id_by_name(&row, &col_name(AideonPropFactTime::EntityId))?;
                let valid_from: i64 = row.try_get("", &col_name(AideonPropFactTime::ValidFrom))?;
                let valid_to: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactTime::ValidTo))?;
                let valid_bucket: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactTime::ValidBucket))?;
                let asserted_at: i64 =
                    row.try_get("", &col_name(AideonPropFactTime::AssertedAtHlc))?;
                let layer: i64 = row.try_get("", &col_name(AideonPropFactTime::Layer))?;
                let value: i64 = row.try_get("", &col_name(AideonPropFactTime::ValueTime))?;
                insert.values_panic([
                    id_value(self.backend, partition.0).into(),
                    opt_id_value(self.backend, scenario_id).into(),
                    id_value(self.backend, field_id).into(),
                    value.into(),
                    id_value(self.backend, entity_id).into(),
                    valid_from.into(),
                    valid_to.into(),
                    valid_bucket.into(),
                    asserted_at.into(),
                    layer.into(),
                ]);
            }
            exec(tx, &insert).await?;
            offset += INDEX_REBUILD_BATCH;
        }

        Ok(())
    }

    async fn rebuild_index_ref(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        partition: PartitionId,
    ) -> MnemeResult<()> {
        let delete = Query::delete()
            .from_table(AideonIdxFieldRef::Table)
            .and_where(
                Expr::col(AideonIdxFieldRef::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        exec(tx, &delete).await?;

        let mut offset = 0u64;
        loop {
            let select = Query::select()
                .from(AideonPropFactRef::Table)
                .inner_join(
                    AideonFields::Table,
                    Expr::col((AideonPropFactRef::Table, AideonPropFactRef::PartitionId))
                        .equals((AideonFields::Table, AideonFields::PartitionId))
                        .and(
                            Expr::col((AideonPropFactRef::Table, AideonPropFactRef::FieldId))
                                .equals((AideonFields::Table, AideonFields::FieldId)),
                        ),
                )
                .columns([
                    (AideonPropFactRef::Table, AideonPropFactRef::ScenarioId),
                    (AideonPropFactRef::Table, AideonPropFactRef::FieldId),
                    (AideonPropFactRef::Table, AideonPropFactRef::EntityId),
                    (AideonPropFactRef::Table, AideonPropFactRef::ValidFrom),
                    (AideonPropFactRef::Table, AideonPropFactRef::ValidTo),
                    (AideonPropFactRef::Table, AideonPropFactRef::ValidBucket),
                    (AideonPropFactRef::Table, AideonPropFactRef::AssertedAtHlc),
                    (AideonPropFactRef::Table, AideonPropFactRef::Layer),
                    (
                        AideonPropFactRef::Table,
                        AideonPropFactRef::ValueRefEntityId,
                    ),
                ])
                .and_where(
                    Expr::col((AideonPropFactRef::Table, AideonPropFactRef::PartitionId))
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(Expr::col((AideonFields::Table, AideonFields::IsIndexed)).eq(true))
                .order_by(
                    (AideonPropFactRef::Table, AideonPropFactRef::AssertedAtHlc),
                    Order::Asc,
                )
                .limit(INDEX_REBUILD_BATCH)
                .offset(offset)
                .to_owned();

            let rows = query_all(tx, &select).await?;
            if rows.is_empty() {
                break;
            }

            let mut insert = Query::insert()
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
                .to_owned();

            for row in rows {
                let scenario_id =
                    read_opt_id_by_name(&row, &col_name(AideonPropFactRef::ScenarioId))?;
                let field_id = read_id_by_name(&row, &col_name(AideonPropFactRef::FieldId))?;
                let entity_id = read_id_by_name(&row, &col_name(AideonPropFactRef::EntityId))?;
                let valid_from: i64 = row.try_get("", &col_name(AideonPropFactRef::ValidFrom))?;
                let valid_to: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactRef::ValidTo))?;
                let valid_bucket: Option<i64> =
                    row.try_get("", &col_name(AideonPropFactRef::ValidBucket))?;
                let asserted_at: i64 =
                    row.try_get("", &col_name(AideonPropFactRef::AssertedAtHlc))?;
                let layer: i64 = row.try_get("", &col_name(AideonPropFactRef::Layer))?;
                let value = read_id_by_name(&row, &col_name(AideonPropFactRef::ValueRefEntityId))?;
                insert.values_panic([
                    id_value(self.backend, partition.0).into(),
                    opt_id_value(self.backend, scenario_id).into(),
                    id_value(self.backend, field_id).into(),
                    id_value(self.backend, value).into(),
                    id_value(self.backend, entity_id).into(),
                    valid_from.into(),
                    valid_to.into(),
                    valid_bucket.into(),
                    asserted_at.into(),
                    layer.into(),
                ]);
            }
            exec(tx, &insert).await?;
            offset += INDEX_REBUILD_BATCH;
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
        let payload =
            serde_json::to_vec(&payload).map_err(|err| MnemeError::storage(err.to_string()))?;
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
        let payload =
            serde_json::to_vec(&payload).map_err(|err| MnemeError::storage(err.to_string()))?;
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
        let payload =
            serde_json::to_vec(&payload).map_err(|err| MnemeError::storage(err.to_string()))?;
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

    async fn trigger_retention(&self, input: TriggerRetentionInput) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        let payload = RetentionJobPayload {
            partition_id: input.partition,
            scenario_id: input.scenario_id,
            policy: input.policy,
            reason: input.reason,
        };
        let payload =
            serde_json::to_vec(&payload).map_err(|err| MnemeError::storage(err.to_string()))?;
        self.enqueue_job(
            &tx,
            input.partition,
            JOB_TYPE_RETENTION,
            payload,
            -1,
            Some("retention:run".to_string()),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn trigger_compaction(&self, input: TriggerCompactionInput) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        let payload = CompactionJobPayload {
            partition_id: input.partition,
            scenario_id: input.scenario_id,
            reason: input.reason,
        };
        let payload =
            serde_json::to_vec(&payload).map_err(|err| MnemeError::storage(err.to_string()))?;
        self.enqueue_job(
            &tx,
            input.partition,
            JOB_TYPE_COMPACTION,
            payload,
            -1,
            Some("compaction:rebuild_indexes".to_string()),
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
            let tx = self.conn.begin().await?;
            let result = match job.job_type.as_str() {
                JOB_TYPE_SCHEMA_REBUILD => {
                    let payload: TriggerJobPayload = serde_json::from_slice(&job.payload)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
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
                        let _version = self
                            .compile_effective_schema_with_conn(
                                &tx,
                                payload.partition_id,
                                Hlc::now(),
                                type_id,
                            )
                            .await?;
                    }
                    Ok(())
                }
                JOB_TYPE_INTEGRITY_REFRESH => {
                    let payload: TriggerJobPayload = serde_json::from_slice(&job.payload)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    self.refresh_integrity(
                        &tx,
                        payload.partition_id,
                        payload.scenario_id,
                        &payload.reason,
                    )
                    .await
                }
                JOB_TYPE_ANALYTICS_REFRESH => {
                    let payload: TriggerJobPayload = serde_json::from_slice(&job.payload)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    self.refresh_analytics(&tx, payload.partition_id, payload.scenario_id)
                        .await
                }
                JOB_TYPE_RETENTION => {
                    let payload: RetentionJobPayload = serde_json::from_slice(&job.payload)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    self.apply_retention_policy(&tx, payload.partition_id, &payload.policy)
                        .await
                }
                JOB_TYPE_COMPACTION => {
                    let payload: CompactionJobPayload = serde_json::from_slice(&job.payload)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    self.compact_partition(&tx, payload.partition_id).await
                }
                other => Err(MnemeError::processing(format!("unknown job type {other}"))),
            };
            match result {
                Ok(()) => {
                    self.mark_job_succeeded(&tx, job.partition, job.job_id)
                        .await?;
                    tx.commit().await?;
                    processed += 1;
                }
                Err(err) => {
                    self.mark_job_failed(&tx, job.partition, &job, &err.to_string())
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

    async fn subscribe_partition(
        &self,
        partition: PartitionId,
        from_sequence: Option<i64>,
    ) -> MnemeResult<mpsc::Receiver<ChangeEvent>> {
        let (tx, rx) = mpsc::channel(256);
        let mut cursor = from_sequence;
        let store = self.clone();
        tokio::spawn(async move {
            loop {
                if tx.is_closed() {
                    break;
                }
                let changes = store
                    .get_changes_since(partition, cursor, SUBSCRIPTION_POLL_LIMIT)
                    .await
                    .unwrap_or_default();
                if !changes.is_empty() {
                    cursor = Some(changes.last().map(|c| c.sequence).unwrap_or(0));
                    for change in changes {
                        if tx.send(change).await.is_err() {
                            break;
                        }
                    }
                } else {
                    sleep(Duration::from_millis(SUBSCRIPTION_POLL_INTERVAL_MS)).await;
                }
            }
        });
        Ok(rx)
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
                    OnConflict::columns([
                        AideonValidationRules::PartitionId,
                        AideonValidationRules::RuleId,
                    ])
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
impl ComputedRulesApi for MnemeStore {
    async fn upsert_computed_rules(
        &self,
        partition: PartitionId,
        _actor: ActorId,
        asserted_at: Hlc,
        rules: Vec<ComputedRule>,
    ) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        for rule in rules {
            let params = serde_json::to_string(&rule.params)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let insert = Query::insert()
                .into_table(AideonComputedRules::Table)
                .columns([
                    AideonComputedRules::PartitionId,
                    AideonComputedRules::RuleId,
                    AideonComputedRules::TargetTypeId,
                    AideonComputedRules::OutputFieldId,
                    AideonComputedRules::TemplateKind,
                    AideonComputedRules::ParamsJson,
                    AideonComputedRules::UpdatedAssertedAtHlc,
                ])
                .values_panic([
                    id_value(self.backend, partition.0).into(),
                    id_value(self.backend, rule.rule_id).into(),
                    opt_id_value(self.backend, rule.target_type_id).into(),
                    opt_id_value(self.backend, rule.output_field_id).into(),
                    rule.template_kind.into(),
                    params.into(),
                    asserted_at.as_i64().into(),
                ])
                .on_conflict(
                    OnConflict::columns([
                        AideonComputedRules::PartitionId,
                        AideonComputedRules::RuleId,
                    ])
                    .update_columns([
                        AideonComputedRules::TargetTypeId,
                        AideonComputedRules::OutputFieldId,
                        AideonComputedRules::TemplateKind,
                        AideonComputedRules::ParamsJson,
                        AideonComputedRules::UpdatedAssertedAtHlc,
                    ])
                    .to_owned(),
                )
                .to_owned();
            exec(&tx, &insert).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_computed_rules(&self, partition: PartitionId) -> MnemeResult<Vec<ComputedRule>> {
        let select = Query::select()
            .from(AideonComputedRules::Table)
            .columns([
                AideonComputedRules::RuleId,
                AideonComputedRules::TargetTypeId,
                AideonComputedRules::OutputFieldId,
                AideonComputedRules::TemplateKind,
                AideonComputedRules::ParamsJson,
            ])
            .and_where(
                Expr::col(AideonComputedRules::PartitionId).eq(id_value(self.backend, partition.0)),
            )
            .to_owned();
        let rows = query_all(&self.conn, &select).await?;
        let mut rules = Vec::new();
        for row in rows {
            let rule_id = read_id(&row, AideonComputedRules::RuleId)?;
            let target_type_id = read_opt_id(&row, AideonComputedRules::TargetTypeId)?;
            let output_field_id = read_opt_id(&row, AideonComputedRules::OutputFieldId)?;
            let template_kind: String =
                row.try_get("", &col_name(AideonComputedRules::TemplateKind))?;
            let params_json: String =
                row.try_get("", &col_name(AideonComputedRules::ParamsJson))?;
            let params = serde_json::from_str(&params_json)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            rules.push(ComputedRule {
                rule_id,
                target_type_id,
                output_field_id,
                template_kind,
                params,
            });
        }
        Ok(rules)
    }
}

#[async_trait]
impl ComputedCacheApi for MnemeStore {
    async fn upsert_computed_cache(
        &self,
        partition: PartitionId,
        entries: Vec<ComputedCacheEntry>,
    ) -> MnemeResult<()> {
        let tx = self.conn.begin().await?;
        for entry in entries {
            let constraints = self.fetch_field_def(&tx, partition, entry.field_id).await?;
            if entry.value.value_type() != constraints.value_type {
                return Err(MnemeError::invalid("computed cache value type mismatch"));
            }
            match entry.value {
                Value::Str(value) => {
                    let spec = ComputedCacheTableSpec {
                        table: AideonComputedCacheStr::Table,
                        partition_col: "partition_id",
                        entity_col: "entity_id",
                        field_col: "field_id",
                        valid_from_col: "valid_from",
                        valid_to_col: "valid_to",
                        rule_hash_col: "rule_version_hash",
                        computed_at_col: "computed_asserted_at_hlc",
                        value_col: &col_name(AideonComputedCacheStr::ValueText),
                        value_type: ValueType::Str,
                    };
                    upsert_computed_cache_row(
                        &tx,
                        self.backend,
                        ComputedCacheRowInput {
                            partition,
                            entity_id: entry.entity_id,
                            field_id: entry.field_id,
                            valid_from: entry.valid_from,
                            valid_to: entry.valid_to,
                            rule_version_hash: &entry.rule_version_hash,
                            computed_asserted_at: entry.computed_asserted_at,
                            value: value.into(),
                        },
                        &spec,
                    )
                    .await?;
                }
                Value::I64(value) => {
                    let spec = ComputedCacheTableSpec {
                        table: AideonComputedCacheI64::Table,
                        partition_col: "partition_id",
                        entity_col: "entity_id",
                        field_col: "field_id",
                        valid_from_col: "valid_from",
                        valid_to_col: "valid_to",
                        rule_hash_col: "rule_version_hash",
                        computed_at_col: "computed_asserted_at_hlc",
                        value_col: &col_name(AideonComputedCacheI64::ValueI64),
                        value_type: ValueType::I64,
                    };
                    upsert_computed_cache_row(
                        &tx,
                        self.backend,
                        ComputedCacheRowInput {
                            partition,
                            entity_id: entry.entity_id,
                            field_id: entry.field_id,
                            valid_from: entry.valid_from,
                            valid_to: entry.valid_to,
                            rule_version_hash: &entry.rule_version_hash,
                            computed_asserted_at: entry.computed_asserted_at,
                            value: value.into(),
                        },
                        &spec,
                    )
                    .await?;
                }
                Value::F64(value) => {
                    let spec = ComputedCacheTableSpec {
                        table: AideonComputedCacheF64::Table,
                        partition_col: "partition_id",
                        entity_col: "entity_id",
                        field_col: "field_id",
                        valid_from_col: "valid_from",
                        valid_to_col: "valid_to",
                        rule_hash_col: "rule_version_hash",
                        computed_at_col: "computed_asserted_at_hlc",
                        value_col: &col_name(AideonComputedCacheF64::ValueF64),
                        value_type: ValueType::F64,
                    };
                    upsert_computed_cache_row(
                        &tx,
                        self.backend,
                        ComputedCacheRowInput {
                            partition,
                            entity_id: entry.entity_id,
                            field_id: entry.field_id,
                            valid_from: entry.valid_from,
                            valid_to: entry.valid_to,
                            rule_version_hash: &entry.rule_version_hash,
                            computed_asserted_at: entry.computed_asserted_at,
                            value: value.into(),
                        },
                        &spec,
                    )
                    .await?;
                }
                Value::Bool(value) => {
                    let spec = ComputedCacheTableSpec {
                        table: AideonComputedCacheBool::Table,
                        partition_col: "partition_id",
                        entity_col: "entity_id",
                        field_col: "field_id",
                        valid_from_col: "valid_from",
                        valid_to_col: "valid_to",
                        rule_hash_col: "rule_version_hash",
                        computed_at_col: "computed_asserted_at_hlc",
                        value_col: &col_name(AideonComputedCacheBool::ValueBool),
                        value_type: ValueType::Bool,
                    };
                    upsert_computed_cache_row(
                        &tx,
                        self.backend,
                        ComputedCacheRowInput {
                            partition,
                            entity_id: entry.entity_id,
                            field_id: entry.field_id,
                            valid_from: entry.valid_from,
                            valid_to: entry.valid_to,
                            rule_version_hash: &entry.rule_version_hash,
                            computed_asserted_at: entry.computed_asserted_at,
                            value: value.into(),
                        },
                        &spec,
                    )
                    .await?;
                }
                Value::Time(value) => {
                    let spec = ComputedCacheTableSpec {
                        table: AideonComputedCacheTime::Table,
                        partition_col: "partition_id",
                        entity_col: "entity_id",
                        field_col: "field_id",
                        valid_from_col: "valid_from",
                        valid_to_col: "valid_to",
                        rule_hash_col: "rule_version_hash",
                        computed_at_col: "computed_asserted_at_hlc",
                        value_col: &col_name(AideonComputedCacheTime::ValueTime),
                        value_type: ValueType::Time,
                    };
                    upsert_computed_cache_row(
                        &tx,
                        self.backend,
                        ComputedCacheRowInput {
                            partition,
                            entity_id: entry.entity_id,
                            field_id: entry.field_id,
                            valid_from: entry.valid_from,
                            valid_to: entry.valid_to,
                            rule_version_hash: &entry.rule_version_hash,
                            computed_asserted_at: entry.computed_asserted_at,
                            value: value.0.into(),
                        },
                        &spec,
                    )
                    .await?;
                }
                Value::Ref(value) => {
                    let spec = ComputedCacheTableSpec {
                        table: AideonComputedCacheRef::Table,
                        partition_col: "partition_id",
                        entity_col: "entity_id",
                        field_col: "field_id",
                        valid_from_col: "valid_from",
                        valid_to_col: "valid_to",
                        rule_hash_col: "rule_version_hash",
                        computed_at_col: "computed_asserted_at_hlc",
                        value_col: &col_name(AideonComputedCacheRef::ValueRefEntityId),
                        value_type: ValueType::Ref,
                    };
                    upsert_computed_cache_row(
                        &tx,
                        self.backend,
                        ComputedCacheRowInput {
                            partition,
                            entity_id: entry.entity_id,
                            field_id: entry.field_id,
                            valid_from: entry.valid_from,
                            valid_to: entry.valid_to,
                            rule_version_hash: &entry.rule_version_hash,
                            computed_asserted_at: entry.computed_asserted_at,
                            value: id_value(self.backend, value),
                        },
                        &spec,
                    )
                    .await?;
                }
                Value::Blob(value) => {
                    let spec = ComputedCacheTableSpec {
                        table: AideonComputedCacheBlob::Table,
                        partition_col: "partition_id",
                        entity_col: "entity_id",
                        field_col: "field_id",
                        valid_from_col: "valid_from",
                        valid_to_col: "valid_to",
                        rule_hash_col: "rule_version_hash",
                        computed_at_col: "computed_asserted_at_hlc",
                        value_col: &col_name(AideonComputedCacheBlob::ValueBlob),
                        value_type: ValueType::Blob,
                    };
                    upsert_computed_cache_row(
                        &tx,
                        self.backend,
                        ComputedCacheRowInput {
                            partition,
                            entity_id: entry.entity_id,
                            field_id: entry.field_id,
                            valid_from: entry.valid_from,
                            valid_to: entry.valid_to,
                            rule_version_hash: &entry.rule_version_hash,
                            computed_asserted_at: entry.computed_asserted_at,
                            value: value.into(),
                        },
                        &spec,
                    )
                    .await?;
                }
                Value::Json(value) => {
                    let spec = ComputedCacheTableSpec {
                        table: AideonComputedCacheJson::Table,
                        partition_col: "partition_id",
                        entity_col: "entity_id",
                        field_col: "field_id",
                        valid_from_col: "valid_from",
                        valid_to_col: "valid_to",
                        rule_hash_col: "rule_version_hash",
                        computed_at_col: "computed_asserted_at_hlc",
                        value_col: &col_name(AideonComputedCacheJson::ValueJson),
                        value_type: ValueType::Json,
                    };
                    upsert_computed_cache_row(
                        &tx,
                        self.backend,
                        ComputedCacheRowInput {
                            partition,
                            entity_id: entry.entity_id,
                            field_id: entry.field_id,
                            valid_from: entry.valid_from,
                            valid_to: entry.valid_to,
                            rule_version_hash: &entry.rule_version_hash,
                            computed_asserted_at: entry.computed_asserted_at,
                            value: value.to_string().into(),
                        },
                        &spec,
                    )
                    .await?;
                }
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_computed_cache(
        &self,
        input: ListComputedCacheInput,
    ) -> MnemeResult<Vec<ComputedCacheEntry>> {
        if input.limit == 0 {
            return Ok(Vec::new());
        }
        let constraints = self
            .fetch_field_def(&self.conn, input.partition, input.field_id)
            .await?;
        match constraints.value_type {
            ValueType::Str => {
                let spec = ComputedCacheTableSpec {
                    table: AideonComputedCacheStr::Table,
                    partition_col: "partition_id",
                    entity_col: "entity_id",
                    field_col: "field_id",
                    valid_from_col: "valid_from",
                    valid_to_col: "valid_to",
                    rule_hash_col: "rule_version_hash",
                    computed_at_col: "computed_asserted_at_hlc",
                    value_col: &col_name(AideonComputedCacheStr::ValueText),
                    value_type: ValueType::Str,
                };
                list_computed_cache_table(&self.conn, self.backend, &input, &spec).await
            }
            ValueType::I64 => {
                let spec = ComputedCacheTableSpec {
                    table: AideonComputedCacheI64::Table,
                    partition_col: "partition_id",
                    entity_col: "entity_id",
                    field_col: "field_id",
                    valid_from_col: "valid_from",
                    valid_to_col: "valid_to",
                    rule_hash_col: "rule_version_hash",
                    computed_at_col: "computed_asserted_at_hlc",
                    value_col: &col_name(AideonComputedCacheI64::ValueI64),
                    value_type: ValueType::I64,
                };
                list_computed_cache_table(&self.conn, self.backend, &input, &spec).await
            }
            ValueType::F64 => {
                let spec = ComputedCacheTableSpec {
                    table: AideonComputedCacheF64::Table,
                    partition_col: "partition_id",
                    entity_col: "entity_id",
                    field_col: "field_id",
                    valid_from_col: "valid_from",
                    valid_to_col: "valid_to",
                    rule_hash_col: "rule_version_hash",
                    computed_at_col: "computed_asserted_at_hlc",
                    value_col: &col_name(AideonComputedCacheF64::ValueF64),
                    value_type: ValueType::F64,
                };
                list_computed_cache_table(&self.conn, self.backend, &input, &spec).await
            }
            ValueType::Bool => {
                let spec = ComputedCacheTableSpec {
                    table: AideonComputedCacheBool::Table,
                    partition_col: "partition_id",
                    entity_col: "entity_id",
                    field_col: "field_id",
                    valid_from_col: "valid_from",
                    valid_to_col: "valid_to",
                    rule_hash_col: "rule_version_hash",
                    computed_at_col: "computed_asserted_at_hlc",
                    value_col: &col_name(AideonComputedCacheBool::ValueBool),
                    value_type: ValueType::Bool,
                };
                list_computed_cache_table(&self.conn, self.backend, &input, &spec).await
            }
            ValueType::Time => {
                let spec = ComputedCacheTableSpec {
                    table: AideonComputedCacheTime::Table,
                    partition_col: "partition_id",
                    entity_col: "entity_id",
                    field_col: "field_id",
                    valid_from_col: "valid_from",
                    valid_to_col: "valid_to",
                    rule_hash_col: "rule_version_hash",
                    computed_at_col: "computed_asserted_at_hlc",
                    value_col: &col_name(AideonComputedCacheTime::ValueTime),
                    value_type: ValueType::Time,
                };
                list_computed_cache_table(&self.conn, self.backend, &input, &spec).await
            }
            ValueType::Ref => {
                let spec = ComputedCacheTableSpec {
                    table: AideonComputedCacheRef::Table,
                    partition_col: "partition_id",
                    entity_col: "entity_id",
                    field_col: "field_id",
                    valid_from_col: "valid_from",
                    valid_to_col: "valid_to",
                    rule_hash_col: "rule_version_hash",
                    computed_at_col: "computed_asserted_at_hlc",
                    value_col: &col_name(AideonComputedCacheRef::ValueRefEntityId),
                    value_type: ValueType::Ref,
                };
                list_computed_cache_table(&self.conn, self.backend, &input, &spec).await
            }
            ValueType::Blob => {
                let spec = ComputedCacheTableSpec {
                    table: AideonComputedCacheBlob::Table,
                    partition_col: "partition_id",
                    entity_col: "entity_id",
                    field_col: "field_id",
                    valid_from_col: "valid_from",
                    valid_to_col: "valid_to",
                    rule_hash_col: "rule_version_hash",
                    computed_at_col: "computed_asserted_at_hlc",
                    value_col: &col_name(AideonComputedCacheBlob::ValueBlob),
                    value_type: ValueType::Blob,
                };
                list_computed_cache_table(&self.conn, self.backend, &input, &spec).await
            }
            ValueType::Json => {
                let spec = ComputedCacheTableSpec {
                    table: AideonComputedCacheJson::Table,
                    partition_col: "partition_id",
                    entity_col: "entity_id",
                    field_col: "field_id",
                    valid_from_col: "valid_from",
                    valid_to_col: "valid_to",
                    rule_hash_col: "rule_version_hash",
                    computed_at_col: "computed_asserted_at_hlc",
                    value_col: &col_name(AideonComputedCacheJson::ValueJson),
                    value_type: ValueType::Json,
                };
                list_computed_cache_table(&self.conn, self.backend, &input, &spec).await
            }
        }
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
                Expr::col(AideonIntegrityHead::PartitionId).eq(id_value(self.backend, partition.0)),
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

    async fn get_schema_manifest(&self) -> MnemeResult<crate::SchemaManifest> {
        load_schema_manifest()
    }

    async fn explain_resolution(
        &self,
        input: crate::api::ExplainResolutionInput,
    ) -> MnemeResult<crate::api::ExplainResolutionResult> {
        if let Some(ctx) = &input.security_context
            && let Some(security) = self
                .read_entity_security_with_fallback(
                    input.partition,
                    input.scenario_id,
                    input.entity_id,
                )
                .await?
            && !Self::is_entity_visible(ctx, &security)
        {
            return Err(MnemeError::not_found("entity not visible"));
        }
        let constraints = self
            .fetch_field_def(&self.conn, input.partition, input.field_id)
            .await?;
        let facts = fetch_property_facts_with_fallback(
            &self.conn,
            PropertyFactQueryInput {
                partition: input.partition,
                scenario_id: input.scenario_id,
                entity_id: input.entity_id,
                field_id: input.field_id,
                at_valid_time: input.at_valid_time,
                as_of_asserted_at: input.as_of_asserted_at,
                value_type: constraints.value_type,
            },
            self.backend,
        )
        .await?;
        let resolved = resolve_property(constraints.merge_policy, facts.clone(), &self.limits)?;
        let mut candidates = facts
            .into_iter()
            .map(|fact| explain_property_fact(&fact))
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| compare_precedence(&a.precedence, &b.precedence));
        let winner = candidates.first().cloned();
        Ok(crate::api::ExplainResolutionResult {
            entity_id: input.entity_id,
            field_id: input.field_id,
            resolved,
            winner,
            candidates,
        })
    }

    async fn explain_traversal(
        &self,
        input: crate::api::ExplainTraversalInput,
    ) -> MnemeResult<crate::api::ExplainTraversalResult> {
        if let Some(ctx) = &input.security_context
            && let Some(security) = self
                .read_entity_security_with_fallback(
                    input.partition,
                    input.scenario_id,
                    input.edge_id,
                )
                .await?
            && !Self::is_entity_visible(ctx, &security)
        {
            return Err(MnemeError::not_found("edge not visible"));
        }
        let facts = fetch_edge_facts_for_edge(
            &self.conn,
            input.partition,
            input.scenario_id,
            input.edge_id,
            input.at_valid_time,
            input.as_of_asserted_at,
            self.backend,
        )
        .await?;
        let mut candidates = facts
            .into_iter()
            .map(|fact| explain_edge_fact(&fact))
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| compare_precedence(&a.precedence, &b.precedence));
        let winner = candidates.first().cloned();
        let active = winner
            .as_ref()
            .map(|fact| !fact.is_tombstone)
            .unwrap_or(false);
        Ok(crate::api::ExplainTraversalResult {
            edge_id: input.edge_id,
            active,
            winner,
            candidates,
        })
    }
}

#[async_trait]
impl MnemeExportApi for MnemeStore {
    async fn export_ops_stream(
        &self,
        options: ExportOptions,
    ) -> MnemeResult<Box<dyn Iterator<Item = ExportRecord> + Send>> {
        #[derive(Serialize)]
        struct ExportHeaderOptions {
            include_schema: bool,
            include_data_ops: bool,
            include_scenarios: bool,
        }

        #[derive(Serialize)]
        struct ExportHeader {
            format_version: String,
            exported_at_asserted: String,
            partition_id: PartitionId,
            scenario_id: Option<ScenarioId>,
            mneme_version: String,
            options: ExportHeaderOptions,
        }

        #[derive(Serialize)]
        struct ExportOpRecord {
            partition_id: PartitionId,
            scenario_id: Option<ScenarioId>,
            op_id: OpId,
            actor_id: ActorId,
            asserted_at: String,
            op_type: u16,
            payload_base64: String,
            deps: Vec<OpId>,
        }

        #[derive(Serialize)]
        struct ExportFooter {
            op_count: u32,
            checksum: String,
        }

        let mut select = Query::select()
            .from(AideonOps::Table)
            .columns([
                (AideonOps::Table, AideonOps::OpId),
                (AideonOps::Table, AideonOps::ActorId),
                (AideonOps::Table, AideonOps::AssertedAtHlc),
                (AideonOps::Table, AideonOps::OpType),
                (AideonOps::Table, AideonOps::ScenarioId),
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
        let header = ExportHeader {
            format_version: "1.0".to_string(),
            exported_at_asserted: Hlc::now().as_i64().to_string(),
            partition_id: options.partition,
            scenario_id: options.scenario_id,
            mneme_version: env!("CARGO_PKG_VERSION").to_string(),
            options: ExportHeaderOptions {
                include_schema: options.include_schema,
                include_data_ops: options.include_data_ops,
                include_scenarios: options.include_scenarios,
            },
        };
        let _ = (
            header.partition_id,
            header.scenario_id,
            header.options.include_schema,
            header.options.include_data_ops,
            header.options.include_scenarios,
        );
        let header_value =
            serde_json::to_value(header).map_err(|err| MnemeError::storage(err.to_string()))?;
        records.push(ExportRecord {
            record_type: "header".to_string(),
            data: header_value,
        });
        let mut hasher = blake3::Hasher::new();
        let mut count = 0u32;
        for row in rows {
            let op_id = read_id(&row, AideonOps::OpId)?;
            let actor_id = read_id(&row, AideonOps::ActorId)?;
            let asserted_at = read_hlc(&row, AideonOps::AssertedAtHlc)?;
            let op_type: i64 = row.try_get("", &col_name(AideonOps::OpType))?;
            let scenario_from_row = read_opt_id(&row, AideonOps::ScenarioId)?;
            let payload: Vec<u8> = row.try_get("", &col_name(AideonOps::Payload))?;
            let op_payload: OpPayload = serde_json::from_slice(&payload)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let payload_scenario = match scenario_from_row {
                Some(id) => Some(ScenarioId(id)),
                None => scenario_id_from_payload(&payload)?,
            };
            let is_schema = matches!(op_payload, OpPayload::UpsertMetamodelBatch(_));
            let is_scenario_op = matches!(
                op_payload,
                OpPayload::CreateScenario { .. } | OpPayload::DeleteScenario { .. }
            );
            let is_data_op = !is_schema && !is_scenario_op;

            if is_schema && !options.include_schema {
                continue;
            }
            if is_data_op && !options.include_data_ops {
                continue;
            }
            if is_scenario_op && !options.include_scenarios {
                continue;
            }
            if options.scenario_id.is_some() && !is_schema {
                if payload_scenario != options.scenario_id {
                    continue;
                }
            } else if payload_scenario.is_some() && !options.include_scenarios && !is_schema {
                continue;
            }

            let deps = fetch_op_deps(&self.conn, options.partition, op_id).await?;
            let record = ExportOpRecord {
                partition_id: options.partition,
                scenario_id: payload_scenario,
                op_id: OpId(op_id),
                actor_id: ActorId(actor_id),
                asserted_at: asserted_at.as_i64().to_string(),
                op_type: op_type as u16,
                payload_base64: BASE64.encode(&payload),
                deps: deps.into_iter().map(OpId).collect(),
            };
            let data = serde_json::to_value(&record)
                .map_err(|err| MnemeError::storage(err.to_string()))?;
            let bytes =
                serde_json::to_vec(&record).map_err(|err| MnemeError::storage(err.to_string()))?;
            hasher.update(&bytes);
            count += 1;
            records.push(ExportRecord {
                record_type: "op".to_string(),
                data,
            });
        }
        let checksum = hasher.finalize().to_hex().to_string();
        let footer = ExportFooter {
            op_count: count,
            checksum,
        };
        let _ = footer.op_count;
        let footer_value =
            serde_json::to_value(footer).map_err(|err| MnemeError::storage(err.to_string()))?;
        records.push(ExportRecord {
            record_type: "footer".to_string(),
            data: footer_value,
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
        #[derive(Deserialize)]
        struct ExportHeaderOptions {
            include_schema: bool,
            include_data_ops: bool,
            include_scenarios: bool,
        }

        #[derive(Deserialize)]
        struct ExportHeader {
            format_version: String,
            partition_id: PartitionId,
            scenario_id: Option<ScenarioId>,
            options: ExportHeaderOptions,
        }

        #[derive(Deserialize, Serialize)]
        struct ExportOpRecord {
            partition_id: PartitionId,
            scenario_id: Option<ScenarioId>,
            op_id: OpId,
            actor_id: ActorId,
            asserted_at: String,
            op_type: u16,
            payload_base64: String,
            deps: Vec<OpId>,
        }

        #[derive(Deserialize)]
        struct ExportFooter {
            op_count: u32,
            checksum: String,
        }

        fn parse_hlc_str(value: &str) -> MnemeResult<Hlc> {
            let parsed = value
                .parse::<i64>()
                .map_err(|_| MnemeError::invalid("invalid HLC string"))?;
            Ok(Hlc::from_i64(parsed))
        }

        let mut hasher = blake3::Hasher::new();
        let mut ops = Vec::new();
        let mut schema_ops = Vec::new();
        let mut reported_checksum: Option<String> = None;
        let mut header: Option<ExportHeader> = None;
        let mut op_count_seen = 0u32;
        for record in records {
            match record.record_type.as_str() {
                "header" => {
                    let parsed: ExportHeader = serde_json::from_value(record.data)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    if options.strict_schema && parsed.format_version != "1.0" {
                        return Err(MnemeError::invalid("unsupported export format version"));
                    }
                    let _ = (
                        parsed.partition_id,
                        parsed.scenario_id,
                        parsed.options.include_schema,
                        parsed.options.include_data_ops,
                        parsed.options.include_scenarios,
                    );
                    header = Some(parsed);
                }
                "footer" => {
                    let footer: ExportFooter = serde_json::from_value(record.data)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    if options.strict_schema && footer.op_count != op_count_seen {
                        return Err(MnemeError::invalid("op count mismatch"));
                    }
                    reported_checksum = Some(footer.checksum);
                }
                "op" => {
                    let mut op_record: ExportOpRecord = serde_json::from_value(record.data)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    let payload_bytes = BASE64
                        .decode(op_record.payload_base64.as_bytes())
                        .map_err(|err| {
                            MnemeError::invalid(format!("payload decode failed: {err}"))
                        })?;
                    let asserted_at = parse_hlc_str(&op_record.asserted_at)?;
                    let mut payload: OpPayload = serde_json::from_slice(&payload_bytes)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    if let Some(remapped) = options.remap_actor_ids.get(&op_record.actor_id) {
                        op_record.actor_id = *remapped;
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
                    match &mut payload {
                        OpPayload::CreateNode(input) => input.partition = options.target_partition,
                        OpPayload::CreateEdge(input) => input.partition = options.target_partition,
                        OpPayload::SetEdgeExistenceInterval(input) => {
                            input.partition = options.target_partition
                        }
                        OpPayload::TombstoneEntity { partition, .. } => {
                            *partition = options.target_partition
                        }
                        OpPayload::SetProperty(input) => input.partition = options.target_partition,
                        OpPayload::ClearProperty(input) => {
                            input.partition = options.target_partition
                        }
                        OpPayload::OrSetUpdate(input) => input.partition = options.target_partition,
                        OpPayload::CounterUpdate(input) => {
                            input.partition = options.target_partition
                        }
                        OpPayload::UpsertMetamodelBatch(_) => {}
                        OpPayload::CreateScenario { partition, .. } => {
                            *partition = options.target_partition
                        }
                        OpPayload::DeleteScenario { partition, .. } => {
                            *partition = options.target_partition
                        }
                    }
                    if let Some(scenario_id) = options.scenario_id {
                        match &mut payload {
                            OpPayload::CreateNode(input) => input.scenario_id = Some(scenario_id),
                            OpPayload::CreateEdge(input) => input.scenario_id = Some(scenario_id),
                            OpPayload::SetEdgeExistenceInterval(input) => {
                                input.scenario_id = Some(scenario_id)
                            }
                            OpPayload::TombstoneEntity {
                                scenario_id: sid, ..
                            } => *sid = Some(scenario_id),
                            OpPayload::SetProperty(input) => input.scenario_id = Some(scenario_id),
                            OpPayload::ClearProperty(input) => {
                                input.scenario_id = Some(scenario_id)
                            }
                            OpPayload::OrSetUpdate(input) => input.scenario_id = Some(scenario_id),
                            OpPayload::CounterUpdate(input) => {
                                input.scenario_id = Some(scenario_id)
                            }
                            OpPayload::UpsertMetamodelBatch(_) => {}
                            OpPayload::CreateScenario { .. } => {}
                            OpPayload::DeleteScenario { .. } => {}
                        }
                    }
                    let bytes = serde_json::to_vec(&op_record)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    let payload_bytes = serde_json::to_vec(&payload)
                        .map_err(|err| MnemeError::storage(err.to_string()))?;
                    let envelope = OpEnvelope {
                        op_id: op_record.op_id,
                        actor_id: op_record.actor_id,
                        asserted_at,
                        op_type: op_record.op_type,
                        payload: payload_bytes,
                        deps: op_record.deps.clone(),
                    };
                    hasher.update(&bytes);
                    op_count_seen += 1;
                    if matches!(payload, OpPayload::UpsertMetamodelBatch(_)) {
                        schema_ops.push(envelope);
                    } else {
                        ops.push(envelope);
                    }
                }
                _ => {}
            }
        }
        if header.is_none() && options.strict_schema {
            return Err(MnemeError::invalid("export header missing"));
        }
        if let Some(checksum) = reported_checksum {
            let computed = hasher.finalize().to_hex().to_string();
            if checksum != computed && options.strict_schema {
                return Err(MnemeError::invalid("checksum mismatch"));
            }
        }

        if options.allow_partition_create
            && let Some(first) = ops.first()
        {
            let tx = self.conn.begin().await?;
            self.ensure_partition(&tx, options.target_partition, first.actor_id)
                .await?;
            tx.commit().await?;
        }

        let mut imported = 0u32;
        for chunk in schema_ops.chunks(1000) {
            self.ingest_ops(options.target_partition, chunk.to_vec())
                .await?;
            imported += chunk.len() as u32;
        }
        for chunk in ops.chunks(1000) {
            self.ingest_ops(options.target_partition, chunk.to_vec())
                .await?;
            imported += chunk.len() as u32;
        }
        let reason = "import".to_string();
        let trigger = TriggerProcessingInput {
            partition: options.target_partition,
            scenario_id: options.scenario_id,
            reason,
        };
        self.trigger_rebuild_effective_schema(trigger.clone())
            .await?;
        self.trigger_refresh_integrity(trigger.clone()).await?;
        self.trigger_refresh_analytics_projections(trigger).await?;
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
                    AideonEntities::CreatedOpId,
                    AideonEntities::UpdatedOpId,
                    AideonEntities::CreatedAssertedAtHlc,
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
                    Expr::col(AideonEntities::ScenarioId).eq(id_value(self.backend, scenario_id.0)),
                );
            } else {
                select.and_where(Expr::col(AideonEntities::ScenarioId).is_null());
            }
            for row in query_all(&self.conn, &select).await? {
                let entity_id = read_id(&row, AideonEntities::EntityId)?;
                let kind_raw: i16 = row.try_get("", &col_name(AideonEntities::EntityKind))?;
                let type_id = read_opt_id(&row, AideonEntities::TypeId)?;
                let is_deleted: bool = row.try_get("", &col_name(AideonEntities::IsDeleted))?;
                let created_op_id = read_id(&row, AideonEntities::CreatedOpId)?;
                let updated_op_id = read_id(&row, AideonEntities::UpdatedOpId)?;
                let created_asserted_at = read_hlc(&row, AideonEntities::CreatedAssertedAtHlc)?;
                let updated_asserted_at = read_hlc(&row, AideonEntities::UpdatedAssertedAtHlc)?;
                records.push(ExportRecord {
                    record_type: "snapshot_entity".to_string(),
                    data: serde_json::json!({
                        "entity_id": entity_id,
                        "kind": kind_raw,
                        "type_id": type_id,
                        "is_deleted": is_deleted,
                        "created_op_id": created_op_id,
                        "updated_op_id": updated_op_id,
                        "created_asserted_at": created_asserted_at.as_i64(),
                        "updated_asserted_at": updated_asserted_at.as_i64(),
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
                    Expr::col(AideonEdges::ScenarioId).eq(id_value(self.backend, scenario_id.0)),
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
                    AideonEdgeExistsFacts::OpId,
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
                let valid_from: i64 =
                    row.try_get("", &col_name(AideonEdgeExistsFacts::ValidFrom))?;
                let valid_to: Option<i64> =
                    row.try_get("", &col_name(AideonEdgeExistsFacts::ValidTo))?;
                let layer: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::Layer))?;
                let asserted_at = read_hlc(&row, AideonEdgeExistsFacts::AssertedAtHlc)?;
                let op_id = read_id(&row, AideonEdgeExistsFacts::OpId)?;
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
                        "op_id": op_id,
                        "is_tombstone": is_tombstone,
                    }),
                });
            }

            self.export_fact_table(
                &mut records,
                &opts,
                ExportFactTableSpec {
                    table: AideonPropFactStr::Table,
                    entity_col: AideonPropFactStr::EntityId,
                    field_col: AideonPropFactStr::FieldId,
                    valid_from_col: AideonPropFactStr::ValidFrom,
                    valid_to_col: AideonPropFactStr::ValidTo,
                    layer_col: AideonPropFactStr::Layer,
                    asserted_col: AideonPropFactStr::AssertedAtHlc,
                    op_id_col: AideonPropFactStr::OpId,
                    tombstone_col: AideonPropFactStr::IsTombstone,
                    value_col: AideonPropFactStr::ValueText,
                    record_type: "snapshot_fact_str",
                },
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                ExportFactTableSpec {
                    table: AideonPropFactI64::Table,
                    entity_col: AideonPropFactI64::EntityId,
                    field_col: AideonPropFactI64::FieldId,
                    valid_from_col: AideonPropFactI64::ValidFrom,
                    valid_to_col: AideonPropFactI64::ValidTo,
                    layer_col: AideonPropFactI64::Layer,
                    asserted_col: AideonPropFactI64::AssertedAtHlc,
                    op_id_col: AideonPropFactI64::OpId,
                    tombstone_col: AideonPropFactI64::IsTombstone,
                    value_col: AideonPropFactI64::ValueI64,
                    record_type: "snapshot_fact_i64",
                },
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                ExportFactTableSpec {
                    table: AideonPropFactF64::Table,
                    entity_col: AideonPropFactF64::EntityId,
                    field_col: AideonPropFactF64::FieldId,
                    valid_from_col: AideonPropFactF64::ValidFrom,
                    valid_to_col: AideonPropFactF64::ValidTo,
                    layer_col: AideonPropFactF64::Layer,
                    asserted_col: AideonPropFactF64::AssertedAtHlc,
                    op_id_col: AideonPropFactF64::OpId,
                    tombstone_col: AideonPropFactF64::IsTombstone,
                    value_col: AideonPropFactF64::ValueF64,
                    record_type: "snapshot_fact_f64",
                },
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                ExportFactTableSpec {
                    table: AideonPropFactBool::Table,
                    entity_col: AideonPropFactBool::EntityId,
                    field_col: AideonPropFactBool::FieldId,
                    valid_from_col: AideonPropFactBool::ValidFrom,
                    valid_to_col: AideonPropFactBool::ValidTo,
                    layer_col: AideonPropFactBool::Layer,
                    asserted_col: AideonPropFactBool::AssertedAtHlc,
                    op_id_col: AideonPropFactBool::OpId,
                    tombstone_col: AideonPropFactBool::IsTombstone,
                    value_col: AideonPropFactBool::ValueBool,
                    record_type: "snapshot_fact_bool",
                },
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                ExportFactTableSpec {
                    table: AideonPropFactTime::Table,
                    entity_col: AideonPropFactTime::EntityId,
                    field_col: AideonPropFactTime::FieldId,
                    valid_from_col: AideonPropFactTime::ValidFrom,
                    valid_to_col: AideonPropFactTime::ValidTo,
                    layer_col: AideonPropFactTime::Layer,
                    asserted_col: AideonPropFactTime::AssertedAtHlc,
                    op_id_col: AideonPropFactTime::OpId,
                    tombstone_col: AideonPropFactTime::IsTombstone,
                    value_col: AideonPropFactTime::ValueTime,
                    record_type: "snapshot_fact_time",
                },
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                ExportFactTableSpec {
                    table: AideonPropFactRef::Table,
                    entity_col: AideonPropFactRef::EntityId,
                    field_col: AideonPropFactRef::FieldId,
                    valid_from_col: AideonPropFactRef::ValidFrom,
                    valid_to_col: AideonPropFactRef::ValidTo,
                    layer_col: AideonPropFactRef::Layer,
                    asserted_col: AideonPropFactRef::AssertedAtHlc,
                    op_id_col: AideonPropFactRef::OpId,
                    tombstone_col: AideonPropFactRef::IsTombstone,
                    value_col: AideonPropFactRef::ValueRefEntityId,
                    record_type: "snapshot_fact_ref",
                },
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                ExportFactTableSpec {
                    table: AideonPropFactBlob::Table,
                    entity_col: AideonPropFactBlob::EntityId,
                    field_col: AideonPropFactBlob::FieldId,
                    valid_from_col: AideonPropFactBlob::ValidFrom,
                    valid_to_col: AideonPropFactBlob::ValidTo,
                    layer_col: AideonPropFactBlob::Layer,
                    asserted_col: AideonPropFactBlob::AssertedAtHlc,
                    op_id_col: AideonPropFactBlob::OpId,
                    tombstone_col: AideonPropFactBlob::IsTombstone,
                    value_col: AideonPropFactBlob::ValueBlob,
                    record_type: "snapshot_fact_blob",
                },
            )
            .await?;
            self.export_fact_table(
                &mut records,
                &opts,
                ExportFactTableSpec {
                    table: AideonPropFactJson::Table,
                    entity_col: AideonPropFactJson::EntityId,
                    field_col: AideonPropFactJson::FieldId,
                    valid_from_col: AideonPropFactJson::ValidFrom,
                    valid_to_col: AideonPropFactJson::ValidTo,
                    layer_col: AideonPropFactJson::Layer,
                    asserted_col: AideonPropFactJson::AssertedAtHlc,
                    op_id_col: AideonPropFactJson::OpId,
                    tombstone_col: AideonPropFactJson::IsTombstone,
                    value_col: AideonPropFactJson::ValueJson,
                    record_type: "snapshot_fact_json",
                },
            )
            .await?;
        }

        records.push(ExportRecord {
            record_type: "snapshot_footer".to_string(),
            data: serde_json::json!({ "complete": true }),
        });
        Ok(Box::new(records.into_iter()))
    }

    async fn import_snapshot_stream<I>(&self, opts: ImportOptions, records: I) -> MnemeResult<()>
    where
        I: Iterator<Item = ExportRecord> + Send,
    {
        if opts.allow_partition_create {
            let tx = self.conn.begin().await?;
            self.ensure_partition(&tx, opts.target_partition, ActorId(Id::new()))
                .await?;
            tx.commit().await?;
        }
        if partition_has_rows(&self.conn, opts.target_partition, self.backend).await? {
            return Err(MnemeError::invalid(
                "snapshot import requires empty partition",
            ));
        }
        let mut header_seen = false;
        let mut footer_seen = false;
        let mut header_partition = None;
        let mut header_scenario = None;
        let mut header_as_of = None;
        for record in records {
            match record.record_type.as_str() {
                "snapshot_header" => {
                    if header_seen {
                        return Err(MnemeError::invalid("duplicate snapshot header"));
                    }
                    let partition_id = record
                        .data
                        .get("partition_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| MnemeError::invalid("missing partition_id"))?;
                    let partition_id = PartitionId(Id::from_uuid_str(partition_id)?);
                    let scenario_id = match record.data.get("scenario_id") {
                        Some(value) if !value.is_null() => Some(ScenarioId(Id::from_uuid_str(
                            value.as_str().unwrap_or_default(),
                        )?)),
                        _ => None,
                    };
                    let as_of = record
                        .data
                        .get("as_of_asserted_at")
                        .and_then(|v| v.as_i64())
                        .map(Hlc::from_i64);
                    header_partition = Some(partition_id);
                    header_scenario = scenario_id;
                    header_as_of = as_of;
                    header_seen = true;
                    if partition_id != opts.target_partition {
                        return Err(MnemeError::invalid("snapshot partition mismatch"));
                    }
                    if scenario_id != opts.scenario_id {
                        return Err(MnemeError::invalid("snapshot scenario mismatch"));
                    }
                }
                "snapshot_entity" => {
                    if !header_seen {
                        return Err(MnemeError::invalid("snapshot header required"));
                    }
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
                    let created_op_id = Id::from_uuid_str(
                        record
                            .data
                            .get("created_op_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| MnemeError::invalid("missing created_op_id"))?,
                    )?;
                    let updated_op_id = Id::from_uuid_str(
                        record
                            .data
                            .get("updated_op_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| MnemeError::invalid("missing updated_op_id"))?,
                    )?;
                    let created_asserted_at = record
                        .data
                        .get("created_asserted_at")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| MnemeError::invalid("missing created_asserted_at"))?;
                    let updated_asserted_at = record
                        .data
                        .get("updated_asserted_at")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| MnemeError::invalid("missing updated_asserted_at"))?;
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
                            id_value(self.backend, created_op_id).into(),
                            id_value(self.backend, updated_op_id).into(),
                            created_asserted_at.into(),
                            updated_asserted_at.into(),
                        ])
                        .to_owned();
                    exec(&self.conn, &insert).await?;
                }
                "snapshot_edge" => {
                    if !header_seen {
                        return Err(MnemeError::invalid("snapshot header required"));
                    }
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
                    if !header_seen {
                        return Err(MnemeError::invalid("snapshot header required"));
                    }
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
                    let op_id = Id::from_uuid_str(
                        record
                            .data
                            .get("op_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| MnemeError::invalid("missing op_id"))?,
                    )?;
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
                            id_value(self.backend, op_id).into(),
                            is_tombstone.into(),
                        ])
                        .to_owned();
                    exec(&self.conn, &insert).await?;
                }
                "snapshot_fact_str" | "snapshot_fact_i64" | "snapshot_fact_f64"
                | "snapshot_fact_bool" | "snapshot_fact_time" | "snapshot_fact_ref"
                | "snapshot_fact_blob" | "snapshot_fact_json" => {
                    if !header_seen {
                        return Err(MnemeError::invalid("snapshot header required"));
                    }
                    self.import_fact_record(&opts, &record).await?;
                }
                "snapshot_footer" => {
                    footer_seen = true;
                }
                _ => {}
            }
        }
        if !header_seen {
            return Err(MnemeError::invalid("snapshot header required"));
        }
        if !footer_seen {
            return Err(MnemeError::invalid("snapshot footer required"));
        }
        let _ = (header_partition, header_scenario, header_as_of);
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

#[derive(Clone)]
struct EntitySecurity {
    acl_group_id: Option<String>,
    owner_actor_id: Option<ActorId>,
    visibility: Option<u8>,
    is_deleted: bool,
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

async fn upsert_computed_cache_row<C, TTable>(
    conn: &C,
    backend: DatabaseBackend,
    input: ComputedCacheRowInput<'_>,
    spec: &ComputedCacheTableSpec<'_, TTable>,
) -> MnemeResult<()>
where
    C: ConnectionTrait,
    TTable: sea_query::Iden + Clone,
{
    let partition_col = Alias::new(spec.partition_col);
    let entity_col = Alias::new(spec.entity_col);
    let field_col = Alias::new(spec.field_col);
    let valid_from_col = Alias::new(spec.valid_from_col);
    let valid_to_col = Alias::new(spec.valid_to_col);
    let rule_hash_col = Alias::new(spec.rule_hash_col);
    let computed_at_col = Alias::new(spec.computed_at_col);
    let value_col = Alias::new(spec.value_col);
    let insert = Query::insert()
        .into_table(spec.table.clone())
        .columns([
            partition_col.clone(),
            entity_col.clone(),
            field_col.clone(),
            valid_from_col.clone(),
            valid_to_col.clone(),
            rule_hash_col.clone(),
            computed_at_col.clone(),
            value_col.clone(),
        ])
        .values_panic([
            Expr::val(id_value(backend, input.partition.0)),
            Expr::val(id_value(backend, input.entity_id)),
            Expr::val(id_value(backend, input.field_id)),
            Expr::val(input.valid_from),
            Expr::val(input.valid_to),
            Expr::val(input.rule_version_hash),
            Expr::val(input.computed_asserted_at.as_i64()),
            Expr::val(input.value),
        ])
        .on_conflict(
            OnConflict::columns([
                partition_col.clone(),
                entity_col.clone(),
                field_col.clone(),
                valid_from_col.clone(),
                rule_hash_col.clone(),
            ])
            .update_columns([valid_to_col, computed_at_col, value_col])
            .to_owned(),
        )
        .to_owned();
    exec(conn, &insert).await?;
    Ok(())
}

async fn list_computed_cache_table<C>(
    conn: &C,
    backend: DatabaseBackend,
    input: &ListComputedCacheInput,
    spec: &ComputedCacheTableSpec<'_, impl sea_query::Iden + Clone>,
) -> MnemeResult<Vec<ComputedCacheEntry>>
where
    C: ConnectionTrait,
{
    let partition_col = Alias::new(spec.partition_col);
    let entity_col = Alias::new(spec.entity_col);
    let field_col = Alias::new(spec.field_col);
    let valid_from_col = Alias::new(spec.valid_from_col);
    let valid_to_col = Alias::new(spec.valid_to_col);
    let rule_hash_col = Alias::new(spec.rule_hash_col);
    let computed_at_col = Alias::new(spec.computed_at_col);
    let value_col = Alias::new(spec.value_col);
    let mut select = Query::select()
        .from(spec.table.clone())
        .columns([
            entity_col.clone(),
            field_col.clone(),
            valid_from_col.clone(),
            valid_to_col.clone(),
            rule_hash_col.clone(),
            computed_at_col.clone(),
            value_col.clone(),
        ])
        .and_where(Expr::col(partition_col.clone()).eq(id_value(backend, input.partition.0)))
        .and_where(Expr::col(field_col.clone()).eq(id_value(backend, input.field_id)))
        .to_owned();
    if let Some(entity_id) = input.entity_id {
        select.and_where(Expr::col(entity_col.clone()).eq(id_value(backend, entity_id)));
    }
    if let Some(at_valid_time) = input.at_valid_time {
        select
            .and_where(Expr::col(valid_from_col.clone()).lte(at_valid_time.0))
            .and_where(
                Expr::col(valid_to_col.clone())
                    .gt(at_valid_time.0)
                    .or(Expr::col(valid_to_col.clone()).is_null()),
            );
    }
    select
        .order_by(entity_col.clone(), Order::Asc)
        .order_by(valid_from_col.clone(), Order::Asc)
        .limit(input.limit as u64);
    let rows = query_all(conn, &select).await?;
    let mut entries = Vec::new();
    for row in rows {
        let entity_id = read_id_by_name(&row, &col_name(entity_col.clone()))?;
        let field_id = read_id_by_name(&row, &col_name(field_col.clone()))?;
        let valid_from: i64 = row.try_get("", &col_name(valid_from_col.clone()))?;
        let valid_to: Option<i64> = row.try_get("", &col_name(valid_to_col.clone()))?;
        let rule_version_hash: String = row.try_get("", &col_name(rule_hash_col.clone()))?;
        let computed_asserted_at: i64 = row.try_get("", &col_name(computed_at_col.clone()))?;
        let value = read_value(spec.value_type, &row, &col_name(value_col.clone()))?;
        entries.push(ComputedCacheEntry {
            entity_id,
            field_id,
            valid_from,
            valid_to,
            value,
            rule_version_hash,
            computed_asserted_at: Hlc::from_i64(computed_asserted_at),
        });
    }
    Ok(entries)
}

#[derive(Clone)]
struct CompactionRow {
    scenario_id: Option<Id>,
    entity_id: Id,
    field_id: Id,
    valid_from: i64,
    valid_to: Option<i64>,
    layer: i64,
    asserted_at: i64,
    op_id: Id,
    is_tombstone: bool,
    value: Value,
}

#[derive(Default)]
struct LwwFields {
    str_fields: Vec<Id>,
    i64_fields: Vec<Id>,
    f64_fields: Vec<Id>,
    bool_fields: Vec<Id>,
    time_fields: Vec<Id>,
    ref_fields: Vec<Id>,
    blob_fields: Vec<Id>,
    json_fields: Vec<Id>,
}

impl LwwFields {
    fn push(&mut self, value_type: ValueType, field_id: Id) {
        match value_type {
            ValueType::Str => self.str_fields.push(field_id),
            ValueType::I64 => self.i64_fields.push(field_id),
            ValueType::F64 => self.f64_fields.push(field_id),
            ValueType::Bool => self.bool_fields.push(field_id),
            ValueType::Time => self.time_fields.push(field_id),
            ValueType::Ref => self.ref_fields.push(field_id),
            ValueType::Blob => self.blob_fields.push(field_id),
            ValueType::Json => self.json_fields.push(field_id),
        }
    }
}

type PropFactKey = (Option<Id>, Id, Id, i64, Option<i64>, i64);
type EdgeExistsKey = (Option<Id>, Id, i64, Option<i64>, i64);
type EdgeExistsMergeKey = (Option<Id>, Id, i64, Option<i64>, i64, i64, Id, bool);

struct PropFactRowKey {
    scenario_id: Option<Id>,
    entity_id: Id,
    field_id: Id,
    valid_from: i64,
    asserted_at: i64,
    op_id: Id,
}

struct PropFactTableInfo<'a, T: sea_query::Iden + Clone> {
    backend: DatabaseBackend,
    partition: PartitionId,
    table: T,
    scenario_col: &'a str,
    entity_col: &'a str,
    field_col: &'a str,
    valid_from_col: &'a str,
    valid_to_col: &'a str,
    asserted_col: &'a str,
    op_id_col: &'a str,
}

struct PropFactCompactionSpec<'a, T: sea_query::Iden + Clone> {
    table_info: PropFactTableInfo<'a, T>,
    field_ids: &'a [Id],
    layer_col: &'a str,
    is_tombstone_col: &'a str,
    value_col: &'a str,
    value_type: ValueType,
}

struct EdgeExistsRowKey {
    scenario_id: Option<Id>,
    edge_id: Id,
    valid_from: i64,
    asserted_at: i64,
    op_id: Id,
}

struct EdgeExistsTableInfo {
    backend: DatabaseBackend,
    partition: PartitionId,
}

struct ExportFactTableSpec<
    'a,
    TTable: sea_query::Iden + Clone,
    TEntity: sea_query::Iden + Clone,
    TField: sea_query::Iden + Clone,
    TValidFrom: sea_query::Iden + Clone,
    TValidTo: sea_query::Iden + Clone,
    TLayer: sea_query::Iden + Clone,
    TAsserted: sea_query::Iden + Clone,
    TOpId: sea_query::Iden + Clone,
    TTombstone: sea_query::Iden + Clone,
    TValue: sea_query::Iden + Clone,
> {
    table: TTable,
    entity_col: TEntity,
    field_col: TField,
    valid_from_col: TValidFrom,
    valid_to_col: TValidTo,
    layer_col: TLayer,
    asserted_col: TAsserted,
    op_id_col: TOpId,
    tombstone_col: TTombstone,
    value_col: TValue,
    record_type: &'a str,
}

struct PropertyOverlapInput {
    partition: PartitionId,
    scenario_id: Option<ScenarioId>,
    entity_id: Id,
    field_id: Id,
    valid_from: ValidTime,
    valid_to: Option<ValidTime>,
}

struct PropertyOverlapTableSpec<
    TTable: Iden + Copy,
    TPartition: Iden + Copy,
    TScenario: Iden + Copy,
    TEntity: Iden + Copy,
    TField: Iden + Copy,
    TValidFrom: Iden + Copy,
    TValidTo: Iden + Copy,
> {
    table: TTable,
    partition_col: TPartition,
    scenario_col: TScenario,
    entity_col: TEntity,
    field_col: TField,
    valid_from_col: TValidFrom,
    valid_to_col: TValidTo,
}

struct OverlapFindingInput<'a> {
    partition: PartitionId,
    scenario_id: Option<ScenarioId>,
    entity_id: Id,
    field_id: Id,
    valid_from: ValidTime,
    valid_to: Option<ValidTime>,
    op_kind: &'a str,
}

struct ComputedCacheRowInput<'a> {
    partition: PartitionId,
    entity_id: Id,
    field_id: Id,
    valid_from: i64,
    valid_to: Option<i64>,
    rule_version_hash: &'a str,
    computed_asserted_at: Hlc,
    value: SeaValue,
}

struct ComputedCacheTableSpec<'a, TTable: sea_query::Iden + Clone> {
    table: TTable,
    partition_col: &'a str,
    entity_col: &'a str,
    field_col: &'a str,
    valid_from_col: &'a str,
    valid_to_col: &'a str,
    rule_hash_col: &'a str,
    computed_at_col: &'a str,
    value_col: &'a str,
    value_type: ValueType,
}

struct PropertyFactInsertInput<'a> {
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    entity_id: Id,
    field_id: Id,
    value: &'a Value,
    valid_from: ValidTime,
    valid_to: Option<ValidTime>,
    layer: Layer,
    asserted_at: Hlc,
    op_id: OpId,
    is_tombstone: bool,
}

struct IndexInsertInput<'a> {
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    field_id: Id,
    entity_id: Id,
    value: &'a Value,
    valid_from: ValidTime,
    valid_to: Option<ValidTime>,
    asserted_at: Hlc,
    layer: Layer,
}

struct PropertyFactQueryInput {
    partition: PartitionId,
    scenario_id: Option<crate::ScenarioId>,
    entity_id: Id,
    field_id: Id,
    at_valid_time: ValidTime,
    as_of_asserted_at: Option<Hlc>,
    value_type: ValueType,
}

async fn compact_prop_fact_table<C>(
    conn: &C,
    spec: PropFactCompactionSpec<'_, impl sea_query::Iden + Clone>,
) -> MnemeResult<()>
where
    C: ConnectionTrait,
{
    let scenario_col_name = spec.table_info.scenario_col;
    let entity_col_name = spec.table_info.entity_col;
    let field_col_name = spec.table_info.field_col;
    let valid_from_col_name = spec.table_info.valid_from_col;
    let valid_to_col_name = spec.table_info.valid_to_col;
    let layer_col_name = spec.layer_col;
    let asserted_col_name = spec.table_info.asserted_col;
    let op_id_col_name = spec.table_info.op_id_col;
    let is_tombstone_col_name = spec.is_tombstone_col;
    let value_col_name = spec.value_col;
    let scenario_col = Alias::new(scenario_col_name);
    let entity_col = Alias::new(entity_col_name);
    let field_col = Alias::new(field_col_name);
    let valid_from_col = Alias::new(valid_from_col_name);
    let valid_to_col = Alias::new(valid_to_col_name);
    let layer_col = Alias::new(layer_col_name);
    let asserted_col = Alias::new(asserted_col_name);
    let op_id_col = Alias::new(op_id_col_name);
    let is_tombstone_col = Alias::new(is_tombstone_col_name);
    let value_col = Alias::new(value_col_name);
    for field_id in spec.field_ids {
        let select = Query::select()
            .from(spec.table_info.table.clone())
            .columns([
                scenario_col.clone(),
                entity_col.clone(),
                field_col.clone(),
                valid_from_col.clone(),
                valid_to_col.clone(),
                layer_col.clone(),
                asserted_col.clone(),
                op_id_col.clone(),
                is_tombstone_col.clone(),
                value_col.clone(),
            ])
            .and_where(Expr::col(Alias::new("partition_id")).eq(id_value(
                spec.table_info.backend,
                spec.table_info.partition.0,
            )))
            .and_where(
                Expr::col(field_col.clone()).eq(id_value(spec.table_info.backend, *field_id)),
            )
            .order_by(scenario_col.clone(), Order::Asc)
            .order_by(entity_col.clone(), Order::Asc)
            .order_by(valid_from_col.clone(), Order::Asc)
            .order_by(valid_to_col.clone(), Order::Asc)
            .order_by(layer_col.clone(), Order::Asc)
            .order_by(asserted_col.clone(), Order::Desc)
            .order_by(op_id_col.clone(), Order::Desc)
            .to_owned();

        let rows = query_all(conn, &select).await?;
        let mut last_key: Option<PropFactKey> = None;
        for row in rows {
            let scenario_id = read_opt_id_by_name(&row, &col_name(scenario_col.clone()))?;
            let entity_id = read_id_by_name(&row, &col_name(entity_col.clone()))?;
            let field_id = read_id_by_name(&row, &col_name(field_col.clone()))?;
            let valid_from: i64 = row.try_get("", &col_name(valid_from_col.clone()))?;
            let valid_to: Option<i64> = row.try_get("", &col_name(valid_to_col.clone()))?;
            let layer: i64 = row.try_get("", &col_name(layer_col.clone()))?;
            let asserted_at: i64 = row.try_get("", &col_name(asserted_col.clone()))?;
            let op_id = read_id_by_name(&row, &col_name(op_id_col.clone()))?;
            let key = (
                scenario_id,
                entity_id,
                field_id,
                valid_from,
                valid_to,
                layer,
            );
            if last_key == Some(key) {
                delete_prop_fact_row(
                    conn,
                    &spec.table_info,
                    PropFactRowKey {
                        scenario_id,
                        entity_id,
                        field_id,
                        valid_from,
                        asserted_at,
                        op_id,
                    },
                )
                .await?;
            } else {
                last_key = Some(key);
            }
        }

        let select = Query::select()
            .from(spec.table_info.table.clone())
            .columns([
                scenario_col.clone(),
                entity_col.clone(),
                field_col.clone(),
                valid_from_col.clone(),
                valid_to_col.clone(),
                layer_col.clone(),
                asserted_col.clone(),
                op_id_col.clone(),
                is_tombstone_col.clone(),
                value_col.clone(),
            ])
            .and_where(Expr::col(Alias::new("partition_id")).eq(id_value(
                spec.table_info.backend,
                spec.table_info.partition.0,
            )))
            .and_where(
                Expr::col(field_col.clone()).eq(id_value(spec.table_info.backend, *field_id)),
            )
            .order_by(scenario_col.clone(), Order::Asc)
            .order_by(entity_col.clone(), Order::Asc)
            .order_by(layer_col.clone(), Order::Asc)
            .order_by(op_id_col.clone(), Order::Asc)
            .order_by(asserted_col.clone(), Order::Asc)
            .order_by(valid_from_col.clone(), Order::Asc)
            .to_owned();

        let rows = query_all(conn, &select).await?;
        let mut prev: Option<CompactionRow> = None;
        for row in rows {
            let scenario_id = read_opt_id_by_name(&row, &col_name(scenario_col.clone()))?;
            let entity_id = read_id_by_name(&row, &col_name(entity_col.clone()))?;
            let field_id = read_id_by_name(&row, &col_name(field_col.clone()))?;
            let valid_from: i64 = row.try_get("", &col_name(valid_from_col.clone()))?;
            let valid_to: Option<i64> = row.try_get("", &col_name(valid_to_col.clone()))?;
            let layer: i64 = row.try_get("", &col_name(layer_col.clone()))?;
            let asserted_at: i64 = row.try_get("", &col_name(asserted_col.clone()))?;
            let op_id = read_id_by_name(&row, &col_name(op_id_col.clone()))?;
            let is_tombstone: bool = row.try_get("", &col_name(is_tombstone_col.clone()))?;
            let value = read_value(spec.value_type, &row, &col_name(value_col.clone()))?;

            let current = CompactionRow {
                scenario_id,
                entity_id,
                field_id,
                valid_from,
                valid_to,
                layer,
                asserted_at,
                op_id,
                is_tombstone,
                value,
            };

            if let Some(mut previous) = prev.take() {
                let can_merge = previous.scenario_id == current.scenario_id
                    && previous.entity_id == current.entity_id
                    && previous.field_id == current.field_id
                    && previous.layer == current.layer
                    && previous.op_id == current.op_id
                    && previous.asserted_at == current.asserted_at
                    && previous.is_tombstone == current.is_tombstone
                    && previous.value == current.value
                    && previous.valid_to == Some(current.valid_from);
                if can_merge {
                    previous.valid_to = current.valid_to;
                    update_prop_fact_valid_to(
                        conn,
                        &spec.table_info,
                        PropFactRowKey {
                            scenario_id: previous.scenario_id,
                            entity_id: previous.entity_id,
                            field_id: previous.field_id,
                            valid_from: previous.valid_from,
                            asserted_at: previous.asserted_at,
                            op_id: previous.op_id,
                        },
                        previous.valid_to,
                    )
                    .await?;
                    delete_prop_fact_row(
                        conn,
                        &spec.table_info,
                        PropFactRowKey {
                            scenario_id: current.scenario_id,
                            entity_id: current.entity_id,
                            field_id: current.field_id,
                            valid_from: current.valid_from,
                            asserted_at: current.asserted_at,
                            op_id: current.op_id,
                        },
                    )
                    .await?;
                    prev = Some(previous);
                    continue;
                }
            }
            prev = Some(current);
        }
    }
    Ok(())
}

async fn delete_prop_fact_row<C, T: sea_query::Iden + Clone>(
    conn: &C,
    table_info: &PropFactTableInfo<'_, T>,
    row_key: PropFactRowKey,
) -> MnemeResult<()>
where
    C: ConnectionTrait,
{
    let scenario_col = Alias::new(table_info.scenario_col);
    let entity_col = Alias::new(table_info.entity_col);
    let field_col = Alias::new(table_info.field_col);
    let valid_from_col = Alias::new(table_info.valid_from_col);
    let asserted_col = Alias::new(table_info.asserted_col);
    let op_id_col = Alias::new(table_info.op_id_col);
    let mut delete = Query::delete()
        .from_table(table_info.table.clone())
        .and_where(
            Expr::col(Alias::new("partition_id"))
                .eq(id_value(table_info.backend, table_info.partition.0)),
        )
        .and_where(Expr::col(entity_col).eq(id_value(table_info.backend, row_key.entity_id)))
        .and_where(Expr::col(field_col).eq(id_value(table_info.backend, row_key.field_id)))
        .and_where(Expr::col(valid_from_col).eq(row_key.valid_from))
        .and_where(Expr::col(asserted_col).eq(row_key.asserted_at))
        .and_where(Expr::col(op_id_col).eq(id_value(table_info.backend, row_key.op_id)))
        .to_owned();
    if let Some(scenario_id) = row_key.scenario_id {
        delete.and_where(Expr::col(scenario_col).eq(id_value(table_info.backend, scenario_id)));
    } else {
        delete.and_where(Expr::col(scenario_col).is_null());
    }
    exec(conn, &delete).await?;
    Ok(())
}

async fn update_prop_fact_valid_to<C, T: sea_query::Iden + Clone>(
    conn: &C,
    table_info: &PropFactTableInfo<'_, T>,
    row_key: PropFactRowKey,
    valid_to: Option<i64>,
) -> MnemeResult<()>
where
    C: ConnectionTrait,
{
    let scenario_col = Alias::new(table_info.scenario_col);
    let entity_col = Alias::new(table_info.entity_col);
    let field_col = Alias::new(table_info.field_col);
    let valid_from_col = Alias::new(table_info.valid_from_col);
    let valid_to_col = Alias::new(table_info.valid_to_col);
    let asserted_col = Alias::new(table_info.asserted_col);
    let op_id_col = Alias::new(table_info.op_id_col);
    let mut update = Query::update()
        .table(table_info.table.clone())
        .values([(valid_to_col, valid_to.into())])
        .and_where(
            Expr::col(Alias::new("partition_id"))
                .eq(id_value(table_info.backend, table_info.partition.0)),
        )
        .and_where(Expr::col(entity_col).eq(id_value(table_info.backend, row_key.entity_id)))
        .and_where(Expr::col(field_col).eq(id_value(table_info.backend, row_key.field_id)))
        .and_where(Expr::col(valid_from_col).eq(row_key.valid_from))
        .and_where(Expr::col(asserted_col).eq(row_key.asserted_at))
        .and_where(Expr::col(op_id_col).eq(id_value(table_info.backend, row_key.op_id)))
        .to_owned();
    if let Some(scenario_id) = row_key.scenario_id {
        update.and_where(Expr::col(scenario_col).eq(id_value(table_info.backend, scenario_id)));
    } else {
        update.and_where(Expr::col(scenario_col).is_null());
    }
    exec(conn, &update).await?;
    Ok(())
}

async fn compact_edge_exists_facts<C>(
    conn: &C,
    backend: DatabaseBackend,
    partition: PartitionId,
) -> MnemeResult<()>
where
    C: ConnectionTrait,
{
    let table_info = EdgeExistsTableInfo { backend, partition };
    let select = Query::select()
        .from(AideonEdgeExistsFacts::Table)
        .columns([
            AideonEdgeExistsFacts::ScenarioId,
            AideonEdgeExistsFacts::EdgeId,
            AideonEdgeExistsFacts::ValidFrom,
            AideonEdgeExistsFacts::ValidTo,
            AideonEdgeExistsFacts::Layer,
            AideonEdgeExistsFacts::AssertedAtHlc,
            AideonEdgeExistsFacts::OpId,
            AideonEdgeExistsFacts::IsTombstone,
        ])
        .and_where(Expr::col(AideonEdgeExistsFacts::PartitionId).eq(id_value(backend, partition.0)))
        .order_by(AideonEdgeExistsFacts::ScenarioId, Order::Asc)
        .order_by(AideonEdgeExistsFacts::EdgeId, Order::Asc)
        .order_by(AideonEdgeExistsFacts::ValidFrom, Order::Asc)
        .order_by(AideonEdgeExistsFacts::ValidTo, Order::Asc)
        .order_by(AideonEdgeExistsFacts::Layer, Order::Asc)
        .order_by(AideonEdgeExistsFacts::AssertedAtHlc, Order::Desc)
        .order_by(AideonEdgeExistsFacts::OpId, Order::Desc)
        .to_owned();
    let rows = query_all(conn, &select).await?;
    let mut last_key: Option<EdgeExistsKey> = None;
    for row in rows {
        let scenario_id = read_opt_id_by_name(&row, &col_name(AideonEdgeExistsFacts::ScenarioId))?;
        let edge_id = read_id(&row, AideonEdgeExistsFacts::EdgeId)?;
        let valid_from: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::ValidFrom))?;
        let valid_to: Option<i64> = row.try_get("", &col_name(AideonEdgeExistsFacts::ValidTo))?;
        let layer: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::Layer))?;
        let asserted_at: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::AssertedAtHlc))?;
        let op_id = read_id(&row, AideonEdgeExistsFacts::OpId)?;
        let key = (scenario_id, edge_id, valid_from, valid_to, layer);
        if last_key == Some(key) {
            delete_edge_exists_row(
                conn,
                &table_info,
                EdgeExistsRowKey {
                    scenario_id,
                    edge_id,
                    valid_from,
                    asserted_at,
                    op_id,
                },
            )
            .await?;
        } else {
            last_key = Some(key);
        }
    }

    let select = Query::select()
        .from(AideonEdgeExistsFacts::Table)
        .columns([
            AideonEdgeExistsFacts::ScenarioId,
            AideonEdgeExistsFacts::EdgeId,
            AideonEdgeExistsFacts::ValidFrom,
            AideonEdgeExistsFacts::ValidTo,
            AideonEdgeExistsFacts::Layer,
            AideonEdgeExistsFacts::AssertedAtHlc,
            AideonEdgeExistsFacts::OpId,
            AideonEdgeExistsFacts::IsTombstone,
        ])
        .and_where(Expr::col(AideonEdgeExistsFacts::PartitionId).eq(id_value(backend, partition.0)))
        .order_by(AideonEdgeExistsFacts::ScenarioId, Order::Asc)
        .order_by(AideonEdgeExistsFacts::EdgeId, Order::Asc)
        .order_by(AideonEdgeExistsFacts::Layer, Order::Asc)
        .order_by(AideonEdgeExistsFacts::OpId, Order::Asc)
        .order_by(AideonEdgeExistsFacts::AssertedAtHlc, Order::Asc)
        .order_by(AideonEdgeExistsFacts::ValidFrom, Order::Asc)
        .to_owned();
    let rows = query_all(conn, &select).await?;
    let mut prev: Option<EdgeExistsMergeKey> = None;
    for row in rows {
        let scenario_id = read_opt_id_by_name(&row, &col_name(AideonEdgeExistsFacts::ScenarioId))?;
        let edge_id = read_id(&row, AideonEdgeExistsFacts::EdgeId)?;
        let valid_from: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::ValidFrom))?;
        let valid_to: Option<i64> = row.try_get("", &col_name(AideonEdgeExistsFacts::ValidTo))?;
        let layer: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::Layer))?;
        let asserted_at: i64 = row.try_get("", &col_name(AideonEdgeExistsFacts::AssertedAtHlc))?;
        let op_id = read_id(&row, AideonEdgeExistsFacts::OpId)?;
        let is_tombstone: bool = row.try_get("", &col_name(AideonEdgeExistsFacts::IsTombstone))?;
        let key: EdgeExistsMergeKey = (
            scenario_id,
            edge_id,
            valid_from,
            valid_to,
            layer,
            asserted_at,
            op_id,
            is_tombstone,
        );
        if let Some((
            prev_scenario,
            prev_edge,
            prev_from,
            prev_to,
            prev_layer,
            prev_asserted,
            prev_op,
            prev_tombstone,
        )) = prev.take()
        {
            let can_merge = prev_scenario == scenario_id
                && prev_edge == edge_id
                && prev_layer == layer
                && prev_op == op_id
                && prev_asserted == asserted_at
                && prev_tombstone == is_tombstone
                && prev_to == Some(valid_from);
            if can_merge {
                update_edge_exists_valid_to(
                    conn,
                    &table_info,
                    EdgeExistsRowKey {
                        scenario_id: prev_scenario,
                        edge_id: prev_edge,
                        valid_from: prev_from,
                        asserted_at: prev_asserted,
                        op_id: prev_op,
                    },
                    valid_to,
                )
                .await?;
                delete_edge_exists_row(
                    conn,
                    &table_info,
                    EdgeExistsRowKey {
                        scenario_id,
                        edge_id,
                        valid_from,
                        asserted_at,
                        op_id,
                    },
                )
                .await?;
                prev = Some((
                    prev_scenario,
                    prev_edge,
                    prev_from,
                    valid_to,
                    prev_layer,
                    prev_asserted,
                    prev_op,
                    prev_tombstone,
                ));
                continue;
            }
        }
        prev = Some(key);
    }
    Ok(())
}

async fn delete_edge_exists_row<C>(
    conn: &C,
    table_info: &EdgeExistsTableInfo,
    row_key: EdgeExistsRowKey,
) -> MnemeResult<()>
where
    C: ConnectionTrait,
{
    let mut delete = Query::delete()
        .from_table(AideonEdgeExistsFacts::Table)
        .and_where(
            Expr::col(AideonEdgeExistsFacts::PartitionId)
                .eq(id_value(table_info.backend, table_info.partition.0)),
        )
        .and_where(
            Expr::col(AideonEdgeExistsFacts::EdgeId)
                .eq(id_value(table_info.backend, row_key.edge_id)),
        )
        .and_where(Expr::col(AideonEdgeExistsFacts::ValidFrom).eq(row_key.valid_from))
        .and_where(Expr::col(AideonEdgeExistsFacts::AssertedAtHlc).eq(row_key.asserted_at))
        .and_where(
            Expr::col(AideonEdgeExistsFacts::OpId).eq(id_value(table_info.backend, row_key.op_id)),
        )
        .to_owned();
    if let Some(scenario_id) = row_key.scenario_id {
        delete.and_where(
            Expr::col(AideonEdgeExistsFacts::ScenarioId)
                .eq(id_value(table_info.backend, scenario_id)),
        );
    } else {
        delete.and_where(Expr::col(AideonEdgeExistsFacts::ScenarioId).is_null());
    }
    exec(conn, &delete).await?;
    Ok(())
}

async fn update_edge_exists_valid_to<C>(
    conn: &C,
    table_info: &EdgeExistsTableInfo,
    row_key: EdgeExistsRowKey,
    valid_to: Option<i64>,
) -> MnemeResult<()>
where
    C: ConnectionTrait,
{
    let mut update = Query::update()
        .table(AideonEdgeExistsFacts::Table)
        .values([(AideonEdgeExistsFacts::ValidTo, valid_to.into())])
        .and_where(
            Expr::col(AideonEdgeExistsFacts::PartitionId)
                .eq(id_value(table_info.backend, table_info.partition.0)),
        )
        .and_where(
            Expr::col(AideonEdgeExistsFacts::EdgeId)
                .eq(id_value(table_info.backend, row_key.edge_id)),
        )
        .and_where(Expr::col(AideonEdgeExistsFacts::ValidFrom).eq(row_key.valid_from))
        .and_where(Expr::col(AideonEdgeExistsFacts::AssertedAtHlc).eq(row_key.asserted_at))
        .and_where(
            Expr::col(AideonEdgeExistsFacts::OpId).eq(id_value(table_info.backend, row_key.op_id)),
        )
        .to_owned();
    if let Some(scenario_id) = row_key.scenario_id {
        update.and_where(
            Expr::col(AideonEdgeExistsFacts::ScenarioId)
                .eq(id_value(table_info.backend, scenario_id)),
        );
    } else {
        update.and_where(Expr::col(AideonEdgeExistsFacts::ScenarioId).is_null());
    }
    exec(conn, &update).await?;
    Ok(())
}

async fn partition_has_rows(
    conn: &DatabaseConnection,
    partition: PartitionId,
    backend: DatabaseBackend,
) -> MnemeResult<bool> {
    let mut select = Query::select()
        .expr(Expr::val(1))
        .from(AideonOps::Table)
        .and_where(Expr::col(AideonOps::PartitionId).eq(id_value(backend, partition.0)))
        .limit(1)
        .to_owned();
    if query_one(conn, &select).await?.is_some() {
        return Ok(true);
    }
    select = Query::select()
        .expr(Expr::val(1))
        .from(AideonEntities::Table)
        .and_where(Expr::col(AideonEntities::PartitionId).eq(id_value(backend, partition.0)))
        .limit(1)
        .to_owned();
    Ok(query_one(conn, &select).await?.is_some())
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

fn payload_scenario_id(payload: &OpPayload) -> Option<crate::ScenarioId> {
    match payload {
        OpPayload::CreateNode(input) => input.scenario_id,
        OpPayload::CreateEdge(input) => input.scenario_id,
        OpPayload::SetEdgeExistenceInterval(input) => input.scenario_id,
        OpPayload::TombstoneEntity { scenario_id, .. } => *scenario_id,
        OpPayload::SetProperty(input) => input.scenario_id,
        OpPayload::ClearProperty(input) => input.scenario_id,
        OpPayload::OrSetUpdate(input) => input.scenario_id,
        OpPayload::CounterUpdate(input) => input.scenario_id,
        OpPayload::UpsertMetamodelBatch(_) => None,
        OpPayload::CreateScenario { scenario_id, .. } => Some(*scenario_id),
        OpPayload::DeleteScenario { scenario_id, .. } => Some(*scenario_id),
    }
}

fn payload_bulk_mode(payload: &OpPayload) -> bool {
    match payload {
        OpPayload::CreateNode(input) => input
            .write_options
            .map(|opts| opts.bulk_mode)
            .unwrap_or(false),
        OpPayload::CreateEdge(input) => input
            .write_options
            .map(|opts| opts.bulk_mode)
            .unwrap_or(false),
        OpPayload::SetEdgeExistenceInterval(input) => input
            .write_options
            .map(|opts| opts.bulk_mode)
            .unwrap_or(false),
        OpPayload::SetProperty(input) => input
            .write_options
            .map(|opts| opts.bulk_mode)
            .unwrap_or(false),
        OpPayload::ClearProperty(input) => input
            .write_options
            .map(|opts| opts.bulk_mode)
            .unwrap_or(false),
        OpPayload::OrSetUpdate(input) => input
            .write_options
            .map(|opts| opts.bulk_mode)
            .unwrap_or(false),
        OpPayload::CounterUpdate(input) => input
            .write_options
            .map(|opts| opts.bulk_mode)
            .unwrap_or(false),
        OpPayload::TombstoneEntity { .. }
        | OpPayload::UpsertMetamodelBatch(_)
        | OpPayload::CreateScenario { .. }
        | OpPayload::DeleteScenario { .. } => false,
    }
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
    backend: DatabaseBackend,
    input: PropertyFactInsertInput<'_>,
) -> MnemeResult<()> {
    let value_type = input.value.value_type();
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
            id_value(backend, input.partition.0).into(),
            opt_id_value(backend, input.scenario_id.map(|s| s.0)).into(),
            id_value(backend, input.entity_id).into(),
            id_value(backend, input.field_id).into(),
            input.valid_from.0.into(),
            input.valid_to.map(|v| v.0).into(),
            valid_bucket(input.valid_from).into(),
            (input.layer as i64).into(),
            input.asserted_at.as_i64().into(),
            id_value(backend, input.op_id.0).into(),
            input.is_tombstone.into(),
            value_to_sea(backend, input.value).into(),
        ])
        .to_owned();
    exec(tx, &insert).await
}

async fn insert_index_row(
    tx: &sea_orm::DatabaseTransaction,
    backend: DatabaseBackend,
    input: IndexInsertInput<'_>,
) -> MnemeResult<()> {
    match input.value {
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
                    id_value(backend, input.partition.0).into(),
                    opt_id_value(backend, input.scenario_id.map(|s| s.0)).into(),
                    id_value(backend, input.field_id).into(),
                    norm.into(),
                    id_value(backend, input.entity_id).into(),
                    input.valid_from.0.into(),
                    input.valid_to.map(|v| v.0).into(),
                    valid_bucket(input.valid_from).into(),
                    input.asserted_at.as_i64().into(),
                    (input.layer as i64).into(),
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
                    id_value(backend, input.partition.0).into(),
                    opt_id_value(backend, input.scenario_id.map(|s| s.0)).into(),
                    id_value(backend, input.field_id).into(),
                    (*value).into(),
                    id_value(backend, input.entity_id).into(),
                    input.valid_from.0.into(),
                    input.valid_to.map(|v| v.0).into(),
                    valid_bucket(input.valid_from).into(),
                    input.asserted_at.as_i64().into(),
                    (input.layer as i64).into(),
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
                    id_value(backend, input.partition.0).into(),
                    opt_id_value(backend, input.scenario_id.map(|s| s.0)).into(),
                    id_value(backend, input.field_id).into(),
                    (*value).into(),
                    id_value(backend, input.entity_id).into(),
                    input.valid_from.0.into(),
                    input.valid_to.map(|v| v.0).into(),
                    valid_bucket(input.valid_from).into(),
                    input.asserted_at.as_i64().into(),
                    (input.layer as i64).into(),
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
                    id_value(backend, input.partition.0).into(),
                    opt_id_value(backend, input.scenario_id.map(|s| s.0)).into(),
                    id_value(backend, input.field_id).into(),
                    (*value).into(),
                    id_value(backend, input.entity_id).into(),
                    input.valid_from.0.into(),
                    input.valid_to.map(|v| v.0).into(),
                    valid_bucket(input.valid_from).into(),
                    input.asserted_at.as_i64().into(),
                    (input.layer as i64).into(),
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
                    id_value(backend, input.partition.0).into(),
                    opt_id_value(backend, input.scenario_id.map(|s| s.0)).into(),
                    id_value(backend, input.field_id).into(),
                    value.0.into(),
                    id_value(backend, input.entity_id).into(),
                    input.valid_from.0.into(),
                    input.valid_to.map(|v| v.0).into(),
                    valid_bucket(input.valid_from).into(),
                    input.asserted_at.as_i64().into(),
                    (input.layer as i64).into(),
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
                    id_value(backend, input.partition.0).into(),
                    opt_id_value(backend, input.scenario_id.map(|s| s.0)).into(),
                    id_value(backend, input.field_id).into(),
                    id_value(backend, *value).into(),
                    id_value(backend, input.entity_id).into(),
                    input.valid_from.0.into(),
                    input.valid_to.map(|v| v.0).into(),
                    valid_bucket(input.valid_from).into(),
                    input.asserted_at.as_i64().into(),
                    (input.layer as i64).into(),
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
                .and_where(
                    Expr::col(AideonIdxFieldStr::PartitionId).eq(id_value(backend, partition.0)),
                )
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
                .and_where(
                    Expr::col(AideonIdxFieldI64::PartitionId).eq(id_value(backend, partition.0)),
                )
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
                .and_where(
                    Expr::col(AideonIdxFieldF64::PartitionId).eq(id_value(backend, partition.0)),
                )
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
                .and_where(
                    Expr::col(AideonIdxFieldBool::PartitionId).eq(id_value(backend, partition.0)),
                )
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
                .and_where(
                    Expr::col(AideonIdxFieldTime::PartitionId).eq(id_value(backend, partition.0)),
                )
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
                .and_where(
                    Expr::col(AideonIdxFieldRef::PartitionId).eq(id_value(backend, partition.0)),
                )
                .and_where(Expr::col(AideonIdxFieldRef::FieldId).eq(id_value(backend, field_id)))
                .and_where(Expr::col(AideonIdxFieldRef::EntityId).eq(id_value(backend, entity_id)))
                .and_where(
                    Expr::col(AideonIdxFieldRef::ValueRefEntityId).eq(id_value(backend, *value)),
                )
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

fn default_value_for_type(value_type: ValueType) -> Value {
    match value_type {
        ValueType::Str => Value::Str(String::new()),
        ValueType::I64 => Value::I64(0),
        ValueType::F64 => Value::F64(0.0),
        ValueType::Bool => Value::Bool(false),
        ValueType::Time => Value::Time(ValidTime(0)),
        ValueType::Ref => Value::Ref(Id::from_bytes([0u8; 16])),
        ValueType::Blob => Value::Blob(Vec::new()),
        ValueType::Json => Value::Json(serde_json::Value::Null),
    }
}

async fn fetch_property_facts_with_fallback(
    conn: &DatabaseConnection,
    input: PropertyFactQueryInput,
    backend: DatabaseBackend,
) -> MnemeResult<Vec<PropertyFact>> {
    let facts = fetch_property_facts_in_partition(conn, &input, backend).await?;
    if !facts.is_empty() {
        return Ok(facts);
    }
    if input.scenario_id.is_some() {
        let fallback = PropertyFactQueryInput {
            scenario_id: None,
            ..input
        };
        return fetch_property_facts_in_partition(conn, &fallback, backend).await;
    }
    Ok(facts)
}

async fn fetch_property_facts_in_partition(
    conn: &DatabaseConnection,
    input: &PropertyFactQueryInput,
    backend: DatabaseBackend,
) -> MnemeResult<Vec<PropertyFact>> {
    let (table, column) = value_table_column(input.value_type);
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
                .eq(id_value(backend, input.partition.0)),
        )
        .and_where(
            Expr::col((table.clone(), Alias::new("entity_id")))
                .eq(id_value(backend, input.entity_id)),
        )
        .and_where(
            Expr::col((table.clone(), Alias::new("field_id")))
                .eq(id_value(backend, input.field_id)),
        )
        .and_where(Expr::col((table.clone(), Alias::new("valid_from"))).lte(input.at_valid_time.0))
        .and_where(
            Expr::col((table.clone(), Alias::new("valid_to")))
                .gt(input.at_valid_time.0)
                .or(Expr::col((table.clone(), Alias::new("valid_to"))).is_null()),
        )
        .to_owned();
    if let Some(scenario_id) = input.scenario_id {
        select.and_where(
            Expr::col((table.clone(), Alias::new("scenario_id")))
                .eq(id_value(backend, scenario_id.0)),
        );
    } else {
        select.and_where(Expr::col((table.clone(), Alias::new("scenario_id"))).is_null());
    }
    if let Some(as_of) = input.as_of_asserted_at {
        select.and_where(
            Expr::col((table.clone(), Alias::new("asserted_at_hlc"))).lte(as_of.as_i64()),
        );
    }
    let rows = query_all(conn, &select).await?;
    let mut facts = Vec::new();
    for row in rows {
        let value = read_value(input.value_type, &row, &col_name(column.clone()))?;
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
            Expr::col((
                AideonEdgeExistsFacts::Table,
                AideonEdgeExistsFacts::ScenarioId,
            ))
            .eq(id_value(backend, scenario_id.0)),
        );
    } else {
        select.and_where(Expr::col((AideonEdges::Table, AideonEdges::ScenarioId)).is_null());
        select.and_where(
            Expr::col((
                AideonEdgeExistsFacts::Table,
                AideonEdgeExistsFacts::ScenarioId,
            ))
            .is_null(),
        );
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

async fn fetch_edge_facts_for_edge(
    conn: &DatabaseConnection,
    partition: PartitionId,
    scenario_id: Option<ScenarioId>,
    edge_id: Id,
    at_valid_time: ValidTime,
    as_of_asserted_at: Option<Hlc>,
    backend: DatabaseBackend,
) -> MnemeResult<Vec<EdgeFact>> {
    let facts = fetch_edge_facts_for_edge_in_partition(
        conn,
        partition,
        scenario_id,
        edge_id,
        at_valid_time,
        as_of_asserted_at,
        backend,
    )
    .await?;
    if facts.is_empty() && scenario_id.is_some() {
        return fetch_edge_facts_for_edge_in_partition(
            conn,
            partition,
            None,
            edge_id,
            at_valid_time,
            as_of_asserted_at,
            backend,
        )
        .await;
    }
    Ok(facts)
}

async fn fetch_edge_facts_for_edge_in_partition(
    conn: &DatabaseConnection,
    partition: PartitionId,
    scenario_id: Option<ScenarioId>,
    edge_id: Id,
    at_valid_time: ValidTime,
    as_of_asserted_at: Option<Hlc>,
    backend: DatabaseBackend,
) -> MnemeResult<Vec<EdgeFact>> {
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
            Expr::col((AideonEdges::Table, AideonEdges::EdgeId)).eq(id_value(backend, edge_id)),
        )
        .and_where(
            Expr::col((
                AideonEdgeExistsFacts::Table,
                AideonEdgeExistsFacts::ValidFrom,
            ))
            .lte(at_valid_time.0),
        )
        .and_where(
            Expr::col((AideonEdgeExistsFacts::Table, AideonEdgeExistsFacts::ValidTo))
                .gt(at_valid_time.0)
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
            Expr::col((
                AideonEdgeExistsFacts::Table,
                AideonEdgeExistsFacts::ScenarioId,
            ))
            .eq(id_value(backend, scenario_id.0)),
        );
    } else {
        select.and_where(Expr::col((AideonEdges::Table, AideonEdges::ScenarioId)).is_null());
        select.and_where(
            Expr::col((
                AideonEdgeExistsFacts::Table,
                AideonEdgeExistsFacts::ScenarioId,
            ))
            .is_null(),
        );
    }
    if let Some(as_of) = as_of_asserted_at {
        select.and_where(
            Expr::col((
                AideonEdgeExistsFacts::Table,
                AideonEdgeExistsFacts::AssertedAtHlc,
            ))
            .lte(as_of.as_i64()),
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
            Expr::col(AideonGraphProjectionEdges::ScenarioId).eq(id_value(backend, scenario_id.0)),
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

async fn list_entities_without_filters(
    store: &MnemeStore,
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
        if let Some(cursor) = input.cursor.as_deref() {
            let cursor_id = crate::decode_entity_cursor(cursor)?;
            select.and_where(Expr::col(AideonEntities::EntityId).gt(id_value(backend, cursor_id)));
        }
        if let Some(scenario_id) = scenario_id {
            select.and_where(
                Expr::col(AideonEntities::ScenarioId).eq(id_value(backend, scenario_id.0)),
            );
        } else {
            select.and_where(Expr::col(AideonEntities::ScenarioId).is_null());
        }
        if let Some(kind) = input.kind {
            select.and_where(Expr::col(AideonEntities::EntityKind).eq(kind.as_i16() as i64));
        }
        if let Some(type_id) = input.type_id {
            select.and_where(Expr::col(AideonEntities::TypeId).eq(id_value(backend, type_id)));
        }
        select.order_by(AideonEntities::EntityId, Order::Asc);
        let rows = query_all(&store.conn, &select).await?;
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
            if let Some(ctx) = &input.security_context
                && let Some(security) = store
                    .read_entity_security_with_fallback(
                        input.partition,
                        input.scenario_id,
                        entity_id,
                    )
                    .await?
                && !MnemeStore::is_entity_visible(ctx, &security)
            {
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
                        Expr::col(AideonIdxFieldStr::ValueTextNorm).like(format!("{norm}%")),
                    );
                }
                CompareOp::Contains => {
                    select.and_where(
                        Expr::col(AideonIdxFieldStr::ValueTextNorm).like(format!("%{norm}%")),
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
            return Err(MnemeError::invalid(
                "indexed filters not supported for blob/json",
            ));
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
        ReadValue::MultiLimited {
            values,
            more_available,
        } => {
            if filter.op == CompareOp::Ne {
                for value in &values {
                    if !compare_value(value, filter)? {
                        return Ok(false);
                    }
                }
                if more_available { Ok(false) } else { Ok(true) }
            } else {
                for value in &values {
                    if compare_value(value, filter)? {
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

fn read_opt_id_by_name(row: &QueryResult, column: &str) -> MnemeResult<Option<Id>> {
    if let Ok(value) = row.try_get::<Option<String>>("", column) {
        return value.map(|value| Id::from_uuid_str(&value)).transpose();
    }
    if let Ok(value) = row.try_get::<Option<Uuid>>("", column) {
        return Ok(value.map(|value| Id::from_bytes(*value.as_bytes())));
    }
    if let Ok(value) = row.try_get::<Option<Vec<u8>>>("", column) {
        return Ok(value.and_then(bytes_to_id));
    }
    Ok(None)
}

fn read_hlc_by_name(row: &QueryResult, column: &str) -> MnemeResult<Hlc> {
    let value: i64 = row.try_get("", column)?;
    Ok(Hlc::from_i64(value))
}

impl MnemeStore {
    async fn build_effective_schema(
        &self,
        partition: PartitionId,
        type_id: Id,
    ) -> MnemeResult<EffectiveSchema> {
        self.build_effective_schema_with_conn(&self.conn, partition, type_id)
            .await
    }

    async fn build_effective_schema_with_conn<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        type_id: Id,
    ) -> MnemeResult<EffectiveSchema> {
        let lineage = self.type_lineage(conn, partition, type_id).await?;
        let mut applies_to = None;
        for lineage_type in &lineage {
            let meta = self
                .fetch_type_meta(conn, partition, *lineage_type)
                .await?
                .ok_or_else(|| MnemeError::not_found("type not found"))?;
            if let Some(existing) = applies_to {
                if existing != meta.applies_to {
                    self.validate_or_warn(false, "type inheritance applies_to mismatch")?;
                }
            } else {
                applies_to = Some(meta.applies_to);
            }
        }
        let applies_to = applies_to.ok_or_else(|| MnemeError::not_found("type not found"))?;

        let mut fields_by_id: HashMap<Id, EffectiveField> = HashMap::new();
        for lineage_type in lineage {
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
                .column((AideonFields::Table, AideonFields::DisallowOverlap))
                .column((AideonTypeFields::Table, AideonTypeFields::IsRequired))
                .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueStr))
                .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueI64))
                .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueF64))
                .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueBool))
                .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueTime))
                .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueRef))
                .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueBlob))
                .column((AideonTypeFields::Table, AideonTypeFields::DefaultValueJson))
                .column((AideonTypeFields::Table, AideonTypeFields::OverrideDefault))
                .column((AideonTypeFields::Table, AideonTypeFields::TightenRequired))
                .column((AideonTypeFields::Table, AideonTypeFields::DisallowOverlap))
                .and_where(
                    Expr::col((AideonTypeFields::Table, AideonTypeFields::PartitionId))
                        .eq(id_value(self.backend, partition.0)),
                )
                .and_where(
                    Expr::col((AideonTypeFields::Table, AideonTypeFields::TypeId))
                        .eq(id_value(self.backend, lineage_type)),
                )
                .and_where(Expr::col((AideonFields::Table, AideonFields::IsDeleted)).eq(false))
                .to_owned();
            let rows = query_all(conn, &select_fields).await?;
            for row in rows {
                let field_id = read_id(&row, AideonTypeFields::FieldId)?;
                let value_type_raw: i16 = row.try_get("", &col_name(AideonFields::ValueType))?;
                let merge_policy_raw: i16 =
                    row.try_get("", &col_name(AideonFields::MergePolicy))?;
                let cardinality_raw: i16 = row.try_get("", &col_name(AideonFields::Cardinality))?;
                let value_type = ValueType::from_i16(value_type_raw)
                    .ok_or_else(|| MnemeError::storage("invalid value type"))?;
                let merge_policy = MergePolicy::from_i16(merge_policy_raw)
                    .ok_or_else(|| MnemeError::storage("invalid merge policy"))?;
                let cardinality_multi = cardinality_raw == 2;
                let is_indexed: bool = row.try_get("", &col_name(AideonFields::IsIndexed))?;
                let base_disallow: Option<bool> =
                    row.try_get("", &col_name(AideonFields::DisallowOverlap))?;
                let is_required: bool = row.try_get("", &col_name(AideonTypeFields::IsRequired))?;
                let override_default: bool =
                    row.try_get("", &col_name(AideonTypeFields::OverrideDefault))?;
                let tighten_required: bool =
                    row.try_get("", &col_name(AideonTypeFields::TightenRequired))?;
                let override_disallow: Option<bool> =
                    row.try_get("", &col_name(AideonTypeFields::DisallowOverlap))?;
                let disallow_overlap = override_disallow.unwrap_or(base_disallow.unwrap_or(false));
                let default_value = match value_type {
                    ValueType::Str => row
                        .try_get::<Option<String>>(
                            "",
                            &col_name(AideonTypeFields::DefaultValueStr),
                        )?
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
                    ValueType::Ref => {
                        read_opt_id(&row, AideonTypeFields::DefaultValueRef)?.map(Value::Ref)
                    }
                    ValueType::Blob => row
                        .try_get::<Option<Vec<u8>>>(
                            "",
                            &col_name(AideonTypeFields::DefaultValueBlob),
                        )?
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

                if let Some(existing) = fields_by_id.get_mut(&field_id) {
                    if existing.value_type != value_type
                        || existing.merge_policy != merge_policy
                        || existing.cardinality_multi != cardinality_multi
                    {
                        self.validate_or_warn(
                            false,
                            "field definition mismatch across inheritance",
                        )?;
                    }
                    if tighten_required {
                        existing.is_required = existing.is_required || is_required;
                    } else if is_required && !existing.is_required {
                        self.validate_or_warn(false, "required flag tightens without permission")?;
                    }
                    if override_default || existing.default_value.is_none() {
                        existing.default_value = default_value;
                    }
                    if override_disallow.is_some() {
                        existing.disallow_overlap = disallow_overlap;
                    }
                    existing.is_indexed = existing.is_indexed || is_indexed;
                } else {
                    fields_by_id.insert(
                        field_id,
                        EffectiveField {
                            field_id,
                            value_type,
                            cardinality_multi,
                            merge_policy,
                            is_required,
                            default_value,
                            is_indexed,
                            disallow_overlap,
                        },
                    );
                }
            }
        }

        let mut fields: Vec<EffectiveField> = fields_by_id.into_values().collect();
        fields.sort_by(|a, b| a.field_id.as_bytes().cmp(&b.field_id.as_bytes()));
        Ok(EffectiveSchema {
            type_id,
            applies_to,
            fields,
        })
    }

    async fn compile_effective_schema_with_conn<C: ConnectionTrait>(
        &self,
        conn: &C,
        partition: PartitionId,
        asserted_at: Hlc,
        type_id: Id,
    ) -> MnemeResult<SchemaVersion> {
        let schema = self
            .build_effective_schema_with_conn(conn, partition, type_id)
            .await?;
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
        exec(conn, &insert).await?;
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
        exec(conn, &update_head).await?;
        Ok(SchemaVersion {
            schema_version_hash: hash,
        })
    }
}

fn resolve_property(
    merge_policy: MergePolicy,
    mut facts: Vec<PropertyFact>,
    limits: &MnemeLimits,
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
            let mut values = facts
                .into_iter()
                .filter(|fact| fact.layer == top_layer && interval_width(fact) == top_width)
                .filter(|fact| !fact.is_tombstone)
                .map(|fact| fact.value)
                .collect::<Vec<_>>();
            if values.is_empty() {
                return Ok(None);
            }
            let more_available = values.len() > limits.max_mv_values;
            if more_available {
                values.truncate(limits.max_mv_values);
            }
            Ok(Some(ReadValue::MultiLimited {
                values,
                more_available,
            }))
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
            let more_available = values.len() > limits.max_mv_values;
            if more_available {
                values.truncate(limits.max_mv_values);
            }
            Ok(Some(ReadValue::MultiLimited {
                values,
                more_available,
            }))
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

fn explain_property_fact(fact: &PropertyFact) -> crate::api::ExplainPropertyFact {
    let precedence = crate::api::ExplainPrecedence {
        layer: fact.layer,
        interval_width: interval_width(fact),
        asserted_at: fact.asserted_at,
        op_id: fact.op_id,
    };
    crate::api::ExplainPropertyFact {
        value: fact.value.clone(),
        valid_from: fact.valid_from,
        valid_to: fact.valid_to,
        layer: fact.layer,
        asserted_at: fact.asserted_at,
        op_id: fact.op_id,
        is_tombstone: fact.is_tombstone,
        precedence,
    }
}

fn explain_edge_fact(fact: &EdgeFact) -> crate::api::ExplainEdgeFact {
    let precedence = crate::api::ExplainPrecedence {
        layer: fact.layer,
        interval_width: interval_width_edge(fact),
        asserted_at: fact.asserted_at,
        op_id: fact.op_id,
    };
    crate::api::ExplainEdgeFact {
        edge_id: fact.edge_id,
        src_id: fact.src_id,
        dst_id: fact.dst_id,
        edge_type_id: fact.edge_type_id,
        valid_from: fact.valid_from,
        valid_to: fact.valid_to,
        layer: fact.layer,
        asserted_at: fact.asserted_at,
        op_id: fact.op_id,
        is_tombstone: fact.is_tombstone,
        precedence,
    }
}

fn compare_precedence(
    a: &crate::api::ExplainPrecedence,
    b: &crate::api::ExplainPrecedence,
) -> Ordering {
    b.layer
        .cmp(&a.layer)
        .then_with(|| a.interval_width.cmp(&b.interval_width))
        .then_with(|| b.asserted_at.cmp(&a.asserted_at))
        .then_with(|| a.op_id.as_bytes().cmp(&b.op_id.as_bytes()))
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
            Expr::col(AideonOpDeps::PartitionId)
                .eq(id_value(conn.get_database_backend(), partition.0)),
        )
        .and_where(Expr::col(AideonOpDeps::OpId).eq(id_value(conn.get_database_backend(), op_id)))
        .to_owned();
    let rows = query_all(conn, &select).await?;
    rows.into_iter()
        .map(|row| read_id(&row, AideonOpDeps::DepOpId))
        .collect()
}

#[cfg(test)]
mod compaction_tests {
    use super::*;
    use crate::{FieldDef, SetEdgeExistenceIntervalInput, TypeDef, TypeFieldDef};
    use tempfile::tempdir;

    #[tokio::test]
    async fn compaction_prunes_duplicate_intervals() -> MnemeResult<()> {
        let dir = tempdir().expect("tempdir");
        let base = dir.path();
        let config = MnemeConfig::default_sqlite(base.join("compaction.sqlite").to_string_lossy());
        let store = MnemeStore::connect(&config, base).await?;
        let partition = PartitionId(Id::new());
        let actor = ActorId(Id::new());
        let type_id = Id::new();
        let field_id = Id::new();

        store
            .upsert_metamodel_batch(
                partition,
                actor,
                Hlc::now(),
                MetamodelBatch {
                    types: vec![TypeDef {
                        type_id,
                        applies_to: EntityKind::Node,
                        label: "Service".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    }],
                    fields: vec![FieldDef {
                        field_id,
                        label: "name".to_string(),
                        value_type: ValueType::Str,
                        cardinality_multi: false,
                        merge_policy: MergePolicy::Lww,
                        is_indexed: false,
                        disallow_overlap: false,
                    }],
                    type_fields: vec![TypeFieldDef {
                        type_id,
                        field_id,
                        is_required: false,
                        default_value: None,
                        override_default: false,
                        tighten_required: false,
                        disallow_overlap: None,
                    }],
                    edge_type_rules: vec![],
                    metamodel_version: None,
                    metamodel_source: None,
                },
            )
            .await?;

        let node_id = Id::new();
        store
            .create_node(CreateNodeInput {
                partition,
                scenario_id: None,
                actor,
                asserted_at: Hlc::now(),
                node_id,
                type_id: Some(type_id),
                acl_group_id: None,
                owner_actor_id: None,
                visibility: None,
                write_options: None,
            })
            .await?;

        store
            .set_property_interval(SetPropIntervalInput {
                partition,
                scenario_id: None,
                actor,
                asserted_at: Hlc::now(),
                entity_id: node_id,
                field_id,
                value: Value::Str("alpha".to_string()),
                valid_from: ValidTime(0),
                valid_to: Some(ValidTime(10)),
                layer: Layer::Actual,
                write_options: None,
            })
            .await?;
        store
            .set_property_interval(SetPropIntervalInput {
                partition,
                scenario_id: None,
                actor,
                asserted_at: Hlc::now(),
                entity_id: node_id,
                field_id,
                value: Value::Str("beta".to_string()),
                valid_from: ValidTime(0),
                valid_to: Some(ValidTime(10)),
                layer: Layer::Actual,
                write_options: None,
            })
            .await?;

        let edge_id = Id::new();
        store
            .create_edge(CreateEdgeInput {
                partition,
                scenario_id: None,
                actor,
                asserted_at: Hlc::now(),
                edge_id,
                type_id: None,
                src_id: node_id,
                dst_id: node_id,
                exists_valid_from: ValidTime(0),
                exists_valid_to: Some(ValidTime(10)),
                layer: Layer::Actual,
                weight: None,
                acl_group_id: None,
                owner_actor_id: None,
                visibility: None,
                write_options: None,
            })
            .await?;
        store
            .set_edge_existence_interval(SetEdgeExistenceIntervalInput {
                partition,
                scenario_id: None,
                actor,
                asserted_at: Hlc::now(),
                edge_id,
                valid_from: ValidTime(0),
                valid_to: Some(ValidTime(10)),
                layer: Layer::Actual,
                is_tombstone: false,
                write_options: None,
            })
            .await?;

        let before = store
            .export_snapshot_stream(SnapshotOptions {
                partition_id: partition,
                scenario_id: None,
                as_of_asserted_at: Hlc::now(),
                include_facts: true,
                include_entities: true,
            })
            .await?
            .collect::<Vec<_>>();
        let before_prop = before
            .iter()
            .filter(|rec| rec.record_type == "snapshot_fact_str")
            .count();
        let before_edge = before
            .iter()
            .filter(|rec| rec.record_type == "snapshot_edge_exists")
            .count();

        let tx = store.conn.begin().await?;
        store.compact_partition(&tx, partition).await?;
        tx.commit().await?;

        let after = store
            .export_snapshot_stream(SnapshotOptions {
                partition_id: partition,
                scenario_id: None,
                as_of_asserted_at: Hlc::now(),
                include_facts: true,
                include_entities: true,
            })
            .await?
            .collect::<Vec<_>>();
        let after_prop = after
            .iter()
            .filter(|rec| rec.record_type == "snapshot_fact_str")
            .count();
        let after_edge = after
            .iter()
            .filter(|rec| rec.record_type == "snapshot_edge_exists")
            .count();

        assert!(before_prop > after_prop);
        assert!(before_edge > after_edge);
        Ok(())
    }
}
