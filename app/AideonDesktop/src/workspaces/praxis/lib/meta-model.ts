import type { MetaModelDocument } from '../../../dtos';

import { getMetaModelDocument } from 'praxis/praxis-api';
import { isTauri } from 'praxis/platform';

/**
 * Fetch the current meta-model document.
 *
 * - In Tauri: delegates to the host contract (`praxis.metamodel.get`).
 * - Outside Tauri: returns a deterministic sample schema for previews/tests.
 */
export async function fetchMetaModel(): Promise<MetaModelDocument> {
  if (isTauri()) {
    return getMetaModelDocument();
  }
  return SAMPLE_SCHEMA;
}

const SAMPLE_SCHEMA: MetaModelDocument = {
  version: '2025.4',
  description: 'Baseline schema for the Chrona digital twin.',
  types: [
    {
      id: 'Capability',
      label: 'Business Capability',
      category: 'Business',
      attributes: [
        { name: 'criticality', type: 'enum', enum: ['low', 'medium', 'high'] },
        { name: 'owner', type: 'string', required: true },
      ],
    },
    {
      id: 'Application',
      label: 'Application Service',
      category: 'Technology',
      extends: 'Service',
      attributes: [
        { name: 'lifecycle', type: 'enum', enum: ['plan', 'live', 'retire'] },
        { name: 'platform', type: 'string' },
      ],
    },
  ],
  relationships: [
    {
      id: 'supports',
      label: 'Supports',
      directed: true,
      from: ['Application'],
      to: ['Capability'],
      attributes: [{ name: 'confidence', type: 'number' }],
    },
    {
      id: 'depends_on',
      directed: true,
      from: ['Capability'],
      to: ['Capability'],
    },
  ],
};
