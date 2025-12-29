import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('workspaces/mneme/platform', () => ({ isTauri: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { isTauri } from 'workspaces/mneme/platform';

import {
  createEdge,
  createNode,
  compileEffectiveSchema,
  getEffectiveSchema,
  listEdgeTypeRules,
  counterUpdate,
  orSetUpdate,
  setPropertyInterval,
  clearPropertyInterval,
  setEdgeExistenceInterval,
  listEntities,
  getProjectionEdges,
  getGraphDegreeStats,
  getGraphEdgeTypeCounts,
  storePageRankRun,
  getPageRankScores,
  exportOps,
  exportOpsStream,
  ingestOps,
  importOpsStream,
  getPartitionHead,
  createScenario,
  deleteScenario,
  exportSnapshotStream,
  importSnapshotStream,
  upsertValidationRules,
  listValidationRules,
  upsertComputedRules,
  listComputedRules,
  upsertComputedCache,
  listComputedCache,
  listJobs,
  runProcessingWorker,
  triggerCompaction,
  triggerRebuildEffectiveSchema,
  triggerRefreshAnalyticsProjections,
  triggerRefreshIntegrity,
  triggerRetention,
  getChangesSince,
  subscribePartition,
  unsubscribePartition,
  onChangeEvents,
  getIntegrityHead,
  getLastSchemaCompile,
  listFailedJobs,
  getSchemaManifest,
  explainResolution,
  explainTraversal,
  readEntityAtTime,
  traverseAtTime,
  tombstoneEntity,
  upsertMetamodelBatch,
} from 'workspaces/mneme/mneme-api';

const isTauriMock = vi.mocked(isTauri);
const invokeMock = vi.mocked(invoke);
const listenMock = vi.mocked(listen);

describe('mneme-api metamodel bindings', () => {
  beforeEach(() => {
    isTauriMock.mockReturnValue(true);
    invokeMock.mockReset();
    listenMock.mockReset();
  });

  it('maps metamodel batch payloads to rust shapes', async () => {
    invokeMock.mockResolvedValue({ opId: 'op-1' });
    await upsertMetamodelBatch({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      batch: {
        types: [
          {
            typeId: 't-1',
            appliesTo: 'Node',
            label: 'Capability',
            isAbstract: false,
          },
        ],
        fields: [
          {
            fieldId: 'f-1',
            label: 'name',
            valueType: 'str',
            cardinality: 'single',
            mergePolicy: 'LWW',
            indexed: true,
          },
        ],
        typeFields: [
          {
            typeId: 't-1',
            fieldId: 'f-1',
            required: true,
          },
        ],
        edgeTypeRules: [
          {
            edgeTypeId: 'e-1',
            semanticDirection: 'supports',
            allowedSrcTypeIds: ['t-1'],
            allowedDstTypeIds: ['t-2'],
          },
        ],
      },
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_upsert_metamodel_batch', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      scenarioId: undefined,
      batch: {
        types: [
          {
            type_id: 't-1',
            applies_to: 'Node',
            label: 'Capability',
            is_abstract: false,
            parent_type_id: undefined,
          },
        ],
        fields: [
          {
            field_id: 'f-1',
            label: 'name',
            value_type: 'Str',
            cardinality_multi: false,
            merge_policy: 'Lww',
            is_indexed: true,
          },
        ],
        type_fields: [
          {
            type_id: 't-1',
            field_id: 'f-1',
            is_required: true,
            default_value: undefined,
            override_default: undefined,
            tighten_required: undefined,
          },
        ],
        edge_type_rules: [
          {
            edge_type_id: 'e-1',
            semantic_direction: 'supports',
            allowed_src_type_ids: ['t-1'],
            allowed_dst_type_ids: ['t-2'],
          },
        ],
        metamodel_version: undefined,
        metamodel_source: undefined,
      },
    });
  });

  it('returns a mock schema hash outside Tauri', async () => {
    isTauriMock.mockReturnValue(false);
    const result = await compileEffectiveSchema({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      typeId: 't-1',
    });
    expect(result.schemaVersionHash).toBe('mock-schema-hash');
  });

  it('maps effective schema results from rust to ts', async () => {
    invokeMock.mockResolvedValue({
      type_id: 't-1',
      applies_to: 'Node',
      fields: [
        {
          field_id: 'f-1',
          value_type: 'Str',
          cardinality_multi: true,
          merge_policy: 'Lww',
          is_required: true,
          default_value: 'Cap',
          is_indexed: false,
          disallow_overlap: false,
        },
      ],
    });

    const schema = await getEffectiveSchema('p-1', 't-1');

    expect(schema).toEqual({
      typeId: 't-1',
      appliesTo: 'Node',
      fields: [
        {
          fieldId: 'f-1',
          valueType: 'str',
          cardinality: 'multi',
          mergePolicy: 'LWW',
          required: true,
          defaultValue: 'Cap',
          indexed: false,
          disallowOverlap: false,
        },
      ],
    });
  });

  it('maps edge type rules from rust to ts', async () => {
    invokeMock.mockResolvedValue([
      {
        edge_type_id: 'e-1',
        semantic_direction: 'supports',
        allowed_src_type_ids: ['t-1'],
        allowed_dst_type_ids: ['t-2'],
      },
    ]);

    const rules = await listEdgeTypeRules('p-1');

    expect(rules).toEqual([
      {
        edgeTypeId: 'e-1',
        semanticDirection: 'supports',
        allowedSrcTypeIds: ['t-1'],
        allowedDstTypeIds: ['t-2'],
      },
    ]);
  });

  it('passes create node inputs through to the host', async () => {
    invokeMock.mockResolvedValue({ opId: 'op-node' });

    await createNode({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      nodeId: 'n-1',
      typeId: 't-1',
      scenarioId: 's-1',
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_create_node', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      nodeId: 'n-1',
      typeId: 't-1',
      scenarioId: 's-1',
    });
  });

  it('passes create edge inputs through to the host', async () => {
    invokeMock.mockResolvedValue({ opId: 'op-edge' });

    await createEdge({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      edgeId: 'e-1',
      typeId: 'et-1',
      srcId: 'n-1',
      dstId: 'n-2',
      existsValidFrom: '2025-01-01T00:00:00Z',
      existsValidTo: '2025-12-31T00:00:00Z',
      layer: 'Plan',
      weight: 0.5,
      scenarioId: 's-1',
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_create_edge', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      edgeId: 'e-1',
      typeId: 'et-1',
      srcId: 'n-1',
      dstId: 'n-2',
      existsValidFrom: '2025-01-01T00:00:00Z',
      existsValidTo: '2025-12-31T00:00:00Z',
      layer: 'Plan',
      weight: 0.5,
      scenarioId: 's-1',
    });
  });

  it('passes edge existence interval updates through to the host', async () => {
    invokeMock.mockResolvedValue({ opId: 'op-exists' });

    await setEdgeExistenceInterval({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      edgeId: 'e-1',
      validFrom: '2025-02-01T00:00:00Z',
      validTo: '2025-03-01T00:00:00Z',
      isTombstone: true,
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_set_edge_existence_interval', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      edgeId: 'e-1',
      validFrom: '2025-02-01T00:00:00Z',
      validTo: '2025-03-01T00:00:00Z',
      isTombstone: true,
    });
  });

  it('passes tombstone entity inputs through to the host', async () => {
    invokeMock.mockResolvedValue({ opId: 'op-tomb' });

    await tombstoneEntity({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      entityId: 'n-1',
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_tombstone_entity', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      entityId: 'n-1',
    });
  });

  it('converts property values to rust enum shapes', async () => {
    invokeMock.mockResolvedValue({ opId: 'op-prop' });
    const validFrom = '2025-01-01T00:00:00Z';

    await setPropertyInterval({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      entityId: 'n-1',
      fieldId: 'f-1',
      value: { t: 'time', v: validFrom },
      validFrom,
      layer: 'Actual',
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_set_property_interval', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      entityId: 'n-1',
      fieldId: 'f-1',
      value: { Time: new Date(validFrom).getTime() * 1000 },
      validFrom,
      validTo: undefined,
      layer: 'Actual',
      scenarioId: undefined,
    });
  });

  it('passes clear property interval inputs through to the host', async () => {
    invokeMock.mockResolvedValue({ opId: 'op-clear' });

    await clearPropertyInterval({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      entityId: 'n-1',
      fieldId: 'f-1',
      validFrom: '2025-01-01T00:00:00Z',
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_clear_property_interval', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      entityId: 'n-1',
      fieldId: 'f-1',
      validFrom: '2025-01-01T00:00:00Z',
      validTo: undefined,
      layer: undefined,
      scenarioId: undefined,
    });
  });

  it('converts OR-set elements to rust enum shapes', async () => {
    invokeMock.mockResolvedValue({ opId: 'op-or' });

    await orSetUpdate({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      entityId: 'n-1',
      fieldId: 'f-1',
      op: 'Add',
      element: { t: 'i64', v: 42n },
      validFrom: '2025-01-01T00:00:00Z',
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_or_set_update', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      entityId: 'n-1',
      fieldId: 'f-1',
      op: 'Add',
      element: { I64: 42 },
      validFrom: '2025-01-01T00:00:00Z',
      validTo: undefined,
      layer: undefined,
      scenarioId: undefined,
    });
  });

  it('passes counter update inputs through to the host', async () => {
    invokeMock.mockResolvedValue({ opId: 'op-counter' });

    await counterUpdate({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      entityId: 'n-1',
      fieldId: 'f-1',
      delta: 5,
      validFrom: '2025-01-01T00:00:00Z',
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_counter_update', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      entityId: 'n-1',
      fieldId: 'f-1',
      delta: 5,
      validFrom: '2025-01-01T00:00:00Z',
      validTo: undefined,
      layer: undefined,
      scenarioId: undefined,
    });
  });

  it('maps read entity results from rust to ts', async () => {
    invokeMock.mockResolvedValue({
      entity_id: 'n-1',
      kind: 'Node',
      type_id: 't-1',
      is_deleted: false,
      properties: {
        'f-1': { Single: { Str: 'alpha' } },
        'f-2': { Multi: [{ I64: 7 }, { I64: 8 }] },
        'f-3': {
          MultiLimited: { values: [{ Bool: true }], more_available: true },
        },
      },
    });

    const result = await readEntityAtTime({
      partitionId: 'p-1',
      entityId: 'n-1',
      at: '2025-01-01T00:00:00Z',
    });

    expect(result).toEqual({
      entityId: 'n-1',
      kind: 'Node',
      typeId: 't-1',
      isDeleted: false,
      properties: {
        'f-1': { k: 'single', v: { t: 'str', v: 'alpha' } },
        'f-2': {
          k: 'multi',
          v: [
            { t: 'i64', v: 7n },
            { t: 'i64', v: 8n },
          ],
        },
        'f-3': {
          k: 'multi_limited',
          v: { values: [{ t: 'bool', v: true }], moreAvailable: true },
        },
      },
    });
  });

  it('maps traverse results from rust to ts', async () => {
    invokeMock.mockResolvedValue([
      { edge_id: 'e-1', src_id: 'n-1', dst_id: 'n-2', type_id: 'et-1' },
    ]);

    const edges = await traverseAtTime({
      partitionId: 'p-1',
      fromEntityId: 'n-1',
      direction: 'out',
      at: '2025-01-01T00:00:00Z',
    });

    expect(edges).toEqual([
      { edgeId: 'e-1', srcId: 'n-1', dstId: 'n-2', edgeTypeId: 'et-1' },
    ]);
  });

  it('maps list entity inputs and results', async () => {
    invokeMock.mockResolvedValue([{ entity_id: 'n-1', kind: 'Node', type_id: 't-1' }]);

    const results = await listEntities({
      partitionId: 'p-1',
      at: '2025-01-01T00:00:00Z',
      filters: [{ fieldId: 'f-1', op: 'Eq', value: { t: 'str', v: 'alpha' } }],
      limit: 5,
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_list_entities', {
      partitionId: 'p-1',
      at: '2025-01-01T00:00:00Z',
      filters: [{ fieldId: 'f-1', op: 'Eq', value: { Str: 'alpha' } }],
      limit: 5,
    });
    expect(results).toEqual([{ entityId: 'n-1', kind: 'Node', typeId: 't-1' }]);
  });

  it('maps projection edges from rust to ts', async () => {
    invokeMock.mockResolvedValue([
      {
        edge_id: 'e-1',
        src_id: 'n-1',
        dst_id: 'n-2',
        edge_type_id: 't-1',
        weight: 0.8,
      },
    ]);

    const edges = await getProjectionEdges({ partitionId: 'p-1' });

    expect(edges).toEqual([
      { edgeId: 'e-1', srcId: 'n-1', dstId: 'n-2', edgeTypeId: 't-1', weight: 0.8 },
    ]);
  });

  it('maps degree stats from rust to ts', async () => {
    invokeMock.mockResolvedValue([
      {
        entity_id: 'n-1',
        out_degree: 2,
        in_degree: 1,
        as_of_valid_time: 1735689600000000,
        computed_asserted_at: 555,
      },
    ]);

    const stats = await getGraphDegreeStats({ partitionId: 'p-1' });

    expect(stats[0]).toMatchObject({
      entityId: 'n-1',
      outDegree: 2,
      inDegree: 1,
      computedAssertedAt: '555',
    });
    expect(stats[0]?.asOfValidTime).toBe('2025-01-01T00:00:00.000Z');
  });

  it('maps edge type counts from rust to ts', async () => {
    invokeMock.mockResolvedValue([
      { edge_type_id: 't-1', count: 4, computed_asserted_at: 777 },
    ]);

    const counts = await getGraphEdgeTypeCounts({ partitionId: 'p-1' });

    expect(counts).toEqual([
      { edgeTypeId: 't-1', count: 4, computedAssertedAt: '777' },
    ]);
  });

  it('stores pagerank runs with seed conversion', async () => {
    invokeMock.mockResolvedValue({ runId: 'r-1' });

    const result = await storePageRankRun({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      params: {
        damping: 0.85,
        maxIters: 20,
        tol: 0.0001,
        personalisedSeed: [{ id: 'n-1', w: 0.5 }],
      },
      scores: [{ id: 'n-1', score: 1.0 }],
    });

    expect(result.runId).toBe('r-1');
    expect(invokeMock).toHaveBeenCalledWith('mneme_store_pagerank_scores', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      params: {
        damping: 0.85,
        maxIters: 20,
        tol: 0.0001,
        personalisedSeed: [{ id: 'n-1', weight: 0.5 }],
      },
      scores: [{ id: 'n-1', score: 1.0 }],
    });
  });

  it('returns pagerank scores', async () => {
    invokeMock.mockResolvedValue([{ id: 'n-1', score: 0.9 }]);

    const scores = await getPageRankScores({ partitionId: 'p-1', runId: 'r-1', topN: 3 });

    expect(scores).toEqual([{ id: 'n-1', score: 0.9 }]);
  });

  it('exports ops and normalizes payloads', async () => {
    invokeMock.mockResolvedValue([
      {
        op_id: 'op-1',
        actor_id: 'a-1',
        asserted_at: 123,
        op_type: 7,
        payload: [1, 2, 3],
        deps: ['op-0'],
      },
    ]);

    const result = await exportOps({ partitionId: 'p-1' });

    expect(result.ops[0]).toMatchObject({
      opId: 'op-1',
      actorId: 'a-1',
      assertedAt: '123',
      opType: 7,
      deps: ['op-0'],
    });
    expect(Array.from(result.ops[0]?.payload ?? [])).toEqual([1, 2, 3]);
  });

  it('exports op streams and yields records', async () => {
    invokeMock.mockResolvedValue([
      { record_type: 'header', data: { version: 1 } },
      { record_type: 'op', data: { opId: 'o-1' } },
    ]);

    const iterable = await exportOpsStream({ partitionId: 'p-1' });
    const records: Array<{ recordType: string; data: unknown }> = [];
    for await (const record of iterable) {
      records.push(record);
    }

    expect(records).toEqual([
      { recordType: 'header', data: { version: 1 } },
      { recordType: 'op', data: { opId: 'o-1' } },
    ]);
    expect(invokeMock).toHaveBeenCalledWith('mneme_export_ops_stream', {
      partitionId: 'p-1',
    });
  });

  it('ingests ops with byte payload conversion', async () => {
    invokeMock.mockResolvedValue(undefined);

    await ingestOps({
      partitionId: 'p-1',
      ops: [
        {
          opId: 'op-1',
          actorId: 'a-1',
          assertedAt: '123',
          opType: 7,
          payload: new Uint8Array([9, 8]),
          deps: [],
        },
      ],
    });

    expect(invokeMock).toHaveBeenCalledWith('mneme_ingest_ops', {
      partitionId: 'p-1',
      ops: [
        {
          opId: 'op-1',
          actorId: 'a-1',
          assertedAt: '123',
          opType: 7,
          payload: [9, 8],
          deps: [],
        },
      ],
    });
  });

  it('imports op streams and maps reports', async () => {
    invokeMock.mockResolvedValue({ ops_imported: 2, ops_skipped: 1, errors: 0 });

    async function* records() {
      yield { recordType: 'op', data: { opId: 'o-1' } };
    }

    const report = await importOpsStream(
      {
        targetPartition: 'p-1',
        allowPartitionCreate: true,
        strictSchema: true,
      },
      records(),
    );

    expect(report).toEqual({ opsImported: 2, opsSkipped: 1, errors: 0 });
    expect(invokeMock).toHaveBeenCalledWith('mneme_import_ops_stream', {
      targetPartition: 'p-1',
      allowPartitionCreate: true,
      strictSchema: true,
      records: [{ record_type: 'op', data: { opId: 'o-1' } }],
    });
  });

  it('exports snapshot streams', async () => {
    invokeMock.mockResolvedValue([{ record_type: 'header', data: { snap: true } }]);

    const iterable = await exportSnapshotStream({
      partitionId: 'p-1',
      asOfAssertedAt: '123',
    });
    const records: Array<{ recordType: string; data: unknown }> = [];
    for await (const record of iterable) {
      records.push(record);
    }

    expect(records).toEqual([{ recordType: 'header', data: { snap: true } }]);
    expect(invokeMock).toHaveBeenCalledWith('mneme_export_snapshot_stream', {
      partitionId: 'p-1',
      asOfAssertedAt: '123',
    });
  });

  it('imports snapshot streams', async () => {
    invokeMock.mockResolvedValue(undefined);

    async function* records() {
      yield { recordType: 'header', data: { snap: true } };
    }

    await importSnapshotStream(
      {
        targetPartition: 'p-1',
        allowPartitionCreate: false,
      },
      records(),
    );

    expect(invokeMock).toHaveBeenCalledWith('mneme_import_snapshot_stream', {
      targetPartition: 'p-1',
      allowPartitionCreate: false,
      records: [{ record_type: 'header', data: { snap: true } }],
    });
  });

  it('upserts and lists validation rules', async () => {
    invokeMock
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce([
        {
          rule_id: 'r-1',
          scope_kind: 2,
          scope_id: 't-1',
          severity: 1,
          template_kind: 'required',
          params: { field: 'name' },
        },
      ]);

    await upsertValidationRules({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      rules: [
        {
          ruleId: 'r-1',
          scopeKind: 2,
          scopeId: 't-1',
          severity: 1,
          templateKind: 'required',
          params: { field: 'name' },
        },
      ],
    });

    const rules = await listValidationRules('p-1');

    expect(rules).toEqual([
      {
        ruleId: 'r-1',
        scopeKind: 2,
        scopeId: 't-1',
        severity: 1,
        templateKind: 'required',
        params: { field: 'name' },
      },
    ]);
    expect(invokeMock).toHaveBeenCalledWith('mneme_upsert_validation_rules', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      rules: [
        {
          rule_id: 'r-1',
          scope_kind: 2,
          scope_id: 't-1',
          severity: 1,
          template_kind: 'required',
          params: { field: 'name' },
        },
      ],
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_list_validation_rules', {
      partitionId: 'p-1',
    });
  });

  it('upserts and lists computed rules', async () => {
    invokeMock
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce([
        {
          rule_id: 'c-1',
          target_type_id: 't-1',
          output_field_id: 'f-1',
          template_kind: 'sum',
          params: { fields: ['f-2'] },
        },
      ]);

    await upsertComputedRules({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '555',
      rules: [
        {
          ruleId: 'c-1',
          targetTypeId: 't-1',
          outputFieldId: 'f-1',
          templateKind: 'sum',
          params: { fields: ['f-2'] },
        },
      ],
    });

    const rules = await listComputedRules('p-1');

    expect(rules).toEqual([
      {
        ruleId: 'c-1',
        targetTypeId: 't-1',
        outputFieldId: 'f-1',
        templateKind: 'sum',
        params: { fields: ['f-2'] },
      },
    ]);
    expect(invokeMock).toHaveBeenCalledWith('mneme_upsert_computed_rules', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '555',
      rules: [
        {
          rule_id: 'c-1',
          target_type_id: 't-1',
          output_field_id: 'f-1',
          template_kind: 'sum',
          params: { fields: ['f-2'] },
        },
      ],
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_list_computed_rules', {
      partitionId: 'p-1',
    });
  });

  it('upserts and lists computed cache entries', async () => {
    invokeMock
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce([
        {
          entity_id: 'n-1',
          field_id: 'f-1',
          valid_from: 1_000_000,
          valid_to: null,
          value: { Str: 'value' },
          rule_version_hash: 'hash-1',
          computed_asserted_at: 321,
        },
      ]);

    await upsertComputedCache({
      partitionId: 'p-1',
      entries: [
        {
          entityId: 'n-1',
          fieldId: 'f-1',
          validFrom: '2024-01-01T00:00:00.000Z',
          value: { t: 'str', v: 'value' },
          ruleVersionHash: 'hash-1',
          computedAssertedAt: '321',
        },
      ],
    });

    const entries = await listComputedCache({
      partitionId: 'p-1',
      fieldId: 'f-1',
      limit: 25,
    });

    expect(entries).toEqual([
      {
        entityId: 'n-1',
        fieldId: 'f-1',
        validFrom: '1970-01-01T00:00:01.000Z',
        validTo: undefined,
        value: { t: 'str', v: 'value' },
        ruleVersionHash: 'hash-1',
        computedAssertedAt: '321',
      },
    ]);
    expect(invokeMock).toHaveBeenCalledWith('mneme_upsert_computed_cache', {
      partitionId: 'p-1',
      entries: [
        {
          entity_id: 'n-1',
          field_id: 'f-1',
          valid_from: '2024-01-01T00:00:00.000Z',
          valid_to: null,
          value: { Str: 'value' },
          rule_version_hash: 'hash-1',
          computed_asserted_at: '321',
        },
      ],
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_list_computed_cache', {
      partitionId: 'p-1',
      fieldId: 'f-1',
      limit: 25,
    });
  });

  it('fetches partition head', async () => {
    invokeMock.mockResolvedValue({ head: '999' });

    const result = await getPartitionHead('p-1');

    expect(result.head).toBe('999');
    expect(invokeMock).toHaveBeenCalledWith('mneme_get_partition_head', { partitionId: 'p-1' });
  });

  it('creates and deletes scenarios', async () => {
    invokeMock.mockResolvedValueOnce('s-1').mockResolvedValueOnce(undefined);

    const scenarioId = await createScenario({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      name: 'Plan A',
    });
    await deleteScenario({
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '124',
      scenarioId: 's-1',
    });

    expect(scenarioId).toBe('s-1');
    expect(invokeMock).toHaveBeenCalledWith('mneme_create_scenario', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '123',
      name: 'Plan A',
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_delete_scenario', {
      partitionId: 'p-1',
      actorId: 'a-1',
      assertedAt: '124',
      scenarioId: 's-1',
    });
  });

  it('triggers processing jobs', async () => {
    invokeMock.mockResolvedValue(undefined);

    await triggerRebuildEffectiveSchema({ partitionId: 'p-1', reason: 'rebuild' });
    await triggerRefreshIntegrity({ partitionId: 'p-1', reason: 'integrity' });
    await triggerRefreshAnalyticsProjections({ partitionId: 'p-1', reason: 'analytics' });
    await triggerRetention({
      partitionId: 'p-1',
      reason: 'cleanup',
      policy: { keepOpsDays: 30 },
    });
    await triggerCompaction({ partitionId: 'p-1', reason: 'compact' });

    expect(invokeMock).toHaveBeenCalledWith('mneme_trigger_rebuild_effective_schema', {
      partitionId: 'p-1',
      reason: 'rebuild',
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_trigger_refresh_integrity', {
      partitionId: 'p-1',
      reason: 'integrity',
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_trigger_refresh_analytics_projections', {
      partitionId: 'p-1',
      reason: 'analytics',
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_trigger_retention', {
      partitionId: 'p-1',
      reason: 'cleanup',
      policy: { keepOpsDays: 30 },
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_trigger_compaction', {
      partitionId: 'p-1',
      reason: 'compact',
    });
  });

  it('runs processing workers and maps job summaries', async () => {
    invokeMock
      .mockResolvedValueOnce({ jobsProcessed: 2 })
      .mockResolvedValueOnce([
        {
          partition: 'p-1',
          job_id: 'j-1',
          job_type: 'schema',
          status: 1,
          priority: 5,
          attempts: 0,
          max_attempts: 3,
          lease_expires_at: null,
          next_run_after: null,
          created_asserted_at: 11,
          updated_asserted_at: 22,
          dedupe_key: null,
          last_error: null,
        },
      ]);

    const workerResult = await runProcessingWorker({ maxJobs: 5, leaseMillis: 1000 });
    const jobs = await listJobs({ partitionId: 'p-1', limit: 10 });

    expect(workerResult.jobsProcessed).toBe(2);
    expect(jobs[0]).toMatchObject({
      partitionId: 'p-1',
      jobId: 'j-1',
      jobType: 'schema',
      status: 1,
      priority: 5,
      attempts: 0,
      maxAttempts: 3,
      createdAssertedAt: '11',
      updatedAssertedAt: '22',
    });
  });

  it('maps change feed events from rust to ts', async () => {
    invokeMock.mockResolvedValue([
      {
        partition: 'p-1',
        sequence: 10,
        op_id: 'op-1',
        asserted_at: 101,
        entity_id: 'n-1',
        change_kind: 2,
        payload: { note: 'changed' },
      },
    ]);

    const events = await getChangesSince({ partitionId: 'p-1' });

    expect(events).toEqual([
      {
        partitionId: 'p-1',
        sequence: 10,
        opId: 'op-1',
        assertedAt: '101',
        entityId: 'n-1',
        changeKind: 2,
        payload: { note: 'changed' },
      },
    ]);
  });

  it('subscribes and unsubscribes to change feeds', async () => {
    invokeMock.mockResolvedValueOnce({ subscriptionId: 'sub-1' }).mockResolvedValueOnce(true);

    const sub = await subscribePartition({ partitionId: 'p-1' });
    const ok = await unsubscribePartition({ subscriptionId: 'sub-1' });

    expect(sub.subscriptionId).toBe('sub-1');
    expect(ok).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith('mneme_subscribe_partition', {
      partitionId: 'p-1',
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_unsubscribe_partition', {
      subscriptionId: 'sub-1',
    });
  });

  it('registers change event listeners', async () => {
    listenMock.mockResolvedValue(() => {});

    const unlisten = await onChangeEvents(() => {});

    expect(listenMock).toHaveBeenCalledWith('mneme_change_event', expect.any(Function));
    expect(typeof unlisten).toBe('function');
  });

  it('maps integrity heads from rust', async () => {
    invokeMock.mockResolvedValue({
      partition: 'p-1',
      scenario_id: null,
      run_id: 'run-1',
      updated_asserted_at: 90210,
    });

    const result = await getIntegrityHead('p-1');

    expect(result).toEqual({
      partitionId: 'p-1',
      scenarioId: undefined,
      runId: 'run-1',
      updatedAssertedAt: '90210',
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_get_integrity_head', {
      partitionId: 'p-1',
      scenarioId: undefined,
    });
  });

  it('maps schema compile heads from rust', async () => {
    invokeMock.mockResolvedValue({
      partition: 'p-1',
      type_id: 't-1',
      schema_version_hash: 'hash-1',
      updated_asserted_at: 12,
    });

    const result = await getLastSchemaCompile('p-1', 't-1');

    expect(result).toEqual({
      partitionId: 'p-1',
      typeId: 't-1',
      schemaVersionHash: 'hash-1',
      updatedAssertedAt: '12',
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_get_last_schema_compile', {
      partitionId: 'p-1',
      typeId: 't-1',
    });
  });

  it('lists failed jobs', async () => {
    invokeMock.mockResolvedValue([
      {
        partition: 'p-1',
        job_id: 'j-1',
        job_type: 'schema',
        status: 3,
        priority: 1,
        attempts: 2,
        max_attempts: 3,
        lease_expires_at: null,
        next_run_after: null,
        created_asserted_at: 11,
        updated_asserted_at: 22,
        dedupe_key: 'key',
        last_error: 'oops',
      },
    ]);

    const result = await listFailedJobs('p-1', 5);

    expect(result[0]).toMatchObject({
      partitionId: 'p-1',
      jobId: 'j-1',
      jobType: 'schema',
      status: 3,
      priority: 1,
      attempts: 2,
      maxAttempts: 3,
      createdAssertedAt: '11',
      updatedAssertedAt: '22',
      dedupeKey: 'key',
      lastError: 'oops',
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_list_failed_jobs', {
      partitionId: 'p-1',
      limit: 5,
    });
  });

  it('maps schema manifest rows', async () => {
    invokeMock.mockResolvedValue({
      manifest_version: 'v2',
      migrations: ['001_init', '002_next'],
      tables: [
        {
          name: 'nodes',
          columns: [
            { name: 'id', logical_type: 'uuid', nullable: false },
            { name: 'name', logical_type: 'text', nullable: true },
          ],
          indexes: [{ name: 'idx_nodes_id', columns: ['id'], unique: true }],
        },
      ],
    });

    const result = await getSchemaManifest();

    expect(result).toEqual({
      manifestVersion: 'v2',
      migrations: ['001_init', '002_next'],
      tables: [
        {
          name: 'nodes',
          columns: [
            { name: 'id', logicalType: 'uuid', nullable: false },
            { name: 'name', logicalType: 'text', nullable: true },
          ],
          indexes: [{ name: 'idx_nodes_id', columns: ['id'], unique: true }],
        },
      ],
    });
  });

  it('maps explain resolution results', async () => {
    invokeMock.mockResolvedValue({
      entity_id: 'n-1',
      field_id: 'f-1',
      resolved: { t: 1, v: { Str: 'ok' } },
      winner: {
        value: { Str: 'ok' },
        valid_from: 1_000_000,
        valid_to: null,
        layer: 2,
        asserted_at: 100,
        op_id: 'op-1',
        is_tombstone: false,
        precedence: {
          layer: 2,
          interval_width: 1,
          asserted_at: 100,
          op_id: 'op-1',
        },
      },
      candidates: [],
    });

    const result = await explainResolution({
      partitionId: 'p-1',
      entityId: 'n-1',
      fieldId: 'f-1',
      at: '2024-01-01T00:00:00.000Z',
    });

    expect(result).toMatchObject({
      entityId: 'n-1',
      fieldId: 'f-1',
      resolved: { t: 'str', v: 'ok' },
      winner: {
        value: { t: 'str', v: 'ok' },
        validFrom: '1970-01-01T00:00:01.000Z',
        layer: 2,
        assertedAt: '100',
        opId: 'op-1',
        isTombstone: false,
      },
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_explain_resolution', {
      partitionId: 'p-1',
      entityId: 'n-1',
      fieldId: 'f-1',
      at: '2024-01-01T00:00:00.000Z',
      asOfAssertedAt: undefined,
      scenarioId: undefined,
    });
  });

  it('maps explain traversal results', async () => {
    invokeMock.mockResolvedValue({
      edge_id: 'e-1',
      active: true,
      winner: {
        edge_id: 'e-1',
        src_id: 'n-1',
        dst_id: 'n-2',
        edge_type_id: 'rel',
        valid_from: 2_000_000,
        valid_to: 3_000_000,
        layer: 1,
        asserted_at: 200,
        op_id: 'op-2',
        is_tombstone: false,
        precedence: {
          layer: 1,
          interval_width: 1,
          asserted_at: 200,
          op_id: 'op-2',
        },
      },
      candidates: [],
    });

    const result = await explainTraversal({
      partitionId: 'p-1',
      edgeId: 'e-1',
      at: '2024-01-02T00:00:00.000Z',
    });

    expect(result).toMatchObject({
      edgeId: 'e-1',
      active: true,
      winner: {
        edgeId: 'e-1',
        srcId: 'n-1',
        dstId: 'n-2',
        edgeTypeId: 'rel',
        validFrom: '1970-01-01T00:00:02.000Z',
        validTo: '1970-01-01T00:00:03.000Z',
        layer: 1,
        assertedAt: '200',
        opId: 'op-2',
        isTombstone: false,
      },
    });
    expect(invokeMock).toHaveBeenCalledWith('mneme_explain_traversal', {
      partitionId: 'p-1',
      edgeId: 'e-1',
      at: '2024-01-02T00:00:00.000Z',
      asOfAssertedAt: undefined,
      scenarioId: undefined,
    });
  });
});
