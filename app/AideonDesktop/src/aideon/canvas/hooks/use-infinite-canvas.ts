import { useCallback, useEffect, useRef, useState } from 'react';

interface ViewportState {
  x: number;
  y: number;
  scale: number;
}

interface UseInfiniteCanvasOptions {
  minScale?: number;
  maxScale?: number;
  initialScale?: number;
}

/**
 * Pointer + wheel interaction helper for a pan/zoom "infinite canvas" surface.
 * @param options
 * @param options.minScale
 * @param options.maxScale
 * @param options.initialScale
 */
export function useInfiniteCanvas(options: UseInfiniteCanvasOptions = {}) {
  const { minScale = 0.1, maxScale = 5, initialScale = 1 } = options;
  const [viewport, setViewport] = useState<ViewportState>({ x: 0, y: 0, scale: initialScale });
  const containerReference = useRef<HTMLDivElement>(null);
  const isDraggingReference = useRef(false);
  const lastMouseReference = useRef({ x: 0, y: 0 });

  const handleWheel = useCallback(
    (event: WheelEvent) => {
      // Prevent default browser zooming/scrolling behaviour when hovering the canvas.
      if (event.ctrlKey || event.metaKey) {
        event.preventDefault();
        const zoomSensitivity = 0.001;
        const delta = -event.deltaY * zoomSensitivity;
        const newScale = Math.min(Math.max(viewport.scale + delta, minScale), maxScale);

        // Zoom towards mouse pointer logic could go here; for now keep it stable.
        setViewport((previous) => ({
          ...previous,
          scale: newScale,
        }));
      } else {
        // Pan
        event.preventDefault();
        setViewport((previous) => ({
          ...previous,
          x: previous.x - event.deltaX,
          y: previous.y - event.deltaY,
        }));
      }
    },
    [maxScale, minScale, viewport.scale],
  );

  const handlePointerDown = useCallback((event: React.PointerEvent) => {
    // Middle mouse (button 1) or Shift + Left Click (button 0)
    if (event.button === 1 || (event.button === 0 && event.shiftKey)) {
      isDraggingReference.current = true;
      lastMouseReference.current = { x: event.clientX, y: event.clientY };
      if (containerReference.current) {
        containerReference.current.style.cursor = 'grabbing';
      }
    }
  }, []);

  const handlePointerMove = useCallback((event: React.PointerEvent) => {
    if (isDraggingReference.current) {
      const dx = event.clientX - lastMouseReference.current.x;
      const dy = event.clientY - lastMouseReference.current.y;
      lastMouseReference.current = { x: event.clientX, y: event.clientY };

      setViewport((previous) => ({
        ...previous,
        x: previous.x + dx,
        y: previous.y + dy,
      }));
    }
  }, []);

  const handlePointerUp = useCallback(() => {
    isDraggingReference.current = false;
    if (containerReference.current) {
      containerReference.current.style.cursor = 'default';
    }
  }, []);

  useEffect(() => {
    const container = containerReference.current;
    if (!container) {
      return;
    }

    // Native wheel listener to be able to use { passive: false } for preventDefault
    container.addEventListener('wheel', handleWheel, { passive: false });
    return () => {
      container.removeEventListener('wheel', handleWheel);
    };
  }, [handleWheel]);

  return {
    viewport,
    setViewport,
    containerReference,
    events: {
      onPointerDown: handlePointerDown,
      onPointerMove: handlePointerMove,
      onPointerUp: handlePointerUp,
      onPointerLeave: handlePointerUp,
    },
  };
}
