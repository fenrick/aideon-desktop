import { useMemo, useState } from 'react';

import { Avatar, AvatarFallback } from 'design-system/components/ui/avatar';
import { Badge } from 'design-system/components/ui/badge';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from 'design-system/components/ui/collapsible';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from 'design-system/components/ui/dropdown-menu';
import { Skeleton } from 'design-system/components/ui/skeleton';
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInput,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  useSidebar,
} from 'design-system/desktop-shell';

import { Button } from 'design-system/components/ui/button';
import { Separator } from 'design-system/components/ui/separator';
import {
  ChevronRight,
  File,
  Folder,
  Frame,
  LifeBuoy,
  MoreHorizontal,
  Network,
  Send,
  Settings2,
  Star,
  Users,
} from 'lucide-react';
import type { ProjectSummary } from 'praxis/domain-data';
import type { ScenarioSummary } from 'praxis/praxis-api';

const NAV_SECTIONS = [
  { id: 'overview', label: 'Overview', icon: Frame },
  { id: 'scenarios', label: 'Scenarios', icon: Network },
  { id: 'teams', label: 'Teams', icon: Users },
  { id: 'settings', label: 'Settings', icon: Settings2 },
] as const;

type NavigationSectionId = (typeof NAV_SECTIONS)[number]['id'];

type TreeItem = string | TreeItem[];

/**
 * Render menu items for a single project and its scenarios.
 * @param parameters
 * @param parameters.project
 * @param parameters.activeScenarioId
 * @param parameters.onSelectScenario
 * @param parameters.onRevealSidebar
 */
function renderProjectScenarioMenuItems(parameters: {
  project: ProjectSummary;
  activeScenarioId?: string;
  onSelectScenario?: (scenarioId: string) => void;
  onRevealSidebar?: () => void;
}) {
  const { project, activeScenarioId, onSelectScenario, onRevealSidebar } = parameters;
  const headerId = `project-${project.id}`;

  if (project.scenarios.length === 0) {
    return [
      <SidebarMenuItem key={headerId}>
        <SidebarMenuButton disabled className="text-left text-xs font-semibold">
          {project.name}
        </SidebarMenuButton>
      </SidebarMenuItem>,
      <SidebarMenuItem key={`${project.id}-empty`}>
        <SidebarMenuButton disabled className="text-left text-xs text-muted-foreground">
          No scenarios yet.
        </SidebarMenuButton>
      </SidebarMenuItem>,
    ];
  }

  return [
    <SidebarMenuItem key={headerId}>
      <SidebarMenuButton disabled className="text-left text-xs font-semibold">
        {project.name}
      </SidebarMenuButton>
    </SidebarMenuItem>,
    ...project.scenarios.map((scenario) => {
      const active = scenario.id === activeScenarioId;
      return (
        <SidebarMenuItem key={scenario.id}>
          <SidebarMenuButton
            size="sm"
            className="flex flex-col items-start gap-1 text-left data-[state=active]:bg-sidebar-accent data-[state=active]:text-sidebar-accent-foreground"
            onClick={() => {
              onSelectScenario?.(scenario.id);
              onRevealSidebar?.();
            }}
            data-state={active ? 'active' : undefined}
          >
            <div className="flex w-full items-center justify-between gap-2">
              <span className="text-sm font-medium">{scenario.name}</span>
              {scenario.isDefault ? <Badge variant="outline">Default</Badge> : undefined}
            </div>
            <p className="text-xs text-muted-foreground">
              Timeline {scenario.branch} · Updated {formatDate(scenario.updatedAt)}
            </p>
          </SidebarMenuButton>
        </SidebarMenuItem>
      );
    }),
  ];
}

/**
 * Render the scenario list area within the sidebar menu.
 * @param parameters
 * @param parameters.loading
 * @param parameters.errorMessage
 * @param parameters.projectList
 * @param parameters.filteredProjects
 * @param parameters.query
 * @param parameters.activeScenarioId
 * @param parameters.onSelectScenario
 * @param parameters.onRetry
 * @param parameters.onRevealSidebar
 */
