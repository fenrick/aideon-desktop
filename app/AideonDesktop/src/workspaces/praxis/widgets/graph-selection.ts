export interface GraphSelectionSnapshot {
  readonly nodeIds: readonly string[];
  readonly edgeIds: readonly string[];
}

export interface GraphSelectionEvent {
  readonly nodes?: readonly { id: string }[];
  readonly edges?: readonly { id: string }[];
}

/**
 * Normalises a ReactFlow-style selection event into a stable snapshot of ids.
 * @param event - Selection change event payload.
 */
export function selectionFromEvent(event: GraphSelectionEvent): GraphSelectionSnapshot {
  return {
    nodeIds: (event.nodes ?? []).map((node) => node.id),
    edgeIds: (event.edges ?? []).map((edge) => edge.id),
  };
}

/**
 * Returns true when two string arrays contain the same unique members.
 * @param a - First list of ids.
 * @param b - Second list of ids.
 */
export function areStringSetsEqual(a: readonly string[], b: readonly string[]): boolean {
  if (a === b) {
    return true;
  }

  const setA = new Set(a);
  const setB = new Set(b);
  if (setA.size !== setB.size) {
    return false;
  }

  for (const value of setA) {
    if (!setB.has(value)) {
      return false;
    }
  }

  return true;
}
