import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('workspaces/mneme/platform', () => ({ isTauri: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
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
  ingestOps,
  getPartitionHead,
  readEntityAtTime,
  traverseAtTime,
  tombstoneEntity,
  upsertMetamodelBatch,
} from 'workspaces/mneme/mneme-api';

const isTauriMock = vi.mocked(isTauri);
const invokeMock = vi.mocked(invoke);

describe('mneme-api metamodel bindings', () => {
  beforeEach(() => {
    isTauriMock.mockReturnValue(true);
    invokeMock.mockReset();
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

  it('fetches partition head', async () => {
    invokeMock.mockResolvedValue({ head: '999' });

    const result = await getPartitionHead('p-1');

    expect(result.head).toBe('999');
    expect(invokeMock).toHaveBeenCalledWith('mneme_get_partition_head', { partitionId: 'p-1' });
  });
});
