import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from './design-system';
import { Badge } from './design-system/components/ui/badge';
import { Skeleton } from './design-system/components/ui/skeleton';

import { useWorkspaceTree, type WorkspaceTreeItem } from './hooks/use-workspace-tree';

/**
 * Sidebar tree showing available workspaces/projects for the desktop shell.
 * Fetches the tree via `useWorkspaceTree` and renders loading/empty states.
 */
export function DesktopTree() {
  const { loading, items, error } = useWorkspaceTree();

  return (
    <SidebarContent className="p-2">
      <SidebarGroup>
        <SidebarGroupLabel>Workspaces</SidebarGroupLabel>
        <SidebarGroupContent>
          <SidebarMenu>
            {loading && (
              <div className="space-y-2 p-1">
                <Skeleton className="h-5 w-32" />
                <Skeleton className="h-4 w-40" />
              </div>
            )}

            {!loading && error && (
              <SidebarMenuItem>
                <SidebarMenuButton
                  disabled
                  className="text-left text-xs text-destructive hover:text-destructive"
                >
                  Failed to load workspaces: {error}
                </SidebarMenuButton>
              </SidebarMenuItem>
            )}

            {!loading && !error && items.map((item) => <TreeNode key={item.id} item={item} />)}
          </SidebarMenu>
          {!loading && !error && items.length === 0 ? (
            <Sidebar className="mt-2 rounded-md border border-dashed border-border/70 bg-muted/30 p-2 text-xs text-muted-foreground">
              No workspaces available yet.
            </Sidebar>
          ) : undefined}
        </SidebarGroupContent>
      </SidebarGroup>
    </SidebarContent>
  );
}

/**
 * Render a single workspace node with its children.
 * @param root0 component properties.
 * @param root0.item workspace or project node to render.
 */
function TreeNode({ item }: { readonly item: WorkspaceTreeItem }) {
  return (
    <SidebarMenuItem>
      <SidebarMenuButton asChild size="sm">
        <div className="flex flex-col gap-1 text-left">
          <span className="font-medium text-sm">{item.label}</span>
          {item.children ? (
            <div className="text-xs text-muted-foreground">
              {item.children.length} workspace
              {item.children.length === 1 ? '' : 's'}
            </div>
          ) : undefined}
        </div>
      </SidebarMenuButton>
      {item.children ? (
        <div className="ml-3 mt-1 space-y-1">
          {item.children.map((child) => (
            <SidebarMenuButton
              key={child.id}
              size="sm"
              className="items-start gap-2 text-muted-foreground"
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="truncate">{child.label}</span>
                  {child.meta?.isDefault ? (
                    <Badge variant="secondary" className="h-5 px-2 text-[10px]">
                      Default
                    </Badge>
                  ) : undefined}
                </div>
                {child.meta?.branch ? (
                  <div className="truncate text-[11px] text-muted-foreground/80">
                    {child.meta.branch}
                  </div>
                ) : undefined}
              </div>
            </SidebarMenuButton>
          ))}
        </div>
      ) : undefined}
    </SidebarMenuItem>
  );
}
