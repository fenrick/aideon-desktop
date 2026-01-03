import type {
  MetaModelDocument,
  TemporalDiffParameters,
  TemporalDiffSnapshot,
  TemporalStateParameters,
  TemporalStateSnapshot,
  WorkerHealth,
} from '../../dtos';

import { invokeIpc } from '../../adapters/ipc';

import { isTauri } from './platform';
import { toErrorMessage } from './lib/errors';

const COMMANDS = {
  workerHealth: 'system.worker.health',
  graphView: 'praxis.artefact.execute_graph',
  catalogueView: 'praxis.artefact.execute_catalogue',
  matrixView: 'praxis.artefact.execute_matrix',
  chartView: 'praxis.artefact.execute_chart',
  metaModel: 'praxis.metamodel.get',
  listBranches: 'chrona.temporal.list_branches',
  listCommits: 'chrona.temporal.list_commits',
  stateAt: 'chrona.temporal.state_at',
  diffSummary: 'chrona.temporal.diff',
  mergeBranches: 'chrona.temporal.merge_branches',
  applyOperations: 'praxis.task.apply_operations',
  listScenarios: 'praxis.scenario.list',
} as const;

export interface TwinNode {
  id: string;
  type?: string;
  props?: Record<string, unknown>;
}

export interface TwinEdge {
  id?: string;
  from: string;
  to: string;
  type?: string;
  directed?: boolean;
  props?: Record<string, unknown>;
}

export interface ViewFilters {
  nodeTypes?: string[];
  edgeTypes?: string[];
  tags?: string[];
  search?: string;
}

interface ViewDefinitionBase {
  id: string;
  name: string;
  kind: string;
  asOf: string;
  scenario?: string;
  confidence?: number;
  filters?: ViewFilters;
}

export interface GraphViewDefinition extends ViewDefinitionBase {
  kind: 'graph';
  layout?: 'force' | 'hierarchy';
  scope?: {
    rootIds?: string[];
  };
}

export interface CatalogueColumn {
  id: string;
  label: string;
  type: 'string' | 'number' | 'boolean';
}

export interface CatalogueViewDefinition extends ViewDefinitionBase {
  kind: 'catalogue';
  columns: CatalogueColumn[];
  limit?: number;
}

export interface MatrixViewDefinition extends ViewDefinitionBase {
  kind: 'matrix';
  rowType: string;
  columnType: string;
  relationship?: string;
}

export type ChartType = 'kpi' | 'line' | 'bar';

export interface ChartViewDefinition extends ViewDefinitionBase {
  kind: 'chart';
  chartType: ChartType;
  measure: string;
  dimension?: string;
}

export interface ViewMetadata {
  id: string;
  name: string;
  asOf: string;
  scenario?: string;
  fetchedAt: string;
  source: 'host' | 'mock';
}

export interface ViewStats {
  nodes: number;
  edges: number;
}

export interface GraphNodeView extends TwinNode {
  label: string;
  position?: { x: number; y: number };
}

export interface GraphEdgeView extends TwinEdge {
  label?: string;
}

export interface GraphViewModel {
  metadata: ViewMetadata;
  stats: ViewStats;
  nodes: GraphNodeView[];
  edges: GraphEdgeView[];
}

/**
 * Fetch the Praxis meta-model document from the host.
 *
 * This is a contract surface (DTO) and must remain stable; mock-only usage should
 * go through `praxis/lib/meta-model` for browser previews.
 */
export async function getMetaModelDocument(): Promise<MetaModelDocument> {
  if (!isTauri()) {
    throw new Error('Meta-model is only available from the host in Tauri runtime.');
  }
  return invokeIpc<MetaModelDocument>(COMMANDS.metaModel, {});
}

export interface CatalogueRow {
  id: string;
  values: Record<string, string | number | boolean | null>;
}

export interface CatalogueViewModel {
  metadata: ViewMetadata;
  columns: CatalogueColumn[];
  rows: CatalogueRow[];
}

export interface MatrixAxis {
  id: string;
  label: string;
}

