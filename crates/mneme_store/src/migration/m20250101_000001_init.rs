use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DatabaseBackend, Statement};
use sea_orm_migration::sea_query::{
    MysqlQueryBuilder, PostgresQueryBuilder, QueryStatementWriter, SqliteQueryBuilder,
    Value as SeaValue,
};

use crate::db::*;
use aideon_mneme_core::Hlc;

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
                    .col(ColumnDef::new(AideonOps::OpType).small_integer().not_null())
                    .col(ColumnDef::new(AideonOps::Payload).blob().not_null())
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
            PropertyFactTableSpec {
                table: AideonPropFactStr::Table,
                partition_col: AideonPropFactStr::PartitionId,
                scenario_col: AideonPropFactStr::ScenarioId,
                entity_col: AideonPropFactStr::EntityId,
                field_col: AideonPropFactStr::FieldId,
                valid_from_col: AideonPropFactStr::ValidFrom,
                valid_to_col: AideonPropFactStr::ValidTo,
                valid_bucket_col: AideonPropFactStr::ValidBucket,
                layer_col: AideonPropFactStr::Layer,
                asserted_col: AideonPropFactStr::AssertedAtHlc,
                op_col: AideonPropFactStr::OpId,
                tombstone_col: AideonPropFactStr::IsTombstone,
                value_col: ColumnDef::new(AideonPropFactStr::ValueText)
                    .text()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_prop_fact_str",
            },
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            PropertyFactTableSpec {
                table: AideonPropFactI64::Table,
                partition_col: AideonPropFactI64::PartitionId,
                scenario_col: AideonPropFactI64::ScenarioId,
                entity_col: AideonPropFactI64::EntityId,
                field_col: AideonPropFactI64::FieldId,
                valid_from_col: AideonPropFactI64::ValidFrom,
                valid_to_col: AideonPropFactI64::ValidTo,
                valid_bucket_col: AideonPropFactI64::ValidBucket,
                layer_col: AideonPropFactI64::Layer,
                asserted_col: AideonPropFactI64::AssertedAtHlc,
                op_col: AideonPropFactI64::OpId,
                tombstone_col: AideonPropFactI64::IsTombstone,
                value_col: ColumnDef::new(AideonPropFactI64::ValueI64)
                    .big_integer()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_prop_fact_i64",
            },
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            PropertyFactTableSpec {
                table: AideonPropFactF64::Table,
                partition_col: AideonPropFactF64::PartitionId,
                scenario_col: AideonPropFactF64::ScenarioId,
                entity_col: AideonPropFactF64::EntityId,
                field_col: AideonPropFactF64::FieldId,
                valid_from_col: AideonPropFactF64::ValidFrom,
                valid_to_col: AideonPropFactF64::ValidTo,
                valid_bucket_col: AideonPropFactF64::ValidBucket,
                layer_col: AideonPropFactF64::Layer,
                asserted_col: AideonPropFactF64::AssertedAtHlc,
                op_col: AideonPropFactF64::OpId,
                tombstone_col: AideonPropFactF64::IsTombstone,
                value_col: ColumnDef::new(AideonPropFactF64::ValueF64)
                    .double()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_prop_fact_f64",
            },
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            PropertyFactTableSpec {
                table: AideonPropFactBool::Table,
                partition_col: AideonPropFactBool::PartitionId,
                scenario_col: AideonPropFactBool::ScenarioId,
                entity_col: AideonPropFactBool::EntityId,
                field_col: AideonPropFactBool::FieldId,
                valid_from_col: AideonPropFactBool::ValidFrom,
                valid_to_col: AideonPropFactBool::ValidTo,
                valid_bucket_col: AideonPropFactBool::ValidBucket,
                layer_col: AideonPropFactBool::Layer,
                asserted_col: AideonPropFactBool::AssertedAtHlc,
                op_col: AideonPropFactBool::OpId,
                tombstone_col: AideonPropFactBool::IsTombstone,
                value_col: ColumnDef::new(AideonPropFactBool::ValueBool)
                    .boolean()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_prop_fact_bool",
            },
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            PropertyFactTableSpec {
                table: AideonPropFactTime::Table,
                partition_col: AideonPropFactTime::PartitionId,
                scenario_col: AideonPropFactTime::ScenarioId,
                entity_col: AideonPropFactTime::EntityId,
                field_col: AideonPropFactTime::FieldId,
                valid_from_col: AideonPropFactTime::ValidFrom,
                valid_to_col: AideonPropFactTime::ValidTo,
                valid_bucket_col: AideonPropFactTime::ValidBucket,
                layer_col: AideonPropFactTime::Layer,
                asserted_col: AideonPropFactTime::AssertedAtHlc,
                op_col: AideonPropFactTime::OpId,
                tombstone_col: AideonPropFactTime::IsTombstone,
                value_col: ColumnDef::new(AideonPropFactTime::ValueTime)
                    .big_integer()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_prop_fact_time",
            },
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            PropertyFactTableSpec {
                table: AideonPropFactRef::Table,
                partition_col: AideonPropFactRef::PartitionId,
                scenario_col: AideonPropFactRef::ScenarioId,
                entity_col: AideonPropFactRef::EntityId,
                field_col: AideonPropFactRef::FieldId,
                valid_from_col: AideonPropFactRef::ValidFrom,
                valid_to_col: AideonPropFactRef::ValidTo,
                valid_bucket_col: AideonPropFactRef::ValidBucket,
                layer_col: AideonPropFactRef::Layer,
                asserted_col: AideonPropFactRef::AssertedAtHlc,
                op_col: AideonPropFactRef::OpId,
                tombstone_col: AideonPropFactRef::IsTombstone,
                value_col: id_col(backend, AideonPropFactRef::ValueRefEntityId, false),
                pk_name: "pk_aideon_prop_fact_ref",
            },
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            PropertyFactTableSpec {
                table: AideonPropFactBlob::Table,
                partition_col: AideonPropFactBlob::PartitionId,
                scenario_col: AideonPropFactBlob::ScenarioId,
                entity_col: AideonPropFactBlob::EntityId,
                field_col: AideonPropFactBlob::FieldId,
                valid_from_col: AideonPropFactBlob::ValidFrom,
                valid_to_col: AideonPropFactBlob::ValidTo,
                valid_bucket_col: AideonPropFactBlob::ValidBucket,
                layer_col: AideonPropFactBlob::Layer,
                asserted_col: AideonPropFactBlob::AssertedAtHlc,
                op_col: AideonPropFactBlob::OpId,
                tombstone_col: AideonPropFactBlob::IsTombstone,
                value_col: ColumnDef::new(AideonPropFactBlob::ValueBlob)
                    .blob()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_prop_fact_blob",
            },
        )
        .await?;

        create_property_fact_table(
            manager,
            backend,
            PropertyFactTableSpec {
                table: AideonPropFactJson::Table,
                partition_col: AideonPropFactJson::PartitionId,
                scenario_col: AideonPropFactJson::ScenarioId,
                entity_col: AideonPropFactJson::EntityId,
                field_col: AideonPropFactJson::FieldId,
                valid_from_col: AideonPropFactJson::ValidFrom,
                valid_to_col: AideonPropFactJson::ValidTo,
                valid_bucket_col: AideonPropFactJson::ValidBucket,
                layer_col: AideonPropFactJson::Layer,
                asserted_col: AideonPropFactJson::AssertedAtHlc,
                op_col: AideonPropFactJson::OpId,
                tombstone_col: AideonPropFactJson::IsTombstone,
                value_col: ColumnDef::new(AideonPropFactJson::ValueJson)
                    .text()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_prop_fact_json",
            },
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
                    .col(ColumnDef::new(AideonTypes::Label).string().not_null())
                    .col(ColumnDef::new(AideonTypes::IsAbstract).boolean().not_null())
                    .col(ColumnDef::new(AideonTypes::IsDeleted).boolean().not_null())
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
                    .col(ColumnDef::new(AideonFields::Label).string().not_null())
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
                    .col(ColumnDef::new(AideonFields::IsIndexed).boolean().not_null())
                    .col(
                        ColumnDef::new(AideonFields::DisallowOverlap)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(AideonFields::IsDeleted).boolean().not_null())
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
                    .col(ColumnDef::new(AideonTypeFields::DisallowOverlap).boolean())
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
                    .col(id_col(
                        backend,
                        AideonEffectiveSchemaCache::PartitionId,
                        false,
                    ))
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
                    .col(id_col(
                        backend,
                        AideonGraphProjectionEdges::PartitionId,
                        false,
                    ))
                    .col(id_col(
                        backend,
                        AideonGraphProjectionEdges::ScenarioId,
                        true,
                    ))
                    .col(id_col(backend, AideonGraphProjectionEdges::EdgeId, false))
                    .col(id_col(
                        backend,
                        AideonGraphProjectionEdges::SrcEntityId,
                        false,
                    ))
                    .col(id_col(
                        backend,
                        AideonGraphProjectionEdges::DstEntityId,
                        false,
                    ))
                    .col(id_col(
                        backend,
                        AideonGraphProjectionEdges::EdgeTypeId,
                        true,
                    ))
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
                    .col(
                        ColumnDef::new(AideonGraphDegreeStats::OutDegree)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AideonGraphDegreeStats::InDegree)
                            .integer()
                            .not_null(),
                    )
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
                    .col(id_col(
                        backend,
                        AideonGraphEdgeTypeCounts::PartitionId,
                        false,
                    ))
                    .col(id_col(backend, AideonGraphEdgeTypeCounts::ScenarioId, true))
                    .col(id_col(backend, AideonGraphEdgeTypeCounts::EdgeTypeId, true))
                    .col(
                        ColumnDef::new(AideonGraphEdgeTypeCounts::Count)
                            .integer()
                            .not_null(),
                    )
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
                    .col(
                        ColumnDef::new(AideonIntegrityRuns::ParamsJson)
                            .text()
                            .not_null(),
                    )
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
                    .col(id_col(
                        backend,
                        AideonIntegrityFindings::SubjectEntityId,
                        true,
                    ))
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
            ComputedCacheTableSpec {
                table: AideonComputedCacheStr::Table,
                partition_col: AideonComputedCacheStr::PartitionId,
                entity_col: AideonComputedCacheStr::EntityId,
                field_col: AideonComputedCacheStr::FieldId,
                valid_from_col: AideonComputedCacheStr::ValidFrom,
                valid_to_col: AideonComputedCacheStr::ValidTo,
                rule_hash_col: AideonComputedCacheStr::RuleVersionHash,
                computed_at_col: AideonComputedCacheStr::ComputedAssertedAtHlc,
                value_col: ColumnDef::new(AideonComputedCacheStr::ValueText)
                    .text()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_computed_cache_str",
            },
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            ComputedCacheTableSpec {
                table: AideonComputedCacheI64::Table,
                partition_col: AideonComputedCacheI64::PartitionId,
                entity_col: AideonComputedCacheI64::EntityId,
                field_col: AideonComputedCacheI64::FieldId,
                valid_from_col: AideonComputedCacheI64::ValidFrom,
                valid_to_col: AideonComputedCacheI64::ValidTo,
                rule_hash_col: AideonComputedCacheI64::RuleVersionHash,
                computed_at_col: AideonComputedCacheI64::ComputedAssertedAtHlc,
                value_col: ColumnDef::new(AideonComputedCacheI64::ValueI64)
                    .big_integer()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_computed_cache_i64",
            },
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            ComputedCacheTableSpec {
                table: AideonComputedCacheF64::Table,
                partition_col: AideonComputedCacheF64::PartitionId,
                entity_col: AideonComputedCacheF64::EntityId,
                field_col: AideonComputedCacheF64::FieldId,
                valid_from_col: AideonComputedCacheF64::ValidFrom,
                valid_to_col: AideonComputedCacheF64::ValidTo,
                rule_hash_col: AideonComputedCacheF64::RuleVersionHash,
                computed_at_col: AideonComputedCacheF64::ComputedAssertedAtHlc,
                value_col: ColumnDef::new(AideonComputedCacheF64::ValueF64)
                    .double()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_computed_cache_f64",
            },
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            ComputedCacheTableSpec {
                table: AideonComputedCacheBool::Table,
                partition_col: AideonComputedCacheBool::PartitionId,
                entity_col: AideonComputedCacheBool::EntityId,
                field_col: AideonComputedCacheBool::FieldId,
                valid_from_col: AideonComputedCacheBool::ValidFrom,
                valid_to_col: AideonComputedCacheBool::ValidTo,
                rule_hash_col: AideonComputedCacheBool::RuleVersionHash,
                computed_at_col: AideonComputedCacheBool::ComputedAssertedAtHlc,
                value_col: ColumnDef::new(AideonComputedCacheBool::ValueBool)
                    .boolean()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_computed_cache_bool",
            },
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            ComputedCacheTableSpec {
                table: AideonComputedCacheTime::Table,
                partition_col: AideonComputedCacheTime::PartitionId,
                entity_col: AideonComputedCacheTime::EntityId,
                field_col: AideonComputedCacheTime::FieldId,
                valid_from_col: AideonComputedCacheTime::ValidFrom,
                valid_to_col: AideonComputedCacheTime::ValidTo,
                rule_hash_col: AideonComputedCacheTime::RuleVersionHash,
                computed_at_col: AideonComputedCacheTime::ComputedAssertedAtHlc,
                value_col: ColumnDef::new(AideonComputedCacheTime::ValueTime)
                    .big_integer()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_computed_cache_time",
            },
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            ComputedCacheTableSpec {
                table: AideonComputedCacheRef::Table,
                partition_col: AideonComputedCacheRef::PartitionId,
                entity_col: AideonComputedCacheRef::EntityId,
                field_col: AideonComputedCacheRef::FieldId,
                valid_from_col: AideonComputedCacheRef::ValidFrom,
                valid_to_col: AideonComputedCacheRef::ValidTo,
                rule_hash_col: AideonComputedCacheRef::RuleVersionHash,
                computed_at_col: AideonComputedCacheRef::ComputedAssertedAtHlc,
                value_col: id_col(backend, AideonComputedCacheRef::ValueRefEntityId, false),
                pk_name: "pk_aideon_computed_cache_ref",
            },
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            ComputedCacheTableSpec {
                table: AideonComputedCacheBlob::Table,
                partition_col: AideonComputedCacheBlob::PartitionId,
                entity_col: AideonComputedCacheBlob::EntityId,
                field_col: AideonComputedCacheBlob::FieldId,
                valid_from_col: AideonComputedCacheBlob::ValidFrom,
                valid_to_col: AideonComputedCacheBlob::ValidTo,
                rule_hash_col: AideonComputedCacheBlob::RuleVersionHash,
                computed_at_col: AideonComputedCacheBlob::ComputedAssertedAtHlc,
                value_col: ColumnDef::new(AideonComputedCacheBlob::ValueBlob)
                    .blob()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_computed_cache_blob",
            },
        )
        .await?;

        create_computed_cache_table(
            manager,
            backend,
            ComputedCacheTableSpec {
                table: AideonComputedCacheJson::Table,
                partition_col: AideonComputedCacheJson::PartitionId,
                entity_col: AideonComputedCacheJson::EntityId,
                field_col: AideonComputedCacheJson::FieldId,
                valid_from_col: AideonComputedCacheJson::ValidFrom,
                valid_to_col: AideonComputedCacheJson::ValidTo,
                rule_hash_col: AideonComputedCacheJson::RuleVersionHash,
                computed_at_col: AideonComputedCacheJson::ComputedAssertedAtHlc,
                value_col: ColumnDef::new(AideonComputedCacheJson::ValueJson)
                    .text()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_computed_cache_json",
            },
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            IndexFieldTableSpec {
                table: AideonIdxFieldStr::Table,
                partition_col: AideonIdxFieldStr::PartitionId,
                scenario_col: AideonIdxFieldStr::ScenarioId,
                field_col: AideonIdxFieldStr::FieldId,
                entity_col: AideonIdxFieldStr::EntityId,
                valid_from_col: AideonIdxFieldStr::ValidFrom,
                valid_to_col: AideonIdxFieldStr::ValidTo,
                valid_bucket_col: AideonIdxFieldStr::ValidBucket,
                asserted_col: AideonIdxFieldStr::AssertedAtHlc,
                layer_col: AideonIdxFieldStr::Layer,
                value_col: ColumnDef::new(AideonIdxFieldStr::ValueTextNorm)
                    .text()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_idx_field_str",
            },
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            IndexFieldTableSpec {
                table: AideonIdxFieldI64::Table,
                partition_col: AideonIdxFieldI64::PartitionId,
                scenario_col: AideonIdxFieldI64::ScenarioId,
                field_col: AideonIdxFieldI64::FieldId,
                entity_col: AideonIdxFieldI64::EntityId,
                valid_from_col: AideonIdxFieldI64::ValidFrom,
                valid_to_col: AideonIdxFieldI64::ValidTo,
                valid_bucket_col: AideonIdxFieldI64::ValidBucket,
                asserted_col: AideonIdxFieldI64::AssertedAtHlc,
                layer_col: AideonIdxFieldI64::Layer,
                value_col: ColumnDef::new(AideonIdxFieldI64::ValueI64)
                    .big_integer()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_idx_field_i64",
            },
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            IndexFieldTableSpec {
                table: AideonIdxFieldF64::Table,
                partition_col: AideonIdxFieldF64::PartitionId,
                scenario_col: AideonIdxFieldF64::ScenarioId,
                field_col: AideonIdxFieldF64::FieldId,
                entity_col: AideonIdxFieldF64::EntityId,
                valid_from_col: AideonIdxFieldF64::ValidFrom,
                valid_to_col: AideonIdxFieldF64::ValidTo,
                valid_bucket_col: AideonIdxFieldF64::ValidBucket,
                asserted_col: AideonIdxFieldF64::AssertedAtHlc,
                layer_col: AideonIdxFieldF64::Layer,
                value_col: ColumnDef::new(AideonIdxFieldF64::ValueF64)
                    .double()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_idx_field_f64",
            },
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            IndexFieldTableSpec {
                table: AideonIdxFieldBool::Table,
                partition_col: AideonIdxFieldBool::PartitionId,
                scenario_col: AideonIdxFieldBool::ScenarioId,
                field_col: AideonIdxFieldBool::FieldId,
                entity_col: AideonIdxFieldBool::EntityId,
                valid_from_col: AideonIdxFieldBool::ValidFrom,
                valid_to_col: AideonIdxFieldBool::ValidTo,
                valid_bucket_col: AideonIdxFieldBool::ValidBucket,
                asserted_col: AideonIdxFieldBool::AssertedAtHlc,
                layer_col: AideonIdxFieldBool::Layer,
                value_col: ColumnDef::new(AideonIdxFieldBool::ValueBool)
                    .boolean()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_idx_field_bool",
            },
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            IndexFieldTableSpec {
                table: AideonIdxFieldTime::Table,
                partition_col: AideonIdxFieldTime::PartitionId,
                scenario_col: AideonIdxFieldTime::ScenarioId,
                field_col: AideonIdxFieldTime::FieldId,
                entity_col: AideonIdxFieldTime::EntityId,
                valid_from_col: AideonIdxFieldTime::ValidFrom,
                valid_to_col: AideonIdxFieldTime::ValidTo,
                valid_bucket_col: AideonIdxFieldTime::ValidBucket,
                asserted_col: AideonIdxFieldTime::AssertedAtHlc,
                layer_col: AideonIdxFieldTime::Layer,
                value_col: ColumnDef::new(AideonIdxFieldTime::ValueTime)
                    .big_integer()
                    .not_null()
                    .to_owned(),
                pk_name: "pk_aideon_idx_field_time",
            },
        )
        .await?;

        create_index_field_table(
            manager,
            backend,
            IndexFieldTableSpec {
                table: AideonIdxFieldRef::Table,
                partition_col: AideonIdxFieldRef::PartitionId,
                scenario_col: AideonIdxFieldRef::ScenarioId,
                field_col: AideonIdxFieldRef::FieldId,
                entity_col: AideonIdxFieldRef::EntityId,
                valid_from_col: AideonIdxFieldRef::ValidFrom,
                valid_to_col: AideonIdxFieldRef::ValidTo,
                valid_bucket_col: AideonIdxFieldRef::ValidBucket,
                asserted_col: AideonIdxFieldRef::AssertedAtHlc,
                layer_col: AideonIdxFieldRef::Layer,
                value_col: id_col(backend, AideonIdxFieldRef::ValueRefEntityId, false),
                pk_name: "pk_aideon_idx_field_ref",
            },
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
                Hlc::now().as_i64().into(),
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

struct PropertyFactTableSpec<T: Iden + Clone> {
    table: T,
    partition_col: T,
    scenario_col: T,
    entity_col: T,
    field_col: T,
    valid_from_col: T,
    valid_to_col: T,
    valid_bucket_col: T,
    layer_col: T,
    asserted_col: T,
    op_col: T,
    tombstone_col: T,
    value_col: ColumnDef,
    pk_name: &'static str,
}

async fn create_property_fact_table<T: Iden + Clone>(
    manager: &SchemaManager<'_>,
    backend: DatabaseBackend,
    spec: PropertyFactTableSpec<T>,
) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(spec.table.clone())
                .if_not_exists()
                .col(id_col(backend, spec.partition_col.clone(), false))
                .col(id_col(backend, spec.scenario_col.clone(), true))
                .col(id_col(backend, spec.entity_col.clone(), false))
                .col(id_col(backend, spec.field_col.clone(), false))
                .col(
                    ColumnDef::new(spec.valid_from_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(spec.valid_to_col.clone()).big_integer())
                .col(ColumnDef::new(spec.valid_bucket_col.clone()).integer())
                .col(
                    ColumnDef::new(spec.layer_col.clone())
                        .tiny_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(spec.asserted_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .col(id_col(backend, spec.op_col.clone(), false))
                .col(
                    ColumnDef::new(spec.tombstone_col.clone())
                        .boolean()
                        .not_null(),
                )
                .col(spec.value_col)
                .primary_key(
                    Index::create()
                        .name(spec.pk_name)
                        .col(spec.partition_col)
                        .col(spec.entity_col)
                        .col(spec.field_col)
                        .col(spec.valid_from_col)
                        .col(spec.asserted_col)
                        .col(spec.op_col),
                )
                .to_owned(),
        )
        .await?;
    Ok(())
}

struct IndexFieldTableSpec<T: Iden + Clone> {
    table: T,
    partition_col: T,
    scenario_col: T,
    field_col: T,
    entity_col: T,
    valid_from_col: T,
    valid_to_col: T,
    valid_bucket_col: T,
    asserted_col: T,
    layer_col: T,
    value_col: ColumnDef,
    pk_name: &'static str,
}

async fn create_index_field_table<T: Iden + Clone>(
    manager: &SchemaManager<'_>,
    backend: DatabaseBackend,
    spec: IndexFieldTableSpec<T>,
) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(spec.table.clone())
                .if_not_exists()
                .col(id_col(backend, spec.partition_col.clone(), false))
                .col(id_col(backend, spec.scenario_col.clone(), true))
                .col(id_col(backend, spec.field_col.clone(), false))
                .col(spec.value_col)
                .col(id_col(backend, spec.entity_col.clone(), false))
                .col(
                    ColumnDef::new(spec.valid_from_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(spec.valid_to_col.clone()).big_integer())
                .col(ColumnDef::new(spec.valid_bucket_col.clone()).integer())
                .col(
                    ColumnDef::new(spec.asserted_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(spec.layer_col.clone())
                        .tiny_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .name(spec.pk_name)
                        .col(spec.partition_col)
                        .col(spec.field_col)
                        .col(spec.entity_col)
                        .col(spec.valid_from_col)
                        .col(spec.asserted_col),
                )
                .to_owned(),
        )
        .await?;
    Ok(())
}

struct ComputedCacheTableSpec<T: Iden + Clone> {
    table: T,
    partition_col: T,
    entity_col: T,
    field_col: T,
    valid_from_col: T,
    valid_to_col: T,
    rule_hash_col: T,
    computed_at_col: T,
    value_col: ColumnDef,
    pk_name: &'static str,
}

async fn create_computed_cache_table<T: Iden + Clone>(
    manager: &SchemaManager<'_>,
    backend: DatabaseBackend,
    spec: ComputedCacheTableSpec<T>,
) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(spec.table)
                .if_not_exists()
                .col(id_col(backend, spec.partition_col.clone(), false))
                .col(id_col(backend, spec.entity_col.clone(), false))
                .col(id_col(backend, spec.field_col.clone(), false))
                .col(
                    ColumnDef::new(spec.valid_from_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(spec.valid_to_col.clone()).big_integer())
                .col(spec.value_col)
                .col(
                    ColumnDef::new(spec.rule_hash_col.clone())
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(spec.computed_at_col.clone())
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .name(spec.pk_name)
                        .col(spec.partition_col)
                        .col(spec.entity_col)
                        .col(spec.field_col)
                        .col(spec.valid_from_col)
                        .col(spec.rule_hash_col),
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
