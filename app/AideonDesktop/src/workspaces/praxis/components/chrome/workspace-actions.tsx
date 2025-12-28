import { useState } from 'react';

import { Button } from 'design-system/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from 'design-system/components/ui/popover';
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from 'design-system/components/ui/sidebar';
import {
  ArrowDown,
  ArrowUp,
  Bell,
  Copy,
  CornerUpRight,
  FileText,
  LineChart,
  Link,
  MoreHorizontal,
  Settings2,
  Trash2,
} from 'lucide-react';

const ACTION_GROUPS = [
  [
    { label: 'Customize workspace', icon: Settings2 },
    { label: 'Export snapshot', icon: FileText },
  ],
  [
    { label: 'Copy link', icon: Link },
    { label: 'Duplicate', icon: Copy },
    { label: 'Move to', icon: CornerUpRight },
    { label: 'Archive', icon: Trash2 },
  ],
  [
    { label: 'View analytics', icon: LineChart },
    { label: 'Notifications', icon: Bell },
  ],
  [
    { label: 'Import', icon: ArrowUp },
    { label: 'Export', icon: ArrowDown },
  ],
] as const;

/**
 * Popover action menu matching the sidebar-10 block pattern.
 */
export function WorkspaceActions() {
  const [open, setOpen] = useState(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="ghost" size="icon" className="h-7 w-7">
          <MoreHorizontal />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-56 overflow-hidden rounded-lg p-0" align="end">
        <Sidebar collapsible="none" className="bg-transparent">
          <SidebarContent>
            {ACTION_GROUPS.map((group, groupIndex) => (
              <SidebarGroup
                key={`group-${groupIndex.toString()}`}
                className="border-b last:border-none"
              >
                <SidebarGroupContent className="gap-0">
                  <SidebarMenu>
                    {group.map((item) => (
                      <SidebarMenuItem key={item.label}>
                        <SidebarMenuButton>
                          <item.icon />
                          <span>{item.label}</span>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    ))}
                  </SidebarMenu>
                </SidebarGroupContent>
              </SidebarGroup>
            ))}
          </SidebarContent>
        </Sidebar>
      </PopoverContent>
    </Popover>
  );
}