export interface MatrixCell {
  rowId: string;
  columnId: string;
  state: 'connected' | 'missing';
  strength?: number;
  value?: string;
}

export interface MatrixViewModel {
  metadata: ViewMetadata;
  rows: MatrixAxis[];
  columns: MatrixAxis[];
  cells: MatrixCell[];
}

export interface ChartPoint {
  label: string;
  value: number;
  timestamp?: string;
}

export interface ChartSeries {
  id: string;
  label: string;
  color?: string;
  points: ChartPoint[];
}

export interface ChartKpiSummary {
  value: number;
  units?: string;
  delta?: number;
  trend?: 'up' | 'down';
}

export interface ChartViewModel {
  metadata: ViewMetadata;
  chartType: ChartType;
  series: ChartSeries[];
  kpi?: ChartKpiSummary;
}

export type StateAtRequest = TemporalStateParameters;
export type StateAtSnapshot = TemporalStateSnapshot;

export interface TemporalBranchSummary {
  name: string;
  head?: string;
}

export interface TemporalCommitSummary {
  id: string;
  branch: string;
  parents: string[];
  author?: string;
  time?: string;
  message: string;
  tags: string[];
  changeCount: number;
}

export type TemporalDiffRequest = TemporalDiffParameters;

export interface TemporalMergeConflict {
  reference: string;
  kind: string;
  message: string;
}

export interface TemporalMergeResult {
  result?: string;
  conflicts?: TemporalMergeConflict[];
}

export type PraxisOperation =
  | { kind: 'createNode'; node: TwinNode }
  | { kind: 'updateNode'; node: TwinNode }
  | { kind: 'deleteNode'; nodeId: string }
  | { kind: 'createEdge'; edge: TwinEdge }
  | { kind: 'deleteEdge'; edgeId: string };

export interface OperationBatchResult {
  accepted: boolean;
  message?: string;
  commitId?: string;
}

export interface ScenarioSummary {
  id: string;
  name: string;
  branch: string;
  description?: string;
  updatedAt: string;
  isDefault?: boolean;
}

const MOCK_HEALTH: WorkerHealth = {
  ok: true,
  timestamp_ms: Date.now(),
  status: 'mock',
  notes: 'Using mock health state outside Tauri',
};

interface ListBranchesResponse {
  branches?: TemporalBranchSummaryPayload[];
}

interface TemporalBranchSummaryPayload {
  name?: string;
  head?: string | null;
}

interface ListCommitsResponse {
  commits?: TemporalCommitSummaryPayload[];
}

interface TemporalCommitSummaryPayload {
  id?: string;
  branch?: string;
  parents?: unknown;
  author?: string;
  time?: string;
  message?: string;
  tags?: unknown;
  changeCount?: number;
  change_count?: number;
}

interface DiffSummaryResponse {
  from?: string;
  to?: string;
  nodeAdds?: number;
  nodeMods?: number;
  nodeDels?: number;
  edgeAdds?: number;
  edgeMods?: number;
  edgeDels?: number;
  node_adds?: number;
  node_mods?: number;
  node_dels?: number;
  edge_adds?: number;
  edge_mods?: number;
  edge_dels?: number;
}

interface MergeResponsePayload {
  result?: string;
  conflicts?: TemporalMergeConflictPayload[];
}

interface TemporalMergeConflictPayload {
  reference?: string;
  kind?: string;
  message?: string;
}

/**
 * Return worker health, falling back to a mock payload when outside Tauri.
 */
export async function getWorkerHealth(): Promise<WorkerHealth> {
  if (!isTauri()) {
    return { ...MOCK_HEALTH, timestamp_ms: Date.now() };
  }
  return invokeIpc(COMMANDS.workerHealth, {});
}

/**
 * Fetch a time-sliced graph view from the host or return a mock for tests/dev.
 * @param definition Graph view request parameters.
 */
export async function getGraphView(definition: GraphViewDefinition): Promise<GraphViewModel> {
  return callOrMock(COMMANDS.graphView, definition, () => mockGraphView(definition));
}

