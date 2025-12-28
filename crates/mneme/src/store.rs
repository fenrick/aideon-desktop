use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
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

use crate::api::{
    AnalyticsApi, AnalyticsResultsApi, CreateScenarioInput, Direction, ExportOpsInput,
    GraphReadApi, GraphWriteApi, MetamodelApi, ProjectionEdge, PropertyWriteApi, ScenarioApi,
    SyncApi,
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
    ActorId, Hlc, Id, MnemeConfig, MnemeError, MnemeResult, OpEnvelope, OpId, PartitionId,
    ReadEntityAtTimeInput, ReadEntityAtTimeResult, ReadValue, ScenarioId, TraverseAtTimeInput,
    TraverseEdgeItem, ValidTime,
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
        Ok((op_id, asserted_at, payload_bytes, op_type))
    }

    fn layer_value(layer: Layer) -> i64 {
        layer as i64
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
}

#[async_trait]
impl AnalyticsApi for MnemeStore {
    async fn get_projection_edges(
        &self,
        input: crate::GetProjectionEdgesInput,
    ) -> MnemeResult<Vec<ProjectionEdge>> {
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
            let norm = text.to_lowercase();
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
            let norm = text.to_lowercase();
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
                src_id,
                dst_id,
                edge_type_id,
                weight,
            },
        ));
    }
    Ok(edges)
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
