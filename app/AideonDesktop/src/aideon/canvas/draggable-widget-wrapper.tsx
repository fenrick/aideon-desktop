import type { ReactNode } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { cn } from 'design-system/lib/utilities';

interface DraggableWidgetWrapperProperties {
  readonly id: string;
  readonly x: number;
  readonly y: number;
  readonly width?: number;
  readonly height?: number;
  readonly scale: number;
  readonly children: ReactNode;
  readonly onDragEnd: (id: string, x: number, y: number) => void;
  readonly onResizeEnd?: (id: string, width: number, height: number) => void;
}

/**
 * Positions a widget on the canvas and adds drag/resize behaviour.
 * @param root0
 * @param root0.id
 * @param root0.x
 * @param root0.y
 * @param root0.width
 * @param root0.height
 * @param root0.scale
 * @param root0.children
 * @param root0.onDragEnd
 * @param root0.onResizeEnd
 */
export function DraggableWidgetWrapper({
  id,
  x,
  y,
  width = 580,
  height = 500,
  scale,
  children,
  onDragEnd,
  onResizeEnd,
}: DraggableWidgetWrapperProperties) {
  const [isDragging, setIsDragging] = useState(false);
  const [isResizing, setIsResizing] = useState(false);

  const [localPos, setLocalPos] = useState({ x, y });
  const [localSize, setLocalSize] = useState({ w: width, h: height });

  const lastMouseReference = useRef({ x: 0, y: 0 });

  useEffect(() => {
    if (isDragging) {
      return;
    }

    const frame = requestAnimationFrame(() => {
      setLocalPos({ x, y });
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [isDragging, x, y]);

  useEffect(() => {
    if (isResizing) {
      return;
    }

    const frame = requestAnimationFrame(() => {
      setLocalSize({ w: width, h: height });
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [height, isResizing, width]);

  const handlePointerDown = useCallback((event: React.PointerEvent) => {
    const target = event.target as HTMLElement;

    // Check for resize handle first
    if (target.closest('.resize-handle')) {
      event.preventDefault();
      event.stopPropagation();
      setIsResizing(true);
      lastMouseReference.current = { x: event.clientX, y: event.clientY };
      target.setPointerCapture(event.pointerId);
      return;
    }

    // Drag handle check
    if (target.closest('.cursor-grab')) {
      event.preventDefault();
      event.stopPropagation();
      setIsDragging(true);
      lastMouseReference.current = { x: event.clientX, y: event.clientY };
      target.setPointerCapture(event.pointerId);
    }
  }, []);

  const handlePointerMove = useCallback(
    (event: React.PointerEvent) => {
      if (!isDragging && !isResizing) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();

      const deltaX = (event.clientX - lastMouseReference.current.x) / scale;
      const deltaY = (event.clientY - lastMouseReference.current.y) / scale;
      lastMouseReference.current = { x: event.clientX, y: event.clientY };

      if (isDragging) {
        setLocalPos((previous) => ({ x: previous.x + deltaX, y: previous.y + deltaY }));
        return;
      }

      setLocalSize((previous) => ({
        w: Math.max(300, previous.w + deltaX),
        h: Math.max(300, previous.h + deltaY),
      }));
    },
    [isDragging, isResizing, scale],
  );

  const handlePointerUp = useCallback(
    (event: React.PointerEvent) => {
      if (!isDragging && !isResizing) {
        return;
      }

      (event.target as HTMLElement).releasePointerCapture(event.pointerId);

      if (isDragging) {
        setIsDragging(false);
        onDragEnd(id, localPos.x, localPos.y);
        return;
      }

      setIsResizing(false);
      onResizeEnd?.(id, localSize.w, localSize.h);
    },
    [
      id,
      isDragging,
      isResizing,
      localPos.x,
      localPos.y,
      localSize.h,
      localSize.w,
      onDragEnd,
      onResizeEnd,
    ],
  );

  const transform = useMemo(() => {
    const xValue = localPos.x.toString();
    const yValue = localPos.y.toString();
    return `translate(${xValue}px, ${yValue}px)`;
  }, [localPos.x, localPos.y]);

  return (
    <div
      className={cn(
        'group absolute flex flex-col rounded-xl bg-card shadow-sm transition-shadow',
        isDragging ? 'z-50 cursor-grabbing shadow-xl' : 'z-10 hover:shadow-md',
        isResizing ? 'z-50 cursor-se-resize' : undefined,
      )}
      style={{
        transform,
        width: localSize.w,
        height: localSize.h,
      }}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
    >
      {children}

      {/* Resize Handle */}
      <div className="resize-handle absolute bottom-0 right-0 h-6 w-6 cursor-se-resize p-1 opacity-0 transition-opacity hover:opacity-100 group-hover:opacity-100">
        <div className="h-full w-full rounded-br-lg border-b-2 border-r-2 border-muted-foreground/40" />
      </div>
    </div>
  );
}