/**
 * Fetch catalogue rows/columns for the requested definition or a mock payload.
 * @param definition catalogue view definition (columns, filters, pagination).
 */
export async function getCatalogueView(
  definition: CatalogueViewDefinition,
): Promise<CatalogueViewModel> {
  return callOrMock(COMMANDS.catalogueView, definition, () => mockCatalogueView(definition));
}

/**
 * Fetch a matrix view (row/column axes plus cells), defaulting to mock data.
 * @param definition matrix view definition.
 */
export async function getMatrixView(definition: MatrixViewDefinition): Promise<MatrixViewModel> {
  return callOrMock(COMMANDS.matrixView, definition, () => mockMatrixView(definition));
}

/**
 * Fetch a chart view, returning mock KPI/line/bar data when not in Tauri.
 * @param definition chart view definition.
 */
export async function getChartView(definition: ChartViewDefinition): Promise<ChartViewModel> {
  return callOrMock(COMMANDS.chartView, definition, () => mockChartView(definition));
}

/**
 * List temporal branches (scenarios) from the host; mock when offline.
 */
export async function listTemporalBranches(): Promise<TemporalBranchSummary[]> {
  const response = await callOrMock<ListBranchesResponse | TemporalBranchSummary[]>(
    COMMANDS.listBranches,
    {},
    () => mockBranches(),
  );
  if (Array.isArray(response)) {
    return response;
  }
  const entries = Array.isArray(response.branches) ? response.branches : [];
  return entries.map((entry) => ({
    name: typeof entry.name === 'string' ? entry.name : '',
    head: typeof entry.head === 'string' ? entry.head : undefined,
  }));
}

/**
 * List commits for a branch, normalising host payloads into strict types.
 * @param branch branch name to query.
 */
export async function listTemporalCommits(branch: string): Promise<TemporalCommitSummary[]> {
  const response = await callOrMock<ListCommitsResponse | TemporalCommitSummary[]>(
    COMMANDS.listCommits,
    { branch },
    () => mockCommits(branch),
  );
  if (Array.isArray(response)) {
    return response;
  }
  const commits = Array.isArray(response.commits) ? response.commits : [];
  return commits.map((entry) => normalizeCommit(entry, branch));
}

/**
 * Fetch a state-at snapshot for the given request; ensures optional fields are undefined when absent.
 * @param request timestamp/scenario/confidence payload.
 */
export async function getStateAtSnapshot(request: StateAtRequest): Promise<StateAtSnapshot> {
  const snapshot = await callOrMock<StateAtSnapshot>(
    COMMANDS.stateAt,
    serializeStateAtArguments(request),
    () => mockStateAt(request),
  );
  return {
    ...snapshot,
    scenario: snapshot.scenario ?? undefined,
    confidence: snapshot.confidence ?? undefined,
  };
}

/**
 * Fetch diff summary metrics between two references; uses mock data outside Tauri.
 * @param request diff request containing `from`, `to`, and optional scope.
 */
export async function getTemporalDiff(request: TemporalDiffRequest): Promise<TemporalDiffSnapshot> {
  const summary = await callOrMock<DiffSummaryResponse>(
    COMMANDS.diffSummary,
    serializeDiffArguments(request),
    () => mockDiffSummary(request),
  );
  return {
    from: summary.from ?? request.from,
    to: summary.to ?? request.to,
    metrics: {
      nodeAdds: summary.nodeAdds ?? summary.node_adds ?? 0,
      nodeMods: summary.nodeMods ?? summary.node_mods ?? 0,
      nodeDels: summary.nodeDels ?? summary.node_dels ?? 0,
      edgeAdds: summary.edgeAdds ?? summary.edge_adds ?? 0,
      edgeMods: summary.edgeMods ?? summary.edge_mods ?? 0,
      edgeDels: summary.edgeDels ?? summary.edge_dels ?? 0,
    },
  };
}

