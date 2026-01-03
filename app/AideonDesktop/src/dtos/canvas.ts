export interface CanvasLayoutGetRequest {
  readonly docId: string;
  readonly asOf: string;
  readonly scenario?: string;
  readonly layer?: string;
}

export interface CanvasNode {
  readonly id: string;
  readonly typeId: string;
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
  readonly z: number;
  readonly label?: string;
  readonly groupId?: string;
}

export interface CanvasEdge {
  readonly id: string;
  readonly source: string;
  readonly target: string;
  readonly label?: string;
  readonly z?: number;
}

export interface CanvasGroup {
  readonly id: string;
  readonly name?: string;
  readonly parentId?: string;
  readonly z?: number;
}

export interface CanvasLayoutSnapshot {
  readonly docId: string;
  readonly asOf: string;
  readonly scenario?: string;
  readonly layer?: string;
  readonly nodes: CanvasNode[];
  readonly edges: CanvasEdge[];
  readonly groups: CanvasGroup[];
}