function renderProjectsSidebarMenu(parameters: {
  loading: boolean;
  errorMessage?: string;
  projectList: ProjectSummary[];
  filteredProjects: ProjectSummary[];
  query: string;
  activeScenarioId?: string;
  onSelectScenario?: (scenarioId: string) => void;
  onRetry?: () => void;
  onRevealSidebar?: () => void;
}) {
  const {
    loading,
    errorMessage,
    projectList,
    filteredProjects,
    query,
    activeScenarioId,
    onSelectScenario,
    onRetry,
    onRevealSidebar,
  } = parameters;

  if (loading) {
    return (
      <div className="space-y-2 p-1">
        <Skeleton className="h-5 w-32" />
        <Skeleton className="h-4 w-24" />
        <Skeleton className="h-4 w-28" />
      </div>
    );
  }

  if (errorMessage) {
    return (
      <SidebarMenuItem>
        <SidebarMenuButton
          disabled
          className="text-left text-xs text-destructive hover:text-destructive"
        >
          Failed to load scenarios: {errorMessage}
        </SidebarMenuButton>
        {onRetry ? (
          <Button
            variant="link"
            className="px-0 text-xs"
            onClick={() => {
              onRetry();
            }}
          >
            Retry
          </Button>
        ) : undefined}
      </SidebarMenuItem>
    );
  }

  if (projectList.length === 0) {
    return (
      <SidebarMenuItem>
        <SidebarMenuButton disabled className="text-left text-sm text-muted-foreground">
          No projects yet.
        </SidebarMenuButton>
      </SidebarMenuItem>
    );
  }

  if (query.trim() && filteredProjects.length === 0) {
    return (
      <SidebarMenuItem>
        <SidebarMenuButton disabled className="text-left text-sm text-muted-foreground">
          No scenarios match “{query.trim()}”.
        </SidebarMenuButton>
      </SidebarMenuItem>
    );
  }

  return (
    <>
      {filteredProjects.flatMap((project) =>
        renderProjectScenarioMenuItems({
          project,
          activeScenarioId,
          onSelectScenario,
          onRevealSidebar,
        }),
      )}
    </>
  );
}

/**
 * Team/workspace switcher aligned to shadcn sidebar blocks.
 */
/**
 * Favorites group for quick links.
 * @param root0
 * @param root0.items
 */
function FavoritesList({ items }: { readonly items: readonly { name: string; emoji: string }[] }) {
  return (
    <SidebarGroup className="group-data-[collapsible=icon]:hidden">
      <SidebarGroupLabel>Favorites</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          {items.map((item) => (
            <SidebarMenuItem key={item.name}>
              <SidebarMenuButton>
                <span>{item.emoji}</span>
                <span>{item.name}</span>
              </SidebarMenuButton>
              <SidebarMenuAction showOnHover>
                <MoreHorizontal />
                <span className="sr-only">More</span>
              </SidebarMenuAction>
            </SidebarMenuItem>
          ))}
          <SidebarMenuItem>
            <SidebarMenuButton className="text-sidebar-foreground/70">
              <Star />
              <span>More</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}

/**
 * Footer actions aligned to sidebar-08 (settings, feedback, account).
 */
function SidebarFooterMenu() {
  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <SidebarMenuButton size="sm">
          <LifeBuoy />
          <span>Feedback</span>
        </SidebarMenuButton>
      </SidebarMenuItem>
      <SidebarMenuItem>
        <SidebarMenuButton size="sm">
          <Settings2 />
          <span>Settings</span>
        </SidebarMenuButton>
      </SidebarMenuItem>
      <SidebarMenuItem>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton
              size="lg"
              className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
            >
              <Avatar className="h-7 w-7 rounded-lg">
                <AvatarFallback className="rounded-lg">AX</AvatarFallback>
              </Avatar>
              <div className="grid flex-1 text-left text-sm leading-tight">
                <span className="truncate font-medium">Alex Rivera</span>
                <span className="truncate text-xs">alex@aideon.ai</span>
              </div>
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className="min-w-56 rounded-lg"
            side="right"
            align="end"
            sideOffset={4}
          >
            <DropdownMenuLabel className="p-0 font-normal">
              <div className="flex items-center gap-2 px-1 py-1.5 text-left text-sm">
                <Avatar className="h-8 w-8 rounded-lg">
                  <AvatarFallback className="rounded-lg">AX</AvatarFallback>
                </Avatar>
                <div className="grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-medium">Alex Rivera</span>
                  <span className="truncate text-xs">alex@aideon.ai</span>
                </div>
              </div>
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem>
              <Send />
              <span>Account</span>
            </DropdownMenuItem>
            <DropdownMenuItem>
              <LifeBuoy />
              <span>Support</span>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem>
              <Settings2 />
              <span>Switch workspace</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}

interface ProjectsSidebarProperties {
  readonly projects?: ProjectSummary[];
  readonly scenarios: ScenarioSummary[];
  readonly loading: boolean;
  readonly error?: string;
  readonly activeScenarioId?: string;
  readonly onSelectScenario?: (scenarioId: string) => void;
  readonly onRetry?: () => void;
}

/**
 * Project/scenario navigation for the left sidebar.
 * Uses shadcn Sidebar primitives to align with the suite shell.
 * @param root0
 * @param root0.projects
 * @param root0.scenarios
 * @param root0.loading
 * @param root0.error
 * @param root0.activeScenarioId
 * @param root0.onSelectScenario
 * @param root0.onRetry
 */
export function ProjectsSidebar({
  projects,
  scenarios,
  loading,
  error: errorMessage,
  activeScenarioId,
  onSelectScenario,
  onRetry,
}: ProjectsSidebarProperties) {
  const { setOpen } = useSidebar();
  const projectList = useMemo(() => {
    return projects?.length ? projects : [{ id: 'default', name: 'Projects', scenarios }];
  }, [projects, scenarios]);

  const [query, setQuery] = useState('');
  const filteredProjects = useMemo(() => {
    const trimmed = query.trim().toLowerCase();
    if (!trimmed) {
      return projectList;
    }
    return projectList
      .map((project) => {
        const nextScenarios = project.scenarios.filter((scenario) => {
          const haystack = `${scenario.name} ${scenario.branch}`.toLowerCase();
          return haystack.includes(trimmed);
        });
        return { ...project, scenarios: nextScenarios };
      })
      .filter((project) => project.scenarios.length > 0);
  }, [projectList, query]);

  const scenarioCount = projectList.reduce((sum, project) => sum + project.scenarios.length, 0);
  const treeItems = useMemo(() => buildScenarioTree(projectList), [projectList]);
  const favoriteItems = useMemo(() => {
    return projectList.flatMap((project) =>
      project.scenarios.slice(0, 2).map((scenario) => ({
        name: `${project.name} · ${scenario.name}`,
        emoji: scenario.isDefault ? '⭐️' : '📌',
      })),
    );
  }, [projectList]);
  const [activeSection, setActiveSection] = useState<NavigationSectionId>('scenarios');
  const activeSectionLabel =
    NAV_SECTIONS.find((section) => section.id === activeSection)?.label ?? 'Scenarios';

  return (
    <Sidebar variant="inset" collapsible="icon" className="overflow-hidden">
      <div className="flex min-h-screen">
        <Sidebar collapsible="none" className="flex-1">
          <SidebarHeader className="gap-3.5 border-b p-4">
            <div className="flex w-full items-center justify-between">
              <div className="text-base font-medium text-foreground">{activeSectionLabel}</div>
              <Badge variant="secondary" className="text-xs">
                {scenarioCount.toString()} scenarios
              </Badge>
            </div>
            <SidebarInput
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
              }}
              placeholder={`Filter ${scenarioCount.toString()} scenarios…`}
              aria-label="Filter scenarios"
              className="bg-background"
            />
          </SidebarHeader>
          <SidebarContent>
            <SidebarGroup className="space-y-1">
              <SidebarGroupLabel className="text-xs uppercase tracking-[0.08em] text-muted-foreground">
                Navigation
              </SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu>
                  {NAV_SECTIONS.map((item) => (
                    <SidebarMenuItem key={item.id}>
                      <SidebarMenuButton
                        tooltip={{ children: item.label, hidden: false }}
                        isActive={activeSection === item.id}
                        onClick={() => {
                          setActiveSection(item.id);
                          setOpen(true);
                        }}
                        className="px-3"
                        data-state={activeSection === item.id ? 'active' : undefined}
                      >
                        <item.icon />
                        <span>{item.label}</span>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  ))}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
            <Separator />
            {favoriteItems.length > 0 ? <FavoritesList items={favoriteItems} /> : undefined}
            <SidebarGroup>
              <SidebarGroupLabel className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
                Projects
              </SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu>
                  {renderProjectsSidebarMenu({
                    loading,
                    errorMessage,
                    projectList,
                    filteredProjects,
                    query,
                    activeScenarioId,
                    onSelectScenario,
                    onRetry,
                    onRevealSidebar: () => {
                      setActiveSection('scenarios');
                      setOpen(true);
                    },
                  })}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
            <SidebarGroup>
              <SidebarGroupLabel className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
                Files
              </SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu>
                  {treeItems.map((item, index) => (
                    <Tree key={`${activeSection}-${index.toString()}`} item={item} />
                  ))}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          </SidebarContent>
          <SidebarFooter>
            <SidebarFooterMenu />
          </SidebarFooter>
        </Sidebar>
      </div>
    </Sidebar>
  );
}