/**
 * Merge source into target branch, returning conflicts when the host reports them.
 * @param request merge parameters including strategy passthrough.
 * @param request.source
 * @param request.target
 * @param request.strategy
 */
export async function mergeTemporalBranches(request: {
  source: string;
  target: string;
  strategy?: string;
}): Promise<TemporalMergeResult> {
  const response = await callOrMock<MergeResponsePayload>(
    COMMANDS.mergeBranches,
    request,
    () => mockMerge(request),
  );
  const conflicts = Array.isArray(response.conflicts)
    ? response.conflicts
        .map((conflict) => normalizeConflict(conflict))
        .filter((conflict): conflict is TemporalMergeConflict => conflict !== undefined)
    : undefined;
  return {
    result: response.result ?? (conflicts && conflicts.length > 0 ? 'conflicts' : 'ok'),
    conflicts,
  };
}

/**
 * Apply a batch of graph operations; host handles commit creation.
 * Falls back to a deterministic mock commit in dev.
 * @param operations list of operations to apply.
 */
export async function applyOperations(
  operations: PraxisOperation[],
): Promise<OperationBatchResult> {
  return callOrMock(COMMANDS.applyOperations, { operations }, () =>
    mockApplyOperations(operations),
  );
}

/**
 * List available scenarios; returns mock scenarios when running outside Tauri.
 */
export async function listScenarios(): Promise<ScenarioSummary[]> {
  return callOrMock(COMMANDS.listScenarios, {}, () => mockScenarios());
}

/**
 * Invoke a Tauri command when available; otherwise return a mock fallback.
 * Wraps host errors with a readable message.
 * @param command Tauri command name.
 * @param payload payload for the command.
 * @param fallback function returning mock data when not in Tauri.
 */
async function callOrMock<T>(
  command: string,
  payload: object,
  fallback: () => T | Promise<T>,
): Promise<T> {
  if (!isTauri()) {
    return fallback();
  }
  try {
    const result = await invokeIpc<T>(command, payload as Record<string, unknown>);
    return result;
  } catch (error) {
    const message = toErrorMessage(error);
    throw new Error(`Host command '${command}' failed: ${message}`);
  }
}

/**
 * Normalise a commit payload into the strict `TemporalCommitSummary` shape.
 * @param payload raw host commit payload.
 * @param fallbackBranch branch to use when host omits it.
 */
function normalizeCommit(
  payload: TemporalCommitSummaryPayload,
  fallbackBranch: string,
): TemporalCommitSummary {
  const parents = Array.isArray(payload.parents)
    ? (payload.parents as unknown[]).filter((value): value is string => typeof value === 'string')
    : [];
  const tags = Array.isArray(payload.tags)
    ? (payload.tags as unknown[]).filter((value): value is string => typeof value === 'string')
    : [];
  let changeCount = 0;
  if (typeof payload.changeCount === 'number') {
    changeCount = payload.changeCount;
  } else if (typeof payload.change_count === 'number') {
    changeCount = payload.change_count;
  }
  return {
    id: payload.id ?? 'unknown',
    branch: payload.branch ?? fallbackBranch,
    parents,
    author: payload.author ?? undefined,
    time: payload.time ?? undefined,
    message: payload.message ?? 'Commit',
    tags,
    changeCount,
  };
}

/**
 * Convert a host merge-conflict payload to a strongly typed object.
 * @param payload raw conflict payload from host.
 */
function normalizeConflict(
  payload: TemporalMergeConflictPayload,
): TemporalMergeConflict | undefined {
  if (!payload.reference || typeof payload.reference !== 'string') {
    return undefined;
  }
  return {
    reference: payload.reference,
    kind: typeof payload.kind === 'string' ? payload.kind : 'unknown',
    message:
      typeof payload.message === 'string' ? payload.message : 'Conflict requires manual resolution',
  };
}

const GRAPH_NODE_IDS = {
  onboarding: 'cap-customer-onboarding',
  support: 'cap-customer-support',
  workflow: 'app-workflow',
  identity: 'svc-auth',
} as const;

