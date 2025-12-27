import type { ComponentPropsWithoutRef } from 'react';

import { cn } from 'design-system/lib/utilities';

import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from './resizable';
import { Sidebar, SidebarProvider, useSidebar } from './sidebar';
import type { DesktopShellSlots } from './types';

export type DesktopShellProperties = ComponentPropsWithoutRef<'div'> & DesktopShellSlots;

/**
 * Compose the desktop shell with sidebar, main workspace, and properties panels.
 * @param properties - Slots and layout props for the shell.
 */
export function DesktopShell(properties: DesktopShellProperties) {
  return (
    <SidebarProvider>
      <DesktopShellLayout {...properties} />
    </SidebarProvider>
  );
}

/**
 * Internal layout wrapper for the desktop shell.
 * @param root0 - Layout properties.
 * @param root0.tree - Sidebar tree contents.
 * @param root0.toolbar - Top toolbar contents.
 * @param root0.main - Main workspace contents.
 * @param root0.properties - Properties panel contents.
 * @param root0.className - Optional wrapper class.
 */
function DesktopShellLayout({
  tree,
  toolbar,
  main,
  properties,
  className,
  ...rest
}: DesktopShellProperties) {
  const { setOpen } = useSidebar();

  return (
    <div
      className={cn('flex min-h-screen flex-col bg-background text-foreground', className)}
      {...rest}
    >
      <div className="border-b border-border/60 bg-background/95 px-4 py-2">{toolbar}</div>

      <ResizablePanelGroup direction="horizontal" className="min-h-0 flex-1">
        <ResizablePanel
          defaultSize={20}
          collapsedSize={4}
          maxSize={25}
          onCollapse={() => {
            setOpen(false);
          }}
          onExpand={() => {
            setOpen(true);
          }}
          className="border-r border-border/60"
        >
          <Sidebar collapsible="icon" className="h-full">
            {tree}
          </Sidebar>
        </ResizablePanel>

        <ResizableHandle withHandle />

        <ResizablePanel defaultSize={60} minSize={40} className="min-w-[320px]">
          <div className="flex h-full flex-col overflow-hidden bg-background p-4">{main}</div>
        </ResizablePanel>

        <ResizableHandle withHandle />

        <ResizablePanel defaultSize={20} minSize={15} className="min-w-[240px] max-w-[520px]">
          <div className="h-full overflow-hidden border-l border-border/60 bg-card">
            {properties}
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
