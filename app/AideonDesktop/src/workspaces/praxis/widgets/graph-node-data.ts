export interface GraphNodeData extends Record<string, unknown> {
  readonly label: string;
  readonly typeLabel?: string;
  readonly status?: 'active' | 'warning' | 'error';
  readonly meta?: string;
  readonly entityTypes?: readonly string[];
  readonly onInspect?: () => void;
}