/**
 * Mock graph view for offline/dev usage.
 * @param definition
 */
function mockGraphView(definition: GraphViewDefinition): GraphViewModel {
  const nodes: GraphNodeView[] = [
    {
      id: GRAPH_NODE_IDS.onboarding,
      label: 'Customer Onboarding',
      type: 'Capability',
      position: { x: 120, y: 200 },
    },
    {
      id: GRAPH_NODE_IDS.support,
      label: 'Customer Support',
      type: 'Capability',
      position: { x: 420, y: 120 },
    },
    {
      id: GRAPH_NODE_IDS.workflow,
      label: 'Workflow Engine',
      type: 'Application',
      position: { x: 420, y: 320 },
    },
    {
      id: GRAPH_NODE_IDS.identity,
      label: 'Identity Service',
      type: 'Service',
      position: { x: 680, y: 220 },
    },
  ];

  const edges: GraphEdgeView[] = [
    {
      id: 'edge-1',
      from: GRAPH_NODE_IDS.onboarding,
      to: GRAPH_NODE_IDS.support,
      type: 'supports',
      label: 'handoff',
    },
    {
      id: 'edge-2',
      from: GRAPH_NODE_IDS.support,
      to: GRAPH_NODE_IDS.workflow,
      type: 'depends_on',
      label: 'tickets',
    },
    {
      id: 'edge-3',
      from: GRAPH_NODE_IDS.workflow,
      to: GRAPH_NODE_IDS.identity,
      type: 'depends_on',
      label: 'auth',
    },
  ];

  return {
    metadata: buildMetadata(definition),
    stats: {
      nodes: nodes.length,
      edges: edges.length,
    },
    nodes,
    edges,
  };
}

/**
 * Mock catalogue view for offline/dev usage.
 * @param definition
 */
function mockCatalogueView(definition: CatalogueViewDefinition): CatalogueViewModel {
  const columns =
    definition.columns.length > 0
      ? definition.columns
      : [
          { id: 'name', label: 'Name', type: 'string' as const },
          { id: 'owner', label: 'Owner', type: 'string' as const },
          { id: 'state', label: 'State', type: 'string' as const },
        ];

  const rows: CatalogueRow[] = [
    {
      id: 'cap-customer-onboarding',
      values: { name: 'Customer Onboarding', owner: 'CX', state: 'Pilot' },
    },
    {
      id: 'cap-customer-support',
      values: { name: 'Customer Support', owner: 'Ops', state: 'Production' },
    },
    {
      id: 'cap-incident-response',
      values: { name: 'Incident Response', owner: 'SRE', state: 'In Flight' },
    },
  ];

  return {
    metadata: buildMetadata(definition),
    columns,
    rows,
  };
}

const MATRIX_AXIS_IDS = {
  onboarding: 'cap-customer-onboarding',
  incident: 'cap-incident-response',
  identity: 'svc-auth',
  search: 'svc-search',
} as const;

/**
 * Mock matrix view for offline/dev usage.
 * @param definition
 */
function mockMatrixView(definition: MatrixViewDefinition): MatrixViewModel {
  const rows: MatrixAxis[] = [
    { id: MATRIX_AXIS_IDS.onboarding, label: 'Customer Onboarding' },
    { id: MATRIX_AXIS_IDS.incident, label: 'Incident Response' },
  ];
  const columns: MatrixAxis[] = [
    { id: MATRIX_AXIS_IDS.identity, label: 'Identity Service' },
    { id: MATRIX_AXIS_IDS.search, label: 'Search Platform' },
  ];
  const cells: MatrixCell[] = [
    {
      rowId: MATRIX_AXIS_IDS.onboarding,
      columnId: MATRIX_AXIS_IDS.identity,
      state: 'connected',
      strength: 0.8,
    },
    { rowId: MATRIX_AXIS_IDS.onboarding, columnId: MATRIX_AXIS_IDS.search, state: 'missing' },
    {
      rowId: MATRIX_AXIS_IDS.incident,
      columnId: MATRIX_AXIS_IDS.identity,
      state: 'connected',
      strength: 0.4,
    },
    { rowId: MATRIX_AXIS_IDS.incident, columnId: MATRIX_AXIS_IDS.search, state: 'missing' },
  ];
  return {
    metadata: buildMetadata(definition),
    rows,
    columns,
    cells,
  };
}

