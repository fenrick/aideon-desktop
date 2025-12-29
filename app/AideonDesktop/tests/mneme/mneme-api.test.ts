import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('workspaces/mneme/platform', () => ({ isTauri: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
import { isTauri } from 'workspaces/mneme/platform';

import {
  compileEffectiveSchema,
  getEffectiveSchema,
  listEdgeTypeRules,
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
});
