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
pub enum AideonHlcState {
    Table,
    PartitionId,
    LastHlc,
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
pub enum AideonChangeFeed {
    Table,
    PartitionId,
    Sequence,
    OpId,
    AssertedAtHlc,
    EntityId,
    ChangeKind,
    PayloadJson,
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
    AclGroupId,
    OwnerActorId,
    Visibility,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
pub enum AideonMetamodelVersions {
    Table,
    PartitionId,
    Version,
    Source,
    OpId,
    CreatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonValidationRules {
    Table,
    PartitionId,
    RuleId,
    ScopeKind,
    ScopeId,
    Severity,
    TemplateKind,
    ParamsJson,
    UpdatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonComputedRules {
    Table,
    PartitionId,
    RuleId,
    TargetTypeId,
    OutputFieldId,
    TemplateKind,
    ParamsJson,
    UpdatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonComputedCacheStr {
    Table,
    PartitionId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    ValueText,
    RuleVersionHash,
    ComputedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonComputedCacheI64 {
    Table,
    PartitionId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    ValueI64,
    RuleVersionHash,
    ComputedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonComputedCacheF64 {
    Table,
    PartitionId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    ValueF64,
    RuleVersionHash,
    ComputedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonComputedCacheBool {
    Table,
    PartitionId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    ValueBool,
    RuleVersionHash,
    ComputedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonComputedCacheTime {
    Table,
    PartitionId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    ValueTime,
    RuleVersionHash,
    ComputedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonComputedCacheRef {
    Table,
    PartitionId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    ValueRefEntityId,
    RuleVersionHash,
    ComputedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonComputedCacheBlob {
    Table,
    PartitionId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    ValueBlob,
    RuleVersionHash,
    ComputedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonComputedCacheJson {
    Table,
    PartitionId,
    EntityId,
    FieldId,
    ValidFrom,
    ValidTo,
    ValueJson,
    RuleVersionHash,
    ComputedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonJobs {
    Table,
    PartitionId,
    JobId,
    JobType,
    Status,
    Priority,
    Attempts,
    MaxAttempts,
    LeaseExpiresAt,
    NextRunAfter,
    CreatedAssertedAtHlc,
    UpdatedAssertedAtHlc,
    Payload,
    DedupeKey,
    LastError,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonJobEvents {
    Table,
    PartitionId,
    JobId,
    EventTime,
    Message,
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
pub enum AideonGraphDegreeStats {
    Table,
    PartitionId,
    ScenarioId,
    AsOfValidTime,
    EntityId,
    OutDegree,
    InDegree,
    ComputedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonGraphEdgeTypeCounts {
    Table,
    PartitionId,
    ScenarioId,
    EdgeTypeId,
    Count,
    ComputedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonIntegrityRuns {
    Table,
    PartitionId,
    RunId,
    ScenarioId,
    AsOfValidTime,
    AsOfAssertedAtHlc,
    ParamsJson,
    CreatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonIntegrityFindings {
    Table,
    PartitionId,
    RunId,
    FindingType,
    Severity,
    SubjectEntityId,
    DetailsJson,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonIntegrityHead {
    Table,
    PartitionId,
    ScenarioId,
    RunId,
    UpdatedAssertedAtHlc,
}

#[derive(Iden, Clone, Copy)]
pub enum AideonTypeSchemaHead {
    Table,
    PartitionId,
    TypeId,
    SchemaVersionHash,
    UpdatedAssertedAtHlc,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
    ValidBucket,
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