/**
 * Mock chart view for offline/dev usage.
 * @param definition
 */
function mockChartView(definition: ChartViewDefinition): ChartViewModel {
  const metadata = buildMetadata(definition);
  if (definition.chartType === 'kpi') {
    return {
      metadata,
      chartType: 'kpi',
      series: [],
      kpi: {
        value: 128,
        units: 'services',
        delta: 6,
        trend: 'up',
      },
    };
  }
  if (definition.chartType === 'line') {
    const baseTimestamp = Date.now() - 6 * 24 * 60 * 60 * 1000;
    const points: ChartPoint[] = Array.from({ length: 7 }).map((_, index) => {
      const timestamp = new Date(baseTimestamp + index * 24 * 60 * 60 * 1000);
      return {
        label: timestamp.toLocaleDateString('en-US', { weekday: 'short' }),
        value: 80 + index * 5 + (index % 2 === 0 ? 3 : -4),
        timestamp: timestamp.toISOString(),
      };
    });
    return {
      metadata,
      chartType: 'line',
      series: [
        {
          id: 'velocity',
          label: 'Delivery velocity',
          color: '#2563eb',
          points,
        },
      ],
    };
  }
  const categories = ['Security', 'Resilience', 'Efficiency', 'Experience'];
  const series: ChartSeries[] = [
    {
      id: 'current',
      label: 'Current',
      color: '#0f172a',
      points: categories.map((category) => ({ label: category, value: randomScore(category) })),
    },
    {
      id: 'target',
      label: 'Target',
      color: '#10b981',
      points: categories.map((category) => ({ label: category, value: 95 })),
    },
  ];
  return {
    metadata,
    chartType: 'bar',
    series,
  };
}

/** Provide deterministic mock branch list for offline/dev use. */
function mockBranches(): TemporalBranchSummary[] {
  return [
    { name: 'main', head: 'commit-main-004' },
    { name: 'chronaplay', head: 'commit-chronaplay-002' },
  ];
}

/**
 * Mock commit history for a branch with sensible timestamps and messages.
 * @param branch
 */
function mockCommits(branch: string): TemporalCommitSummary[] {
  if (branch === 'chronaplay') {
    return [
      mockCommit('commit-chronaplay-001', 'chronaplay', 'Chrona staging', -3),
      mockCommit('commit-chronaplay-002', 'chronaplay', 'Chrona overlays', -1),
    ];
  }
  return [
    mockCommit('commit-main-001', 'main', 'Initial snapshot', -14),
    mockCommit('commit-main-002', 'main', 'Capability ingest', -7),
    mockCommit('commit-main-003', 'main', 'Application links', -2),
    mockCommit('commit-main-004', 'main', 'Plan Events sync', 0),
  ];
}

/**
 * Fabricate diff summary metrics for mock mode.
 * @param request
 */
function mockDiffSummary(request: TemporalDiffRequest): DiffSummaryResponse {
  return {
    from: request.from,
    to: request.to,
    node_adds: 3,
    node_mods: 2,
    node_dels: 1,
    edge_adds: 4,
    edge_mods: 1,
    edge_dels: 0,
  };
}

/**
 * Simulate merge responses, injecting a conflict for a known branch pair.
 * @param request
 * @param request.source
 * @param request.target
 */
function mockMerge(request: { source: string; target: string }): MergeResponsePayload {
  if (request.source === 'chronaplay' && request.target === 'main') {
    return {
      result: 'conflicts',
      conflicts: [
        {
          reference: 'cap-customer-onboarding',
          kind: 'node',
          message: 'Capability already diverged in main',
        },
      ],
    };
  }
  return { result: 'merged', conflicts: [] };
}

