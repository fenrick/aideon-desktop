import type { ComponentPropsWithoutRef } from 'react';

import { cn } from 'design-system/lib/utilities';

import { Separator } from 'design-system/components/ui/separator';

import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from './resizable';
import { SidebarInset, SidebarProvider, SidebarTrigger } from './sidebar';
import type { DesktopShellSlots } from './types';

export type DesktopShellProperties = ComponentPropsWithoutRef<'div'> & DesktopShellSlots;

/**
 * Compose the desktop shell with sidebar, main workspace, and properties panels.
 * @param properties - Slots and layout props for the shell.
 * @param properties.className - Optional wrapper class.
 * @param properties.tree - Sidebar tree contents.
 * @param properties.toolbar - Top toolbar contents.
 * @param properties.main - Main workspace contents.
 * @param properties.properties - Properties panel contents.
 */
export function DesktopShell({
  className,
  tree,
  toolbar,
  main,
  properties,
  ...rest
}: DesktopShellProperties) {
  return (
    <SidebarProvider className={cn('bg-background text-foreground', className)} {...rest}>
      <DesktopShellLayout tree={tree} toolbar={toolbar} main={main} properties={properties} />
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
function DesktopShellLayout({ tree, toolbar, main, properties }: Readonly<DesktopShellSlots>) {
  return (
    <>
      {tree}
      <SidebarInset>
        <header className="bg-background flex h-16 shrink-0 items-center gap-2 border-b border-border transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-12">
          <div className="flex flex-1 items-center gap-2 px-4">
            <SidebarTrigger className="-ml-1" />
            <Separator orientation="vertical" className="mr-2 data-[orientation=vertical]:h-4" />
            {toolbar}
          </div>
        </header>

        <ResizablePanelGroup direction="horizontal" className="min-h-0 flex-1">
          <ResizablePanel defaultSize={70} minSize={50} className="min-w-[320px]">
            <div className="flex h-full flex-col overflow-hidden bg-background p-4">{main}</div>
          </ResizablePanel>

          <ResizableHandle withHandle />

          <ResizablePanel defaultSize={30} minSize={20} className="min-w-[240px] max-w-[520px]">
            <div className="h-full overflow-hidden border-l border-border/60 bg-card">
              {properties}
            </div>
          </ResizablePanel>
        </ResizablePanelGroup>
      </SidebarInset>
    </>
  );
}
