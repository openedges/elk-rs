export interface ElkPoint {
  x: number;
  y: number;
}

export interface LayoutOptions {
  [key: string]: string;
}

export interface ElkGraphElement {
  id: string;
  labels?: ElkLabel[];
  layoutOptions?: LayoutOptions;
}

export interface ElkShape extends ElkGraphElement {
  x?: number;
  y?: number;
  width?: number;
  height?: number;
}

export interface ElkNode extends ElkShape {
  children?: ElkNode[];
  ports?: ElkPort[];
  edges?: ElkEdge[];
  properties?: LayoutOptions;
}

export type ElkPort = ElkShape;

export interface ElkLabel extends ElkShape {
  text?: string;
}

export interface ElkEdge extends ElkGraphElement {
  sources: string[];
  targets: string[];
  sections?: ElkEdgeSection[];
  junctionPoints?: ElkPoint[];
  container?: string;
}

export interface ElkEdgeSection extends ElkGraphElement {
  startPoint: ElkPoint;
  endPoint: ElkPoint;
  bendPoints?: ElkPoint[];
  incomingShape?: string;
  outgoingShape?: string;
}