/**
 * Create a mock commit with a timestamp offset for deterministic ordering.
 * @param id
 * @param branch
 * @param message
 * @param daysOffset
 */
function mockCommit(
  id: string,
  branch: string,
  message: string,
  daysOffset: number,
): TemporalCommitSummary {
  const time = new Date(Date.now() + daysOffset * 24 * 60 * 60 * 1000).toISOString();
  return {
    id,
    branch,
    parents: [],
    author: 'Praxis Bot',
    time,
    message,
    tags: [],
    changeCount: 12,
  };
}

/**
 * Build a mock state-at snapshot with seeded node/edge counts.
 * @param request
 */
function mockStateAt(request: StateAtRequest): StateAtSnapshot {
  const asOf = request.asOf;
  const scenario = request.scenario ?? 'main';
  const base = seededMetric(`${asOf}-${scenario}`);
  const nodes = 220 + (base % 60);
  const edges = 430 + (base % 90);
  return {
    asOf,
    scenario,
    confidence: request.confidence ?? undefined,
    nodes,
    edges,
  };
}

/**
 * Stub operation application, issuing a deterministic mock commit id.
 * @param operations
 */
function mockApplyOperations(operations: PraxisOperation[]): OperationBatchResult {
  if (operations.length === 0) {
    return { accepted: false, message: 'No operations provided' };
  }
  return {
    accepted: true,
    commitId: `mock-commit-${nextOperationId()}`,
    message: 'Mock commit created',
  };
}

/** Mock scenario list aligned with the desktop shell expectations. */
function mockScenarios(): ScenarioSummary[] {
  const now = nowIso();
  return [
    {
      id: 'scenario-mainline',
      name: 'Mainline FY25',
      branch: 'main',
      description: 'Authoritative twin data for production decisions.',
      updatedAt: now,
      isDefault: true,
    },
    {
      id: 'scenario-chrona',
      name: 'Chrona Playground',
      branch: 'chronaplay',
      description: 'Prototype scenario for Chrona overlays.',
      updatedAt: now,
    },
  ];
}

/**
 * Build consistent metadata for mock and host view payloads.
 * @param definition
 */
function buildMetadata(definition: ViewDefinitionBase): ViewMetadata {
  return {
    id: definition.id,
    name: definition.name,
    asOf: definition.asOf,
    scenario: definition.scenario,
    fetchedAt: nowIso(),
    source: isTauri() ? 'host' : 'mock',
  };
}

const nextOperationId = (() => {
  let counter = 1;
  return () => {
    counter += 1;
    return counter.toString().padStart(4, '0');
  };
})();

/** Generate a fresh ISO timestamp. Extracted for easier testing. */
function nowIso(): string {
  return new Date().toISOString();
}

/**
 * Deterministic pseudo-random score generator used by mock data builders.
 * @param seed
 */
function randomScore(seed: string): number {
  let hash = 0;
  for (const character of seed) {
    hash = (hash << 5) - hash + (character.codePointAt(0) ?? 0);
    hash = Math.trunc(hash);
  }
  const normalized = Math.abs(hash % 40);
  return 60 + normalized;
}

/**
 * Serialize `stateAt` arguments for host invocation.
 * @param request
 */
function serializeStateAtArguments(request: StateAtRequest): Record<string, unknown> {
  return {
    asOf: { id: request.asOf },
    scenario: request.scenario ?? undefined,
    confidence: request.confidence ?? undefined,
  };
}

/**
 * Serialize `diff` arguments for host invocation.
 * @param request
 */
function serializeDiffArguments(request: TemporalDiffRequest): Record<string, unknown> {
  return {
    from: { id: request.from },
    to: { id: request.to },
    scope: request.scope ?? undefined,
  };
}

/**
 * Deterministic metric helper used by mocks to keep numbers stable.
 * @param key
 */
function seededMetric(key: string): number {
  return randomScore(key) * 10;
}

export { type TemporalDiffMetrics, type TemporalDiffSnapshot, type WorkerHealth } from '../../dtos';
