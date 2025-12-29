use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DatabaseBackend, Statement};
use sea_orm_migration::sea_query::{
    MysqlQueryBuilder, PostgresQueryBuilder, QueryStatementWriter, SqliteQueryBuilder,
    Value as SeaValue,
};

use crate::db::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();

        manager
            .create_table(
                Table::create()
                    .table(AideonSchemaVersion::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AideonSchemaVersion::Version)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AideonSchemaVersion::AppliedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonSchemaVersion::Checksum)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AideonSchemaVersion::AppVersion).string())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonPartitions::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonPartitions::PartitionId, false))
                    .col(
                        ColumnDef::new(AideonPartitions::CreatedAtAsserted)
                            .big_integer()
                            .not_null(),
                    )
                    .col(id_col(backend, AideonPartitions::CreatedByActor, true))
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_partitions")
                            .col(AideonPartitions::PartitionId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonActors::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonActors::PartitionId, false))
                    .col(id_col(backend, AideonActors::ActorId, false))
                    .col(ColumnDef::new(AideonActors::MetadataJson).text())
                    .col(
                        ColumnDef::new(AideonActors::CreatedAtAsserted)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_actors")
                            .col(AideonActors::PartitionId)
                            .col(AideonActors::ActorId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonOps::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonOps::PartitionId, false))
                    .col(id_col(backend, AideonOps::OpId, false))
                    .col(id_col(backend, AideonOps::ActorId, false))
                    .col(
                        ColumnDef::new(AideonOps::AssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .col(id_col(backend, AideonOps::TxId, true))
                    .col(
                        ColumnDef::new(AideonOps::OpType)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonOps::Payload)
                            .blob()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AideonOps::SchemaVersionHint).string())
                    .col(ColumnDef::new(AideonOps::IngestedAssertedAtHlc).big_integer())
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_ops")
                            .col(AideonOps::PartitionId)
                            .col(AideonOps::OpId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonOpDeps::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonOpDeps::PartitionId, false))
                    .col(id_col(backend, AideonOpDeps::OpId, false))
                    .col(id_col(backend, AideonOpDeps::DepOpId, false))
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_op_deps")
                            .col(AideonOpDeps::PartitionId)
                            .col(AideonOpDeps::OpId)
                            .col(AideonOpDeps::DepOpId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonEntities::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonEntities::PartitionId, false))
                    .col(id_col(backend, AideonEntities::ScenarioId, true))
                    .col(id_col(backend, AideonEntities::EntityId, false))
                    .col(
                        ColumnDef::new(AideonEntities::EntityKind)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(id_col(backend, AideonEntities::TypeId, true))
                    .col(
                        ColumnDef::new(AideonEntities::IsDeleted)
                            .boolean()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AideonEntities::AclGroupId).string())
                    .col(id_col(backend, AideonEntities::OwnerActorId, true))
                    .col(ColumnDef::new(AideonEntities::Visibility).tiny_integer())
                    .col(id_col(backend, AideonEntities::CreatedOpId, false))
                    .col(id_col(backend, AideonEntities::UpdatedOpId, false))
                    .col(
                        ColumnDef::new(AideonEntities::CreatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonEntities::UpdatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_entities")
                            .col(AideonEntities::PartitionId)
                            .col(AideonEntities::EntityId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonEdges::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonEdges::PartitionId, false))
                    .col(id_col(backend, AideonEdges::ScenarioId, true))
                    .col(id_col(backend, AideonEdges::EdgeId, false))
                    .col(id_col(backend, AideonEdges::SrcEntityId, false))
                    .col(id_col(backend, AideonEdges::DstEntityId, false))
                    .col(id_col(backend, AideonEdges::EdgeTypeId, true))
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_edges")
                            .col(AideonEdges::PartitionId)
                            .col(AideonEdges::EdgeId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonEdgeExistsFacts::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonEdgeExistsFacts::PartitionId, false))
                    .col(id_col(backend, AideonEdgeExistsFacts::ScenarioId, true))
                    .col(id_col(backend, AideonEdgeExistsFacts::EdgeId, false))
                    .col(
                        ColumnDef::new(AideonEdgeExistsFacts::ValidFrom)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AideonEdgeExistsFacts::ValidTo).big_integer())
                    .col(ColumnDef::new(AideonEdgeExistsFacts::ValidBucket).integer())
                    .col(
                        ColumnDef::new(AideonEdgeExistsFacts::Layer)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonEdgeExistsFacts::AssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .col(id_col(backend, AideonEdgeExistsFacts::OpId, false))
                    .col(
                        ColumnDef::new(AideonEdgeExistsFacts::IsTombstone)
                            .boolean()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_edge_exists")
                            .col(AideonEdgeExistsFacts::PartitionId)
                            .col(AideonEdgeExistsFacts::EdgeId)
                            .col(AideonEdgeExistsFacts::ValidFrom)
                            .col(AideonEdgeExistsFacts::AssertedAtHlc)
                            .col(AideonEdgeExistsFacts::OpId),
                    )
                    .to_owned(),
            )
            .await?;

        create_property_fact_table(
            manager,
            backend,
            AideonPropFactStr::Table,
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
            ColumnDef::new(AideonPropFactStr::ValueText)
                .text()
                .not_null()
                .to_owned(),
            "pk_aideon_prop_fact_str",
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            AideonPropFactI64::Table,
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
            ColumnDef::new(AideonPropFactI64::ValueI64)
                .big_integer()
                .not_null()
                .to_owned(),
            "pk_aideon_prop_fact_i64",
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            AideonPropFactF64::Table,
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
            ColumnDef::new(AideonPropFactF64::ValueF64)
                .double()
                .not_null()
                .to_owned(),
            "pk_aideon_prop_fact_f64",
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            AideonPropFactBool::Table,
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
            ColumnDef::new(AideonPropFactBool::ValueBool)
                .boolean()
                .not_null()
                .to_owned(),
            "pk_aideon_prop_fact_bool",
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            AideonPropFactTime::Table,
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
            ColumnDef::new(AideonPropFactTime::ValueTime)
                .big_integer()
                .not_null()
                .to_owned(),
            "pk_aideon_prop_fact_time",
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            AideonPropFactRef::Table,
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
            id_col(backend, AideonPropFactRef::ValueRefEntityId, false),
            "pk_aideon_prop_fact_ref",
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            AideonPropFactBlob::Table,
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
            ColumnDef::new(AideonPropFactBlob::ValueBlob)
                .blob()
                .not_null()
                .to_owned(),
            "pk_aideon_prop_fact_blob",
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            AideonPropFactJson::Table,
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
            ColumnDef::new(AideonPropFactJson::ValueJson)
                .text()
                .not_null()
                .to_owned(),
            "pk_aideon_prop_fact_json",
        )
        .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonTypes::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonTypes::PartitionId, false))
                    .col(id_col(backend, AideonTypes::TypeId, false))
                    .col(
                        ColumnDef::new(AideonTypes::AppliesTo)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonTypes::Label)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonTypes::IsAbstract)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonTypes::IsDeleted)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonTypes::UpdatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_types")
                            .col(AideonTypes::PartitionId)
                            .col(AideonTypes::TypeId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonTypeExtends::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonTypeExtends::PartitionId, false))
                    .col(id_col(backend, AideonTypeExtends::TypeId, false))
                    .col(id_col(backend, AideonTypeExtends::ParentTypeId, false))
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_type_extends")
                            .col(AideonTypeExtends::PartitionId)
                            .col(AideonTypeExtends::TypeId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonFields::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonFields::PartitionId, false))
                    .col(id_col(backend, AideonFields::FieldId, false))
                    .col(
                        ColumnDef::new(AideonFields::Label)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonFields::ValueType)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonFields::Cardinality)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonFields::MergePolicy)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonFields::IsIndexed)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonFields::IsDeleted)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonFields::UpdatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_fields")
                            .col(AideonFields::PartitionId)
                            .col(AideonFields::FieldId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonTypeFields::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonTypeFields::PartitionId, false))
                    .col(id_col(backend, AideonTypeFields::TypeId, false))
                    .col(id_col(backend, AideonTypeFields::FieldId, false))
                    .col(
                        ColumnDef::new(AideonTypeFields::IsRequired)
                            .boolean()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AideonTypeFields::DefaultValueKind).tiny_integer())
                    .col(ColumnDef::new(AideonTypeFields::DefaultValueStr).text())
                    .col(ColumnDef::new(AideonTypeFields::DefaultValueI64).big_integer())
                    .col(ColumnDef::new(AideonTypeFields::DefaultValueF64).double())
                    .col(ColumnDef::new(AideonTypeFields::DefaultValueBool).boolean())
                    .col(ColumnDef::new(AideonTypeFields::DefaultValueTime).big_integer())
                    .col(id_col(backend, AideonTypeFields::DefaultValueRef, true))
                    .col(ColumnDef::new(AideonTypeFields::DefaultValueBlob).blob())
                    .col(ColumnDef::new(AideonTypeFields::DefaultValueJson).text())
                    .col(
                        ColumnDef::new(AideonTypeFields::OverrideDefault)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonTypeFields::TightenRequired)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonTypeFields::UpdatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_type_fields")
                            .col(AideonTypeFields::PartitionId)
                            .col(AideonTypeFields::TypeId)
                            .col(AideonTypeFields::FieldId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonEffectiveSchemaCache::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonEffectiveSchemaCache::PartitionId, false))
                    .col(id_col(backend, AideonEffectiveSchemaCache::TypeId, false))
                    .col(
                        ColumnDef::new(AideonEffectiveSchemaCache::SchemaVersionHash)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonEffectiveSchemaCache::Blob)
                            .blob()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonEffectiveSchemaCache::BuiltAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_effective_schema_cache")
                            .col(AideonEffectiveSchemaCache::PartitionId)
                            .col(AideonEffectiveSchemaCache::TypeId)
                            .col(AideonEffectiveSchemaCache::SchemaVersionHash),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonEdgeTypeRules::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonEdgeTypeRules::PartitionId, false))
                    .col(id_col(backend, AideonEdgeTypeRules::EdgeTypeId, false))
                    .col(
                        ColumnDef::new(AideonEdgeTypeRules::AllowedSrcTypeIdsJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonEdgeTypeRules::AllowedDstTypeIdsJson)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AideonEdgeTypeRules::SemanticDirection).text())
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_edge_type_rules")
                            .col(AideonEdgeTypeRules::PartitionId)
                            .col(AideonEdgeTypeRules::EdgeTypeId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonGraphProjectionEdges::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonGraphProjectionEdges::PartitionId, false))
                    .col(id_col(backend, AideonGraphProjectionEdges::ScenarioId, true))
                    .col(id_col(backend, AideonGraphProjectionEdges::EdgeId, false))
                    .col(id_col(backend, AideonGraphProjectionEdges::SrcEntityId, false))
                    .col(id_col(backend, AideonGraphProjectionEdges::DstEntityId, false))
                    .col(id_col(backend, AideonGraphProjectionEdges::EdgeTypeId, true))
                    .col(
                        ColumnDef::new(AideonGraphProjectionEdges::Weight)
                            .double()
                            .not_null()
                            .default(1.0),
                    )
                    .col(
                        ColumnDef::new(AideonGraphProjectionEdges::UpdatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_graph_projection_edges")
                            .col(AideonGraphProjectionEdges::PartitionId)
                            .col(AideonGraphProjectionEdges::EdgeId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonPagerankRuns::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonPagerankRuns::PartitionId, false))
                    .col(id_col(backend, AideonPagerankRuns::RunId, false))
                    .col(ColumnDef::new(AideonPagerankRuns::AsOfValidTime).big_integer())
                    .col(ColumnDef::new(AideonPagerankRuns::AsOfAssertedAtHlc).big_integer())
                    .col(
                        ColumnDef::new(AideonPagerankRuns::ParamsJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonPagerankRuns::CreatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_pagerank_runs")
                            .col(AideonPagerankRuns::PartitionId)
                            .col(AideonPagerankRuns::RunId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonPagerankScores::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonPagerankScores::PartitionId, false))
                    .col(id_col(backend, AideonPagerankScores::RunId, false))
                    .col(id_col(backend, AideonPagerankScores::EntityId, false))
                    .col(
                        ColumnDef::new(AideonPagerankScores::Score)
                            .double()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_pagerank_scores")
                            .col(AideonPagerankScores::PartitionId)
                            .col(AideonPagerankScores::RunId)
                            .col(AideonPagerankScores::EntityId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonHlcState::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonHlcState::PartitionId, false))
                    .col(
                        ColumnDef::new(AideonHlcState::LastHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_hlc_state")
                            .col(AideonHlcState::PartitionId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonChangeFeed::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonChangeFeed::PartitionId, false))
                    .col(
                        ColumnDef::new(AideonChangeFeed::Sequence)
                            .big_integer()
                            .not_null(),
                    )
                    .col(id_col(backend, AideonChangeFeed::OpId, false))
                    .col(
                        ColumnDef::new(AideonChangeFeed::AssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .col(id_col(backend, AideonChangeFeed::EntityId, true))
                    .col(
                        ColumnDef::new(AideonChangeFeed::ChangeKind)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AideonChangeFeed::PayloadJson).text())
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_change_feed")
                            .col(AideonChangeFeed::PartitionId)
                            .col(AideonChangeFeed::Sequence),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonJobs::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonJobs::PartitionId, false))
                    .col(id_col(backend, AideonJobs::JobId, false))
                    .col(ColumnDef::new(AideonJobs::JobType).string().not_null())
                    .col(ColumnDef::new(AideonJobs::Status).tiny_integer().not_null())
                    .col(ColumnDef::new(AideonJobs::Priority).integer().not_null())
                    .col(ColumnDef::new(AideonJobs::Attempts).integer().not_null())
                    .col(ColumnDef::new(AideonJobs::MaxAttempts).integer().not_null())
                    .col(ColumnDef::new(AideonJobs::LeaseExpiresAt).big_integer())
                    .col(ColumnDef::new(AideonJobs::NextRunAfter).big_integer())
                    .col(
                        ColumnDef::new(AideonJobs::CreatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonJobs::UpdatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AideonJobs::Payload).blob().not_null())
                    .col(ColumnDef::new(AideonJobs::DedupeKey).string())
                    .col(ColumnDef::new(AideonJobs::LastError).text())
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_jobs")
                            .col(AideonJobs::PartitionId)
                            .col(AideonJobs::JobId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonJobEvents::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonJobEvents::PartitionId, false))
                    .col(id_col(backend, AideonJobEvents::JobId, false))
                    .col(
                        ColumnDef::new(AideonJobEvents::EventTime)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AideonJobEvents::Message).text().not_null())
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_job_events")
                            .col(AideonJobEvents::PartitionId)
                            .col(AideonJobEvents::JobId)
                            .col(AideonJobEvents::EventTime),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonGraphDegreeStats::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonGraphDegreeStats::PartitionId, false))
                    .col(id_col(backend, AideonGraphDegreeStats::ScenarioId, true))
                    .col(ColumnDef::new(AideonGraphDegreeStats::AsOfValidTime).big_integer())
                    .col(id_col(backend, AideonGraphDegreeStats::EntityId, false))
                    .col(ColumnDef::new(AideonGraphDegreeStats::OutDegree).integer().not_null())
                    .col(ColumnDef::new(AideonGraphDegreeStats::InDegree).integer().not_null())
                    .col(
                        ColumnDef::new(AideonGraphDegreeStats::ComputedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_graph_degree_stats")
                            .col(AideonGraphDegreeStats::PartitionId)
                            .col(AideonGraphDegreeStats::ScenarioId)
                            .col(AideonGraphDegreeStats::EntityId)
                            .col(AideonGraphDegreeStats::AsOfValidTime),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonGraphEdgeTypeCounts::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonGraphEdgeTypeCounts::PartitionId, false))
                    .col(id_col(backend, AideonGraphEdgeTypeCounts::ScenarioId, true))
                    .col(id_col(backend, AideonGraphEdgeTypeCounts::EdgeTypeId, true))
                    .col(ColumnDef::new(AideonGraphEdgeTypeCounts::Count).integer().not_null())
                    .col(
                        ColumnDef::new(AideonGraphEdgeTypeCounts::ComputedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_graph_edge_type_counts")
                            .col(AideonGraphEdgeTypeCounts::PartitionId)
                            .col(AideonGraphEdgeTypeCounts::ScenarioId)
                            .col(AideonGraphEdgeTypeCounts::EdgeTypeId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonIntegrityRuns::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonIntegrityRuns::PartitionId, false))
                    .col(id_col(backend, AideonIntegrityRuns::RunId, false))
                    .col(id_col(backend, AideonIntegrityRuns::ScenarioId, true))
                    .col(ColumnDef::new(AideonIntegrityRuns::AsOfValidTime).big_integer())
                    .col(ColumnDef::new(AideonIntegrityRuns::AsOfAssertedAtHlc).big_integer())
                    .col(ColumnDef::new(AideonIntegrityRuns::ParamsJson).text().not_null())
                    .col(
                        ColumnDef::new(AideonIntegrityRuns::CreatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_integrity_runs")
                            .col(AideonIntegrityRuns::PartitionId)
                            .col(AideonIntegrityRuns::RunId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonIntegrityFindings::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonIntegrityFindings::PartitionId, false))
                    .col(id_col(backend, AideonIntegrityFindings::RunId, false))
                    .col(
                        ColumnDef::new(AideonIntegrityFindings::FindingType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonIntegrityFindings::Severity)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(id_col(backend, AideonIntegrityFindings::SubjectEntityId, true))
                    .col(
                        ColumnDef::new(AideonIntegrityFindings::DetailsJson)
                            .text()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonTypeSchemaHead::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonTypeSchemaHead::PartitionId, false))
                    .col(id_col(backend, AideonTypeSchemaHead::TypeId, false))
                    .col(
                        ColumnDef::new(AideonTypeSchemaHead::SchemaVersionHash)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonTypeSchemaHead::UpdatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_type_schema_head")
                            .col(AideonTypeSchemaHead::PartitionId)
                            .col(AideonTypeSchemaHead::TypeId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonIntegrityHead::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonIntegrityHead::PartitionId, false))
                    .col(id_col(backend, AideonIntegrityHead::ScenarioId, true))
                    .col(id_col(backend, AideonIntegrityHead::RunId, false))
                    .col(
                        ColumnDef::new(AideonIntegrityHead::UpdatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_integrity_head")
                            .col(AideonIntegrityHead::PartitionId)
                            .col(AideonIntegrityHead::ScenarioId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonMetamodelVersions::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonMetamodelVersions::PartitionId, false))
                    .col(
                        ColumnDef::new(AideonMetamodelVersions::Version)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AideonMetamodelVersions::Source).string())
                    .col(id_col(backend, AideonMetamodelVersions::OpId, true))
                    .col(
                        ColumnDef::new(AideonMetamodelVersions::CreatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_metamodel_versions")
                            .col(AideonMetamodelVersions::PartitionId)
                            .col(AideonMetamodelVersions::Version),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonValidationRules::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonValidationRules::PartitionId, false))
                    .col(id_col(backend, AideonValidationRules::RuleId, false))
                    .col(
                        ColumnDef::new(AideonValidationRules::ScopeKind)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(id_col(backend, AideonValidationRules::ScopeId, true))
                    .col(
                        ColumnDef::new(AideonValidationRules::Severity)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonValidationRules::TemplateKind)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonValidationRules::ParamsJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonValidationRules::UpdatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_validation_rules")
                            .col(AideonValidationRules::PartitionId)
                            .col(AideonValidationRules::RuleId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AideonComputedRules::Table)
                    .if_not_exists()
                    .col(id_col(backend, AideonComputedRules::PartitionId, false))
                    .col(id_col(backend, AideonComputedRules::RuleId, false))
                    .col(id_col(backend, AideonComputedRules::TargetTypeId, true))
                    .col(id_col(backend, AideonComputedRules::OutputFieldId, true))
                    .col(
                        ColumnDef::new(AideonComputedRules::TemplateKind)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonComputedRules::ParamsJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonComputedRules::UpdatedAssertedAtHlc)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_aideon_computed_rules")
                            .col(AideonComputedRules::PartitionId)
                            .col(AideonComputedRules::RuleId),
                    )
                    .to_owned(),
            )
            .await?;

        create_computed_cache_table(
            manager,
            backend,
            AideonComputedCacheStr::Table,
            AideonComputedCacheStr::PartitionId,
            AideonComputedCacheStr::EntityId,
            AideonComputedCacheStr::FieldId,
            AideonComputedCacheStr::ValidFrom,
            AideonComputedCacheStr::ValidTo,
            AideonComputedCacheStr::RuleVersionHash,
            AideonComputedCacheStr::ComputedAssertedAtHlc,
            ColumnDef::new(AideonComputedCacheStr::ValueText)
                .text()
                .not_null()
                .to_owned(),
            "pk_aideon_computed_cache_str",
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            AideonComputedCacheI64::Table,
            AideonComputedCacheI64::PartitionId,
            AideonComputedCacheI64::EntityId,
            AideonComputedCacheI64::FieldId,
            AideonComputedCacheI64::ValidFrom,
            AideonComputedCacheI64::ValidTo,
            AideonComputedCacheI64::RuleVersionHash,
            AideonComputedCacheI64::ComputedAssertedAtHlc,
            ColumnDef::new(AideonComputedCacheI64::ValueI64)
                .big_integer()
                .not_null()
                .to_owned(),
            "pk_aideon_computed_cache_i64",
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            AideonComputedCacheF64::Table,
            AideonComputedCacheF64::PartitionId,
            AideonComputedCacheF64::EntityId,
            AideonComputedCacheF64::FieldId,
            AideonComputedCacheF64::ValidFrom,
            AideonComputedCacheF64::ValidTo,
            AideonComputedCacheF64::RuleVersionHash,
            AideonComputedCacheF64::ComputedAssertedAtHlc,
            ColumnDef::new(AideonComputedCacheF64::ValueF64)
                .double()
                .not_null()
                .to_owned(),
            "pk_aideon_computed_cache_f64",
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            AideonComputedCacheBool::Table,
            AideonComputedCacheBool::PartitionId,
            AideonComputedCacheBool::EntityId,
            AideonComputedCacheBool::FieldId,
            AideonComputedCacheBool::ValidFrom,
            AideonComputedCacheBool::ValidTo,
            AideonComputedCacheBool::RuleVersionHash,
            AideonComputedCacheBool::ComputedAssertedAtHlc,
            ColumnDef::new(AideonComputedCacheBool::ValueBool)
                .boolean()
                .not_null()
                .to_owned(),
            "pk_aideon_computed_cache_bool",
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            AideonComputedCacheTime::Table,
            AideonComputedCacheTime::PartitionId,
            AideonComputedCacheTime::EntityId,
            AideonComputedCacheTime::FieldId,
            AideonComputedCacheTime::ValidFrom,
            AideonComputedCacheTime::ValidTo,
            AideonComputedCacheTime::RuleVersionHash,
            AideonComputedCacheTime::ComputedAssertedAtHlc,
            ColumnDef::new(AideonComputedCacheTime::ValueTime)
                .big_integer()
                .not_null()
                .to_owned(),
            "pk_aideon_computed_cache_time",
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            AideonComputedCacheRef::Table,
            AideonComputedCacheRef::PartitionId,
            AideonComputedCacheRef::EntityId,
            AideonComputedCacheRef::FieldId,
            AideonComputedCacheRef::ValidFrom,
            AideonComputedCacheRef::ValidTo,
            AideonComputedCacheRef::RuleVersionHash,
            AideonComputedCacheRef::ComputedAssertedAtHlc,
            id_col(backend, AideonComputedCacheRef::ValueRefEntityId, false),
            "pk_aideon_computed_cache_ref",
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            AideonComputedCacheBlob::Table,
            AideonComputedCacheBlob::PartitionId,
            AideonComputedCacheBlob::EntityId,
            AideonComputedCacheBlob::FieldId,
            AideonComputedCacheBlob::ValidFrom,
            AideonComputedCacheBlob::ValidTo,
            AideonComputedCacheBlob::RuleVersionHash,
            AideonComputedCacheBlob::ComputedAssertedAtHlc,
            ColumnDef::new(AideonComputedCacheBlob::ValueBlob)
                .blob()
                .not_null()
                .to_owned(),
            "pk_aideon_computed_cache_blob",
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            AideonComputedCacheJson::Table,
            AideonComputedCacheJson::PartitionId,
            AideonComputedCacheJson::EntityId,
            AideonComputedCacheJson::FieldId,
            AideonComputedCacheJson::ValidFrom,
            AideonComputedCacheJson::ValidTo,
            AideonComputedCacheJson::RuleVersionHash,
            AideonComputedCacheJson::ComputedAssertedAtHlc,
            ColumnDef::new(AideonComputedCacheJson::ValueJson)
                .text()
                .not_null()
                .to_owned(),
            "pk_aideon_computed_cache_json",
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            AideonIdxFieldStr::Table,
            AideonIdxFieldStr::PartitionId,
            AideonIdxFieldStr::ScenarioId,
            AideonIdxFieldStr::FieldId,
            AideonIdxFieldStr::EntityId,
            AideonIdxFieldStr::ValidFrom,
            AideonIdxFieldStr::ValidTo,
            AideonIdxFieldStr::ValidBucket,
            AideonIdxFieldStr::AssertedAtHlc,
            AideonIdxFieldStr::Layer,
            ColumnDef::new(AideonIdxFieldStr::ValueTextNorm)
                .text()
                .not_null()
                .to_owned(),
            "pk_aideon_idx_field_str",
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            AideonIdxFieldI64::Table,
            AideonIdxFieldI64::PartitionId,
            AideonIdxFieldI64::ScenarioId,
            AideonIdxFieldI64::FieldId,
            AideonIdxFieldI64::EntityId,
            AideonIdxFieldI64::ValidFrom,
            AideonIdxFieldI64::ValidTo,
            AideonIdxFieldI64::ValidBucket,
            AideonIdxFieldI64::AssertedAtHlc,
            AideonIdxFieldI64::Layer,
            ColumnDef::new(AideonIdxFieldI64::ValueI64)
                .big_integer()
                .not_null()
                .to_owned(),
            "pk_aideon_idx_field_i64",
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            AideonIdxFieldF64::Table,
            AideonIdxFieldF64::PartitionId,
            AideonIdxFieldF64::ScenarioId,
            AideonIdxFieldF64::FieldId,
            AideonIdxFieldF64::EntityId,
            AideonIdxFieldF64::ValidFrom,
            AideonIdxFieldF64::ValidTo,
            AideonIdxFieldF64::ValidBucket,
            AideonIdxFieldF64::AssertedAtHlc,
            AideonIdxFieldF64::Layer,
            ColumnDef::new(AideonIdxFieldF64::ValueF64)
                .double()
                .not_null()
                .to_owned(),
            "pk_aideon_idx_field_f64",
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            AideonIdxFieldBool::Table,
            AideonIdxFieldBool::PartitionId,
            AideonIdxFieldBool::ScenarioId,
            AideonIdxFieldBool::FieldId,
            AideonIdxFieldBool::EntityId,
            AideonIdxFieldBool::ValidFrom,
            AideonIdxFieldBool::ValidTo,
            AideonIdxFieldBool::ValidBucket,
            AideonIdxFieldBool::AssertedAtHlc,
            AideonIdxFieldBool::Layer,
            ColumnDef::new(AideonIdxFieldBool::ValueBool)
                .boolean()
                .not_null()
                .to_owned(),
            "pk_aideon_idx_field_bool",
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            AideonIdxFieldTime::Table,
            AideonIdxFieldTime::PartitionId,
            AideonIdxFieldTime::ScenarioId,
            AideonIdxFieldTime::FieldId,
            AideonIdxFieldTime::EntityId,
            AideonIdxFieldTime::ValidFrom,
            AideonIdxFieldTime::ValidTo,
            AideonIdxFieldTime::ValidBucket,
            AideonIdxFieldTime::AssertedAtHlc,
            AideonIdxFieldTime::Layer,
            ColumnDef::new(AideonIdxFieldTime::ValueTime)
                .big_integer()
                .not_null()
                .to_owned(),
            "pk_aideon_idx_field_time",
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            AideonIdxFieldRef::Table,
            AideonIdxFieldRef::PartitionId,
            AideonIdxFieldRef::ScenarioId,
            AideonIdxFieldRef::FieldId,
            AideonIdxFieldRef::EntityId,
            AideonIdxFieldRef::ValidFrom,
            AideonIdxFieldRef::ValidTo,
            AideonIdxFieldRef::ValidBucket,
            AideonIdxFieldRef::AssertedAtHlc,
            AideonIdxFieldRef::Layer,
            id_col(backend, AideonIdxFieldRef::ValueRefEntityId, false),
            "pk_aideon_idx_field_ref",
        )
        .await?;

        create_indexes(manager).await?;

        let checksum = blake3::hash(self.name().as_bytes()).to_hex().to_string();
        let insert = Query::insert()
            .into_table(AideonSchemaVersion::Table)
            .columns([
                AideonSchemaVersion::Version,
                AideonSchemaVersion::AppliedAssertedAtHlc,
                AideonSchemaVersion::Checksum,
                AideonSchemaVersion::AppVersion,
            ])
            .values_panic([
                self.name().to_string().into(),
                crate::Hlc::now().as_i64().into(),
                checksum.into(),
                SeaValue::String(None).into(),
            ])
            .to_owned();
        let (sql, values) = build_stmt(backend, &insert);
        manager
            .get_connection()
            .execute_raw(Statement::from_sql_and_values(backend, sql, values))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(AideonIdxFieldRef::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonIdxFieldTime::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonIdxFieldBool::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonIdxFieldF64::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonIdxFieldI64::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonIdxFieldStr::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPagerankScores::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPagerankRuns::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonGraphProjectionEdges::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonEdgeTypeRules::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonEffectiveSchemaCache::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonTypeFields::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonFields::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonTypeExtends::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonTypes::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPropFactJson::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPropFactBlob::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPropFactRef::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPropFactTime::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPropFactBool::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPropFactF64::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPropFactI64::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPropFactStr::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonEdgeExistsFacts::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonEdges::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonEntities::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonOpDeps::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(AideonOps::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonActors::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonPartitions::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(AideonSchemaVersion::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

async fn create_property_fact_table(
    manager: &SchemaManager<'_>,
    backend: DatabaseBackend,
    table: impl Iden + Clone,
    partition_col: impl Iden + Clone,
    scenario_col: impl Iden + Clone,
    entity_col: impl Iden + Clone,
    field_col: impl Iden + Clone,
    valid_from_col: impl Iden + Clone,
    valid_to_col: impl Iden + Clone,
    valid_bucket_col: impl Iden + Clone,
    layer_col: impl Iden + Clone,
    asserted_col: impl Iden + Clone,
    op_col: impl Iden + Clone,
    tombstone_col: impl Iden + Clone,
    value_col: ColumnDef,
    pk_name: &str,
) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(table.clone())
                .if_not_exists()
                .col(id_col(backend, partition_col.clone(), false))
                .col(id_col(backend, scenario_col.clone(), true))
                .col(id_col(backend, entity_col.clone(), false))
                .col(id_col(backend, field_col.clone(), false))
                .col(
                    ColumnDef::new(valid_from_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(valid_to_col.clone()).big_integer())
                .col(ColumnDef::new(valid_bucket_col.clone()).integer())
                .col(
                    ColumnDef::new(layer_col.clone())
                        .tiny_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(asserted_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .col(id_col(backend, op_col.clone(), false))
                .col(ColumnDef::new(tombstone_col.clone()).boolean().not_null())
                .col(value_col)
                .primary_key(
                    Index::create()
                        .name(pk_name)
                        .col(partition_col)
                        .col(entity_col)
                        .col(field_col)
                        .col(valid_from_col)
                        .col(asserted_col)
                        .col(op_col),
                )
                .to_owned(),
        )
        .await?;
    Ok(())
}

async fn create_index_field_table(
    manager: &SchemaManager<'_>,
    backend: DatabaseBackend,
    table: impl Iden + Clone,
    partition_col: impl Iden + Clone,
    scenario_col: impl Iden + Clone,
    field_col: impl Iden + Clone,
    entity_col: impl Iden + Clone,
    valid_from_col: impl Iden + Clone,
    valid_to_col: impl Iden + Clone,
    valid_bucket_col: impl Iden + Clone,
    asserted_col: impl Iden + Clone,
    layer_col: impl Iden + Clone,
    value_col: ColumnDef,
    pk_name: &str,
) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(table.clone())
                .if_not_exists()
                .col(id_col(backend, partition_col.clone(), false))
                .col(id_col(backend, scenario_col.clone(), true))
                .col(id_col(backend, field_col.clone(), false))
                .col(value_col)
                .col(id_col(backend, entity_col.clone(), false))
                .col(
                    ColumnDef::new(valid_from_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(valid_to_col.clone()).big_integer())
                .col(ColumnDef::new(valid_bucket_col.clone()).integer())
                .col(
                    ColumnDef::new(asserted_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(layer_col.clone())
                        .tiny_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .name(pk_name)
                        .col(partition_col)
                        .col(field_col)
                        .col(entity_col)
                        .col(valid_from_col)
                        .col(asserted_col),
                )
                .to_owned(),
        )
        .await?;
    Ok(())
}

async fn create_computed_cache_table(
    manager: &SchemaManager<'_>,
    backend: DatabaseBackend,
    table: impl Iden + Clone,
    partition_col: impl Iden + Clone,
    entity_col: impl Iden + Clone,
    field_col: impl Iden + Clone,
    valid_from_col: impl Iden + Clone,
    valid_to_col: impl Iden + Clone,
    rule_hash_col: impl Iden + Clone,
    computed_at_col: impl Iden + Clone,
    value_col: ColumnDef,
    pk_name: &str,
) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(table)
                .if_not_exists()
                .col(id_col(backend, partition_col.clone(), false))
                .col(id_col(backend, entity_col.clone(), false))
                .col(id_col(backend, field_col.clone(), false))
                .col(
                    ColumnDef::new(valid_from_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(valid_to_col.clone()).big_integer())
                .col(value_col)
                .col(
                    ColumnDef::new(rule_hash_col.clone())
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(computed_at_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .name(pk_name)
                        .col(partition_col)
                        .col(entity_col)
                        .col(field_col)
                        .col(valid_from_col)
                        .col(rule_hash_col),
                )
                .to_owned(),
        )
        .await?;
    Ok(())
}

async fn create_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("aideon_ops_partition_asserted_idx")
                .table(AideonOps::Table)
                .col(AideonOps::PartitionId)
                .col(AideonOps::AssertedAtHlc)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_ops_partition_actor_idx")
                .table(AideonOps::Table)
                .col(AideonOps::PartitionId)
                .col(AideonOps::ActorId)
                .col(AideonOps::AssertedAtHlc)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_ops_partition_tx_idx")
                .table(AideonOps::Table)
                .col(AideonOps::PartitionId)
                .col(AideonOps::TxId)
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("aideon_entities_kind_idx")
                .table(AideonEntities::Table)
                .col(AideonEntities::PartitionId)
                .col(AideonEntities::EntityKind)
                .col(AideonEntities::TypeId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_entities_updated_idx")
                .table(AideonEntities::Table)
                .col(AideonEntities::PartitionId)
                .col(AideonEntities::UpdatedAssertedAtHlc)
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("aideon_edges_out_idx")
                .table(AideonEdges::Table)
                .col(AideonEdges::PartitionId)
                .col(AideonEdges::SrcEntityId)
                .col(AideonEdges::EdgeTypeId)
                .col(AideonEdges::EdgeId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_edges_in_idx")
                .table(AideonEdges::Table)
                .col(AideonEdges::PartitionId)
                .col(AideonEdges::DstEntityId)
                .col(AideonEdges::EdgeTypeId)
                .col(AideonEdges::EdgeId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_edges_type_idx")
                .table(AideonEdges::Table)
                .col(AideonEdges::PartitionId)
                .col(AideonEdges::EdgeTypeId)
                .col(AideonEdges::EdgeId)
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("aideon_edge_exists_from_idx")
                .table(AideonEdgeExistsFacts::Table)
                .col(AideonEdgeExistsFacts::PartitionId)
                .col(AideonEdgeExistsFacts::EdgeId)
                .col(AideonEdgeExistsFacts::ValidFrom)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_edge_exists_to_idx")
                .table(AideonEdgeExistsFacts::Table)
                .col(AideonEdgeExistsFacts::PartitionId)
                .col(AideonEdgeExistsFacts::EdgeId)
                .col(AideonEdgeExistsFacts::ValidTo)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_edge_exists_partition_valid_idx")
                .table(AideonEdgeExistsFacts::Table)
                .col(AideonEdgeExistsFacts::PartitionId)
                .col(AideonEdgeExistsFacts::ValidFrom)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_edge_exists_partition_bucket_idx")
                .table(AideonEdgeExistsFacts::Table)
                .col(AideonEdgeExistsFacts::PartitionId)
                .col(AideonEdgeExistsFacts::ValidBucket)
                .to_owned(),
        )
        .await?;

    create_prop_indexes(manager, AideonPropFactStr::Table, "aideon_prop_fact_str").await?;
    create_prop_indexes(manager, AideonPropFactI64::Table, "aideon_prop_fact_i64").await?;
    create_prop_indexes(manager, AideonPropFactF64::Table, "aideon_prop_fact_f64").await?;
    create_prop_indexes(manager, AideonPropFactBool::Table, "aideon_prop_fact_bool").await?;
    create_prop_indexes(manager, AideonPropFactTime::Table, "aideon_prop_fact_time").await?;
    create_prop_indexes(manager, AideonPropFactRef::Table, "aideon_prop_fact_ref").await?;
    create_prop_indexes(manager, AideonPropFactBlob::Table, "aideon_prop_fact_blob").await?;
    create_prop_indexes(manager, AideonPropFactJson::Table, "aideon_prop_fact_json").await?;

    manager
        .create_index(
            Index::create()
                .name("aideon_types_applies_idx")
                .table(AideonTypes::Table)
                .col(AideonTypes::PartitionId)
                .col(AideonTypes::AppliesTo)
                .col(AideonTypes::TypeId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_types_label_idx")
                .table(AideonTypes::Table)
                .col(AideonTypes::PartitionId)
                .col(AideonTypes::Label)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_fields_value_type_idx")
                .table(AideonFields::Table)
                .col(AideonFields::PartitionId)
                .col(AideonFields::ValueType)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_fields_label_idx")
                .table(AideonFields::Table)
                .col(AideonFields::PartitionId)
                .col(AideonFields::Label)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_type_extends_parent_idx")
                .table(AideonTypeExtends::Table)
                .col(AideonTypeExtends::PartitionId)
                .col(AideonTypeExtends::ParentTypeId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_type_fields_type_idx")
                .table(AideonTypeFields::Table)
                .col(AideonTypeFields::PartitionId)
                .col(AideonTypeFields::TypeId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_type_fields_field_idx")
                .table(AideonTypeFields::Table)
                .col(AideonTypeFields::PartitionId)
                .col(AideonTypeFields::FieldId)
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("aideon_graph_projection_src_idx")
                .table(AideonGraphProjectionEdges::Table)
                .col(AideonGraphProjectionEdges::PartitionId)
                .col(AideonGraphProjectionEdges::ScenarioId)
                .col(AideonGraphProjectionEdges::SrcEntityId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_graph_projection_dst_idx")
                .table(AideonGraphProjectionEdges::Table)
                .col(AideonGraphProjectionEdges::PartitionId)
                .col(AideonGraphProjectionEdges::ScenarioId)
                .col(AideonGraphProjectionEdges::DstEntityId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_graph_projection_type_idx")
                .table(AideonGraphProjectionEdges::Table)
                .col(AideonGraphProjectionEdges::PartitionId)
                .col(AideonGraphProjectionEdges::ScenarioId)
                .col(AideonGraphProjectionEdges::EdgeTypeId)
                .col(AideonGraphProjectionEdges::SrcEntityId)
                .to_owned(),
        )
        .await?;

    create_index_table_index(
        manager,
        AideonIdxFieldStr::Table,
        "aideon_idx_field_str_entity_idx",
    )
    .await?;
    create_index_table_bucket_index(
        manager,
        AideonIdxFieldStr::Table,
        "aideon_idx_field_str_bucket_idx",
    )
    .await?;
    create_index_table_index(
        manager,
        AideonIdxFieldI64::Table,
        "aideon_idx_field_i64_entity_idx",
    )
    .await?;
    create_index_table_bucket_index(
        manager,
        AideonIdxFieldI64::Table,
        "aideon_idx_field_i64_bucket_idx",
    )
    .await?;
    create_index_table_index(
        manager,
        AideonIdxFieldF64::Table,
        "aideon_idx_field_f64_entity_idx",
    )
    .await?;
    create_index_table_bucket_index(
        manager,
        AideonIdxFieldF64::Table,
        "aideon_idx_field_f64_bucket_idx",
    )
    .await?;
    create_index_table_index(
        manager,
        AideonIdxFieldBool::Table,
        "aideon_idx_field_bool_entity_idx",
    )
    .await?;
    create_index_table_bucket_index(
        manager,
        AideonIdxFieldBool::Table,
        "aideon_idx_field_bool_bucket_idx",
    )
    .await?;
    create_index_table_index(
        manager,
        AideonIdxFieldTime::Table,
        "aideon_idx_field_time_entity_idx",
    )
    .await?;
    create_index_table_bucket_index(
        manager,
        AideonIdxFieldTime::Table,
        "aideon_idx_field_time_bucket_idx",
    )
    .await?;
    create_index_table_index(
        manager,
        AideonIdxFieldRef::Table,
        "aideon_idx_field_ref_entity_idx",
    )
    .await?;
    create_index_table_bucket_index(
        manager,
        AideonIdxFieldRef::Table,
        "aideon_idx_field_ref_bucket_idx",
    )
    .await?;

    manager
        .create_index(
            Index::create()
                .name("aideon_idx_field_str_lookup_idx")
                .table(AideonIdxFieldStr::Table)
                .col(AideonIdxFieldStr::PartitionId)
                .col(AideonIdxFieldStr::ScenarioId)
                .col(AideonIdxFieldStr::FieldId)
                .col(AideonIdxFieldStr::ValueTextNorm)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_idx_field_i64_lookup_idx")
                .table(AideonIdxFieldI64::Table)
                .col(AideonIdxFieldI64::PartitionId)
                .col(AideonIdxFieldI64::ScenarioId)
                .col(AideonIdxFieldI64::FieldId)
                .col(AideonIdxFieldI64::ValueI64)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_idx_field_f64_lookup_idx")
                .table(AideonIdxFieldF64::Table)
                .col(AideonIdxFieldF64::PartitionId)
                .col(AideonIdxFieldF64::ScenarioId)
                .col(AideonIdxFieldF64::FieldId)
                .col(AideonIdxFieldF64::ValueF64)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_idx_field_bool_lookup_idx")
                .table(AideonIdxFieldBool::Table)
                .col(AideonIdxFieldBool::PartitionId)
                .col(AideonIdxFieldBool::ScenarioId)
                .col(AideonIdxFieldBool::FieldId)
                .col(AideonIdxFieldBool::ValueBool)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_idx_field_time_lookup_idx")
                .table(AideonIdxFieldTime::Table)
                .col(AideonIdxFieldTime::PartitionId)
                .col(AideonIdxFieldTime::ScenarioId)
                .col(AideonIdxFieldTime::FieldId)
                .col(AideonIdxFieldTime::ValueTime)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_idx_field_ref_lookup_idx")
                .table(AideonIdxFieldRef::Table)
                .col(AideonIdxFieldRef::PartitionId)
                .col(AideonIdxFieldRef::ScenarioId)
                .col(AideonIdxFieldRef::FieldId)
                .col(AideonIdxFieldRef::ValueRefEntityId)
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("aideon_pagerank_scores_run_score_idx")
                .table(AideonPagerankScores::Table)
                .col(AideonPagerankScores::PartitionId)
                .col(AideonPagerankScores::RunId)
                .col(AideonPagerankScores::Score)
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("aideon_graph_degree_out_idx")
                .table(AideonGraphDegreeStats::Table)
                .col(AideonGraphDegreeStats::PartitionId)
                .col(AideonGraphDegreeStats::ScenarioId)
                .col(AideonGraphDegreeStats::AsOfValidTime)
                .col(AideonGraphDegreeStats::OutDegree)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_graph_degree_in_idx")
                .table(AideonGraphDegreeStats::Table)
                .col(AideonGraphDegreeStats::PartitionId)
                .col(AideonGraphDegreeStats::ScenarioId)
                .col(AideonGraphDegreeStats::AsOfValidTime)
                .col(AideonGraphDegreeStats::InDegree)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_graph_edge_type_counts_idx")
                .table(AideonGraphEdgeTypeCounts::Table)
                .col(AideonGraphEdgeTypeCounts::PartitionId)
                .col(AideonGraphEdgeTypeCounts::ScenarioId)
                .col(AideonGraphEdgeTypeCounts::EdgeTypeId)
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("aideon_jobs_pending_idx")
                .table(AideonJobs::Table)
                .col(AideonJobs::PartitionId)
                .col(AideonJobs::Status)
                .col(AideonJobs::Priority)
                .col(AideonJobs::CreatedAssertedAtHlc)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_jobs_ready_idx")
                .table(AideonJobs::Table)
                .col(AideonJobs::PartitionId)
                .col(AideonJobs::Status)
                .col(AideonJobs::NextRunAfter)
                .col(AideonJobs::Priority)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_jobs_lease_idx")
                .table(AideonJobs::Table)
                .col(AideonJobs::PartitionId)
                .col(AideonJobs::LeaseExpiresAt)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("aideon_jobs_dedupe_idx")
                .table(AideonJobs::Table)
                .col(AideonJobs::PartitionId)
                .col(AideonJobs::JobType)
                .col(AideonJobs::DedupeKey)
                .col(AideonJobs::Status)
                .unique()
                .to_owned(),
        )
        .await?;

    Ok(())
}

async fn create_prop_indexes(
    manager: &SchemaManager<'_>,
    table: impl Iden + Clone,
    prefix: &str,
) -> Result<(), DbErr> {
    let from_name = format!("{prefix}_from_idx");
    let to_name = format!("{prefix}_to_idx");
    let field_from_name = format!("{prefix}_field_from_idx");
    let field_to_name = format!("{prefix}_field_to_idx");
    let bucket_name = format!("{prefix}_bucket_idx");
    let table_from = table.clone();
    let table_to = table.clone();
    let table_field_from = table.clone();
    let table_field_to = table.clone();
    let table_bucket = table;
    manager
        .create_index(
            Index::create()
                .name(&from_name)
                .table(table_from)
                .col(AideonPropFactStr::PartitionId)
                .col(AideonPropFactStr::ScenarioId)
                .col(AideonPropFactStr::EntityId)
                .col(AideonPropFactStr::FieldId)
                .col(AideonPropFactStr::ValidFrom)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name(&to_name)
                .table(table_to)
                .col(AideonPropFactStr::PartitionId)
                .col(AideonPropFactStr::ScenarioId)
                .col(AideonPropFactStr::EntityId)
                .col(AideonPropFactStr::FieldId)
                .col(AideonPropFactStr::ValidTo)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name(&field_from_name)
                .table(table_field_from)
                .col(AideonPropFactStr::PartitionId)
                .col(AideonPropFactStr::ScenarioId)
                .col(AideonPropFactStr::FieldId)
                .col(AideonPropFactStr::ValidFrom)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name(&field_to_name)
                .table(table_field_to)
                .col(AideonPropFactStr::PartitionId)
                .col(AideonPropFactStr::ScenarioId)
                .col(AideonPropFactStr::FieldId)
                .col(AideonPropFactStr::ValidTo)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name(&bucket_name)
                .table(table_bucket)
                .col(AideonPropFactStr::PartitionId)
                .col(AideonPropFactStr::ValidBucket)
                .to_owned(),
        )
        .await?;
    Ok(())
}

async fn create_index_table_index(
    manager: &SchemaManager<'_>,
    table: impl Iden + Clone,
    name: &str,
) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name(name)
                .table(table)
                .col(AideonIdxFieldStr::PartitionId)
                .col(AideonIdxFieldStr::ScenarioId)
                .col(AideonIdxFieldStr::FieldId)
                .col(AideonIdxFieldStr::EntityId)
                .to_owned(),
        )
        .await?;
    Ok(())
}

async fn create_index_table_bucket_index(
    manager: &SchemaManager<'_>,
    table: impl Iden + Clone,
    name: &str,
) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name(name)
                .table(table)
                .col(AideonIdxFieldStr::PartitionId)
                .col(AideonIdxFieldStr::ValidBucket)
                .to_owned(),
        )
        .await?;
    Ok(())
}

fn build_stmt<S: QueryStatementWriter>(
    backend: DatabaseBackend,
    stmt: &S,
) -> (String, sea_orm_migration::sea_query::Values) {
    match backend {
        DatabaseBackend::Sqlite => stmt.build(SqliteQueryBuilder),
        DatabaseBackend::Postgres => stmt.build(PostgresQueryBuilder),
        DatabaseBackend::MySql => stmt.build(MysqlQueryBuilder),
        _ => stmt.build(SqliteQueryBuilder),
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
