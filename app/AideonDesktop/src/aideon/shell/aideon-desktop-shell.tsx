import type { ReactNode } from 'react';
import { useCallback, useEffect, useRef, useState } from 'react';

import { ScrollArea } from 'design-system/components/ui/scroll-area';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
  SidebarInset,
  SidebarProvider,
} from 'design-system/desktop-shell';
import { cn } from 'design-system/lib/utilities';
import { AideonShellControlsProvider } from './shell-controls';

import type { ImperativePanelGroupHandle, ImperativePanelHandle } from 'react-resizable-panels';

interface AideonDesktopShellProperties {
  readonly navigation: ReactNode;
  readonly content: ReactNode;
  readonly inspector: ReactNode;
  readonly toolbar?: ReactNode;
  readonly className?: string;
}

/**
 * Standard three-pane Aideon desktop layout.
 * Left: navigation. Centre: workspace surface. Right: inspector.
 * @param root0
 * @param root0.navigation
 * @param root0.content
 * @param root0.inspector
 * @param root0.toolbar
 * @param root0.className
 */
const DEFAULT_LAYOUT = { content: 70, inspector: 30 };

/**
 *
 * @param contentSize
 * @param inspectorSize
 */
function normalizeLayout(contentSize: number, inspectorSize: number) {
  const total = contentSize + inspectorSize;
  if (!Number.isFinite(total) || total <= 0) {
    return DEFAULT_LAYOUT;
  }
  if (Math.abs(total - 100) < 0.1) {
    return { content: contentSize, inspector: inspectorSize };
  }
  const normalizedContent = (contentSize / total) * 100;
  return {
    content: Number(normalizedContent.toFixed(2)),
    inspector: Number((100 - normalizedContent).toFixed(2)),
  };
}

/**
 *
 * @param storage
 */
function readStoredLayout(storage: Storage) {
  try {
    const raw = storage.getItem('aideon-shell-panels');
    if (!raw) {
      return;
    }
    const parsed = JSON.parse(raw) as unknown;
    if (Array.isArray(parsed) && parsed.every((value) => typeof value === 'number')) {
      if (parsed.length >= 3) {
        return normalizeLayout(parsed[1], parsed[2]);
      }
      return normalizeLayout(parsed[0], parsed[1] ?? DEFAULT_LAYOUT.inspector);
    }
  } catch {
    return;
  }
}

/**
 *
 * @param storage
 */
function readInspectorCollapsed(storage: Storage) {
  try {
    return storage.getItem('aideon-shell-inspector-collapsed') === '1';
  } catch {
    return false;
  }
}

/**
 *
 * @param root0
 * @param root0.navigation
 * @param root0.content
 * @param root0.inspector
 * @param root0.toolbar
 * @param root0.className
 */
export function AideonDesktopShell({
  navigation,
  content,
  inspector,
  toolbar,
  className,
}: AideonDesktopShellProperties) {
  const inspectorPanelReference = useRef<ImperativePanelHandle>(null);
  const panelGroupReference = useRef<ImperativePanelGroupHandle>(null);
  const layoutInitialized = useRef(false);

  const [inspectorCollapsed, setInspectorCollapsed] = useState(() => {
    try {
      const storage = (globalThis as unknown as { localStorage?: Storage }).localStorage;
      if (storage && typeof storage.getItem === 'function') {
        return readInspectorCollapsed(storage);
      }
    } catch {
      /* ignore */
    }
    return false;
  });

  const persistInspectorCollapsed = useCallback((next: boolean) => {
    try {
      const storage = (globalThis as unknown as { localStorage?: Storage }).localStorage;
      if (storage && typeof storage.setItem === 'function') {
        storage.setItem('aideon-shell-inspector-collapsed', next ? '1' : '0');
      }
    } catch {
      /* ignore */
    }
  }, []);

  const toggleInspector = useCallback(() => {
    const handle = inspectorPanelReference.current;
    if (!handle) {
      setInspectorCollapsed((previous) => {
        const next = !previous;
        persistInspectorCollapsed(next);
        return next;
      });
      return;
    }
    if (inspectorCollapsed) {
      handle.expand();
    } else {
      handle.collapse();
    }
  }, [inspectorCollapsed, persistInspectorCollapsed]);

  useEffect(() => {
    if (!inspectorCollapsed) {
      return;
    }
    queueMicrotask(() => {
      inspectorPanelReference.current?.collapse();
    });
  }, [inspectorCollapsed]);

  useEffect(() => {
    const storage = (globalThis as unknown as { localStorage?: Storage }).localStorage;
    if (!storage || typeof storage.getItem !== 'function') {
      layoutInitialized.current = true;
      return;
    }

    const storedLayout = readStoredLayout(storage);
    if (storedLayout) {
      queueMicrotask(() => {
        panelGroupReference.current?.setLayout([storedLayout.content, storedLayout.inspector]);
      });
    }

    layoutInitialized.current = true;
  }, []);

  return (
    <SidebarProvider>
      <AideonShellControlsProvider value={{ inspectorCollapsed, toggleInspector }}>
        <div className={cn('flex min-h-screen bg-background text-foreground', className)}>
          {navigation}
          <SidebarInset>
            {toolbar ? (
              <header
                data-tauri-drag-region
                className="bg-background/95 sticky top-0 z-20 flex shrink-0 items-center gap-2 border-b border-border px-4 py-3 backdrop-blur"
              >
                {toolbar}
              </header>
            ) : undefined}
            <ResizablePanelGroup
              ref={panelGroupReference}
              direction="horizontal"
              className="min-h-0 flex-1"
              onLayout={(sizes) => {
                if (!layoutInitialized.current) {
                  return;
                }
                try {
                  const storage = (globalThis as unknown as { localStorage?: Storage })
                    .localStorage;
                  if (storage && typeof storage.setItem === 'function') {
                    storage.setItem('aideon-shell-panels', JSON.stringify(sizes));
                  }
                } catch {
                  /* ignore */
                }
              }}
            >
              <ResizablePanel
                defaultSize={DEFAULT_LAYOUT.content}
                minSize={40}
                className="min-w-[360px]"
                data-testid="aideon-shell-panel-content"
              >
                <ScrollArea className="h-full" data-testid="aideon-shell-content">
                  <div className="min-h-full px-6 pb-10 pt-6">{content}</div>
                </ScrollArea>
              </ResizablePanel>

              <ResizableHandle withHandle />

              <ResizablePanel
                ref={inspectorPanelReference}
                defaultSize={DEFAULT_LAYOUT.inspector}
                minSize={16}
                collapsible
                collapsedSize={0}
                onCollapse={() => {
                  setInspectorCollapsed(true);
                  persistInspectorCollapsed(true);
                }}
                onExpand={() => {
                  setInspectorCollapsed(false);
                  persistInspectorCollapsed(false);
                }}
                className="min-w-[260px] max-w-[520px]"
                data-testid="aideon-shell-panel-inspector"
              >
                <ScrollArea
                  className="h-full border-l border-border bg-sidebar text-sidebar-foreground shadow-none"
                  data-testid="aideon-shell-inspector"
                >
                  <div
                    className={cn(
                      'p-4',
                      inspectorCollapsed ? 'pointer-events-none opacity-0' : undefined,
                    )}
                  >
                    {inspector}
                  </div>
                </ScrollArea>
              </ResizablePanel>
            </ResizablePanelGroup>
          </SidebarInset>
        </div>
      </AideonShellControlsProvider>
    </SidebarProvider>
  );
}