/**
 * Build a simple tree structure from projects and scenarios.
 * @param projectList - Projects with scenarios.
 * @returns Tree data for the file-style navigation.
 */
function buildScenarioTree(projectList: ProjectSummary[]): TreeItem[] {
  return projectList.map((project) => [
    project.name,
    ...project.scenarios.map((scenario) => scenario.name),
  ]);
}

/**
 * Render a collapsible file tree for scenarios.
 * @param root0
 * @param root0.item
 */
function Tree({ item }: { readonly item: TreeItem }) {
  const [nameValue, ...items] = Array.isArray(item) ? item : [item];
  const name = String(nameValue);

  if (items.length === 0) {
    return (
      <SidebarMenuButton>
        <File />
        <span>{name}</span>
      </SidebarMenuButton>
    );
  }

  return (
    <SidebarMenuItem>
      <Collapsible
        className="group/collapsible [&[data-state=open]>button>svg:first-child]:rotate-90"
        defaultOpen={name === 'Projects' || name === 'Default'}
      >
        <CollapsibleTrigger asChild>
          <SidebarMenuButton>
            <ChevronRight className="transition-transform" />
            <Folder />
            <span>{name}</span>
          </SidebarMenuButton>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <SidebarMenuSub>
            {items.map((subItem, index) => (
              <Tree key={`${name}-${index.toString()}`} item={subItem} />
            ))}
          </SidebarMenuSub>
        </CollapsibleContent>
      </Collapsible>
    </SidebarMenuItem>
  );
}

/**
 *
 * @param value
 */
function formatDate(value: string | undefined): string {
  if (!value) {
    return 'recently';
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toLocaleDateString();
}
