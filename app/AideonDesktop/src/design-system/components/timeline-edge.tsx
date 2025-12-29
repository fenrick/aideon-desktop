import type { Edge, EdgeProps } from '@xyflow/react';
import { BaseEdge, EdgeLabelRenderer, getBezierPath } from '@xyflow/react';

export interface TimelineEdgeData extends Record<string, unknown> {
  label?: string;
}

export function TimelineEdge(properties: EdgeProps<Edge<TimelineEdgeData, 'timeline'>>) {
  const {
    id,
    sourceX,
    sourceY,
    targetX,
    targetY,
    data,
    markerStart,
    markerEnd,
    style,
    label,
    labelStyle,
    labelShowBg,
    labelBgStyle,
    labelBgPadding,
    labelBgBorderRadius,
    interactionWidth,
  } = properties;
  const [path, labelX, labelY] = getBezierPath({ sourceX, sourceY, targetX, targetY });

  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerStart={markerStart}
        markerEnd={markerEnd}
        style={style}
        label={label}
        labelStyle={labelStyle}
        labelShowBg={labelShowBg}
        labelBgStyle={labelBgStyle}
        labelBgPadding={labelBgPadding}
        labelBgBorderRadius={labelBgBorderRadius}
        interactionWidth={interactionWidth}
      />
      {data?.label ? (
        <EdgeLabelRenderer>
          <div
            data-slot="timeline-edge-label"
            style={{
              position: 'absolute',
              transform: 'translate(-50%, -50%)',
              pointerEvents: 'all',
              left: labelX,
              top: labelY,
            }}
            className="rounded bg-background/80 px-1.5 py-0.5 text-xs text-foreground shadow"
          >
            {data.label}
          </div>
        </EdgeLabelRenderer>
      ) : null}
    </>
  );
}
