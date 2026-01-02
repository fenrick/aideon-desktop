export { ensureIsoDateTime } from './contracts';

export type {
  ConfidencePercent,
  GraphSnapshotMetrics,
  MetaAttributeKind,
  MetaModelAttribute,
  MetaModelDocument,
  MetaModelRelationship,
  MetaModelType,
  MetaValidationRules,
  PlanEvent,
  PlanEventEffect,
  PlanEventSource,
  ScenarioKey,
  TemporalDiffMetrics,
  TemporalDiffParameters,
  TemporalDiffSnapshot,
  TemporalStateParameters,
  TemporalStateSnapshot,
  TemporalTopologyDeltaMetrics,
  TemporalTopologyDeltaParameters,
  TemporalTopologyDeltaSnapshot,
  WorkerHealth,
  WorkerJobMap,
  WorkerJobRequest,
  WorkerJobResult,
} from './contracts';

import type {
  MetaModelDocument,
  TemporalDiffParameters,
  TemporalDiffSnapshot,
  TemporalStateParameters,
  TemporalStateSnapshot,
  TemporalTopologyDeltaParameters,
  TemporalTopologyDeltaSnapshot,
  WorkerJobRequest,
  WorkerJobResult,
} from './contracts';

export { IpcTemporalAdapter } from './timegraph-ipc';

/**
 * GraphAdapter defines the read-only time-sliced graph access used by the UI.
 * Implementations must be backend-agnostic and respect AGENTS.md boundaries.
 */
export interface GraphAdapter {
  /**
   * Fetches the immutable graph snapshot for the provided timestamp.
   * Implementations must honour the "time-first" semantics by avoiding
   * in-place mutation of the returned object.
   */
  stateAt(parameters: TemporalStateParameters): Promise<TemporalStateSnapshot>;

  /**
   * Computes change statistics between two snapshots. Backends may choose the
   * most efficient path (e.g., diffing plateau IDs or re-running analytics).
   */
  diff(parameters: TemporalDiffParameters): Promise<TemporalDiffSnapshot>;
}

export interface MetaModelProvider {
  getMetaModel(): Promise<MetaModelDocument>;
}

/**
 * StorageAdapter abstracts snapshot persistence without leaking backend specifics.
 * References are opaque handles chosen by the host (e.g., plateau IDs or URIs).
 */
export interface StorageAdapter {
  getSnapshot(reference: string): Promise<ArrayBuffer>;
  putSnapshot(reference: string, bytesData: ArrayBuffer): Promise<void>;
}

/**
 * WorkerClient runs analytics jobs via the Rust engine adapters (local or remote).
 * The discriminated union ensures only supported jobs can be invoked.
 */
export interface WorkerClient {
  runJob<TJob extends WorkerJobRequest>(job: TJob): Promise<WorkerJobResult<TJob>>;
}

/** Mutable graph adapter extends read-only with commit/branch operations for dev flows. */
export interface MutableGraphAdapter extends GraphAdapter {
  commit(parameters: {
    branch: string;
    parent?: string;
    author?: string;
    message: string;
    tags?: string[];
    time?: string;
    changes: {
      nodeCreates?: string[];
      nodeDeletes?: string[];
      edgeCreates?: { from: string; to: string }[];
      edgeDeletes?: { from: string; to: string }[];
    };
  }): Promise<{ id: string }>;
  listCommits(parameters: { branch: string }): Promise<
    {
      id: string;
      branch: string;
      parents: string[];
      author?: string;
      time?: string;
      message: string;
      tags: string[];
      changeCount: number;
    }[]
  >;
  createBranch(parameters: {
    name: string;
    from?: string;
  }): Promise<{ name: string; head?: string }>;
  listBranches(): Promise<{ name: string; head?: string }[]>;
  mergeBranches(parameters: { source: string; target: string; strategy?: string }): Promise<{
    result?: string;
    conflicts?: { reference: string; kind: string; message: string }[];
  }>;
  topologyDelta(
    parameters: TemporalTopologyDeltaParameters,
  ): Promise<TemporalTopologyDeltaSnapshot>;
}
