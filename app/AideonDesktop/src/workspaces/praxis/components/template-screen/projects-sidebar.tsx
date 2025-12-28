import { useMemo, useState } from 'react';

import { Badge } from 'design-system/components/ui/badge';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from 'design-system/components/ui/collapsible';
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
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarRail,
  useSidebar,
} from 'design-system/desktop-shell';

import { Button } from 'design-system/components/ui/button';
import {
  ChevronRight,
  File,
  Folder,
  LayersIcon,
  LayoutPanelTop,
  Network,
  Settings2,
} from 'lucide-react';
import type { ProjectSummary } from 'praxis/domain-data';
import type { ScenarioSummary } from 'praxis/praxis-api';

const NAV_SECTIONS = [
  { id: 'overview', label: 'Overview', icon: LayoutPanelTop },
  { id: 'scenarios', label: 'Scenarios', icon: LayersIcon },
  { id: 'canvas', label: 'Canvases', icon: Network },
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
      <SidebarMenuItem key={headerId} className="mt-2">
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
    <SidebarMenuItem key={headerId} className="mt-2">
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
              Branch {scenario.branch} · Updated {formatDate(scenario.updatedAt)}
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
  const [activeSection, setActiveSection] = useState<NavigationSectionId>('scenarios');
  const activeSectionLabel =
    NAV_SECTIONS.find((section) => section.id === activeSection)?.label ?? 'Scenarios';

  return (
    <Sidebar
      variant="inset"
      collapsible="icon"
      className="overflow-hidden *:data-[sidebar=sidebar]:flex-row"
    >
      <Sidebar collapsible="none" className="w-[calc(var(--sidebar-width-icon)+1px)]! border-r">
        <SidebarHeader>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton size="lg" className="md:h-8 md:p-0">
                <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
                  <LayersIcon className="size-4" />
                </div>
                <div className="grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-medium">Praxis</span>
                  <span className="truncate text-xs">Workspace</span>
                </div>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarHeader>
        <SidebarContent>
          <SidebarGroup>
            <SidebarGroupContent className="px-1.5 md:px-0">
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
                      className="px-2.5 md:px-2"
                    >
                      <item.icon />
                      <span>{item.label}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarContent>
        <SidebarFooter>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton>
                <Settings2 />
                <span>Workspace settings</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarFooter>
      </Sidebar>

      <Sidebar collapsible="none" className="hidden flex-1 md:flex">
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
        <SidebarRail />
      </Sidebar>
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
