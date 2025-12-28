use sea_orm::sea_query;
use sea_orm_migration::prelude::Iden;

#[derive(Iden, Clone, Copy)]
pub enum AideonPartitions {
    Table,
    PartitionId,
    CreatedAtAsserted,
    CreatedByActor,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonActors {
    Table,
    PartitionId,
    ActorId,
    MetadataJson,
    CreatedAtAsserted,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonOps {
    Table,
    PartitionId,
    OpId,
    ActorId,
    AssertedAtHlc,
    TxId,
    OpType,
    Payload,
    SchemaVersionHint,
    IngestedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonOpDeps {
    Table,
    PartitionId,
    OpId,
    DepOpId,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonEntities {
    Table,
    PartitionId,
    ScenarioId,
    EntityId,
    EntityKind,
    TypeId,
    IsDeleted,
    CreatedOpId,
    UpdatedOpId,
    CreatedAssertedAtHlc,
    UpdatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonEdges {
    Table,
    PartitionId,
    ScenarioId,
    EdgeId,
    SrcEntityId,
    DstEntityId,
    EdgeTypeId,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonEdgeExistsFacts {
    Table,
    PartitionId,
    ScenarioId,
    EdgeId,
    ValidFrom,
    ValidTo,
    Layer,
    AssertedAtHlc,
    OpId,
    IsTombstone,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonPropFactStr {
    Table,
    PartitionId,
    ScenarioId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    Layer,
    AssertedAtHlc,
    OpId,
    IsTombstone,
    ValueText,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonPropFactI64 {
    Table,
    PartitionId,
    ScenarioId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    Layer,
    AssertedAtHlc,
    OpId,
    IsTombstone,
    ValueI64,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonPropFactF64 {
    Table,
    PartitionId,
    ScenarioId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    Layer,
    AssertedAtHlc,
    OpId,
    IsTombstone,
    ValueF64,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonPropFactBool {
    Table,
    PartitionId,
    ScenarioId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    Layer,
    AssertedAtHlc,
    OpId,
    IsTombstone,
    ValueBool,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonPropFactTime {
    Table,
    PartitionId,
    ScenarioId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    Layer,
    AssertedAtHlc,
    OpId,
    IsTombstone,
    ValueTime,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonPropFactRef {
    Table,
    PartitionId,
    ScenarioId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    Layer,
    AssertedAtHlc,
    OpId,
    IsTombstone,
    ValueRefEntityId,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonPropFactBlob {
    Table,
    PartitionId,
    ScenarioId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    Layer,
    AssertedAtHlc,
    OpId,
    IsTombstone,
    ValueBlob,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonPropFactJson {
    Table,
    PartitionId,
    ScenarioId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    Layer,
    AssertedAtHlc,
    OpId,
    IsTombstone,
    ValueJson,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonTypes {
    Table,
    PartitionId,
    TypeId,
    AppliesTo,
    Label,
    IsAbstract,
    IsDeleted,
    UpdatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonTypeExtends {
    Table,
    PartitionId,
    TypeId,
    ParentTypeId,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonFields {
    Table,
    PartitionId,
    FieldId,
    ValueType,
    Cardinality,
    MergePolicy,
    IsIndexed,
    Label,
    IsDeleted,
    UpdatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonTypeFields {
    Table,
    PartitionId,
    TypeId,
    FieldId,
    IsRequired,
    DefaultValueKind,
    DefaultValueStr,
    DefaultValueI64,
    DefaultValueF64,
    DefaultValueBool,
    DefaultValueTime,
    DefaultValueRef,
    DefaultValueBlob,
    DefaultValueJson,
    OverrideDefault,
    TightenRequired,
    UpdatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonEffectiveSchemaCache {
    Table,
    PartitionId,
    TypeId,
    SchemaVersionHash,
    Blob,
    BuiltAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonEdgeTypeRules {
    Table,
    PartitionId,
    EdgeTypeId,
    AllowedSrcTypeIdsJson,
    AllowedDstTypeIdsJson,
    SemanticDirection,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonGraphProjectionEdges {
    Table,
    PartitionId,
    ScenarioId,
    EdgeId,
    SrcEntityId,
    DstEntityId,
    EdgeTypeId,
    Weight,
    UpdatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonPagerankRuns {
    Table,
    PartitionId,
    RunId,
    AsOfValidTime,
    AsOfAssertedAtHlc,
    ParamsJson,
    CreatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonPagerankScores {
    Table,
    PartitionId,
    RunId,
    EntityId,
    Score,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonIdxFieldStr {
    Table,
    PartitionId,
    ScenarioId,
    FieldId,
    ValueTextNorm,
    EntityId,
    ValidFrom,
    ValidTo,
    AssertedAtHlc,
    Layer,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonIdxFieldI64 {
    Table,
    PartitionId,
    ScenarioId,
    FieldId,
    ValueI64,
    EntityId,
    ValidFrom,
    ValidTo,
    AssertedAtHlc,
    Layer,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonIdxFieldF64 {
    Table,
    PartitionId,
    ScenarioId,
    FieldId,
    ValueF64,
    EntityId,
    ValidFrom,
    ValidTo,
    AssertedAtHlc,
    Layer,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonIdxFieldBool {
    Table,
    PartitionId,
    ScenarioId,
    FieldId,
    ValueBool,
    EntityId,
    ValidFrom,
    ValidTo,
    AssertedAtHlc,
    Layer,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonIdxFieldTime {
    Table,
    PartitionId,
    ScenarioId,
    FieldId,
    ValueTime,
    EntityId,
    ValidFrom,
    ValidTo,
    AssertedAtHlc,
    Layer,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonIdxFieldRef {
    Table,
    PartitionId,
    ScenarioId,
    FieldId,
    ValueRefEntityId,
    EntityId,
    ValidFrom,
    ValidTo,
    AssertedAtHlc,
    Layer,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonSchemaVersion {
    Table,
    Version,
    AppliedAssertedAtHlc,
    Checksum,
    AppVersion,
}
