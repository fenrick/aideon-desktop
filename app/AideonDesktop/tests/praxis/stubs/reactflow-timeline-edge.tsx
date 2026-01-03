import type { ComponentPropsWithoutRef } from 'react';

type TimelineEdgeProperties = ComponentPropsWithoutRef<'g'> & Record<string, unknown>;

export const TimelineEdge = (properties: TimelineEdgeProperties) => (
  <g data-testid="timeline-edge" {...properties} />
);
