import { Layers, LayoutPanelTop, Network, NotebookTabs, Settings2 } from 'lucide-react';

import {
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from 'design-system/components/ui/sidebar';
import type { ScenarioSummary } from 'praxis/praxis-api';

const NAV_ITEMS = [
  { label: 'Overview', icon: LayoutPanelTop, active: true },
  { label: 'Workflows', icon: NotebookTabs },
  { label: 'Canvases', icon: Network },
  { label: 'Catalogues', icon: Layers },
];

interface AppSidebarProperties {
  readonly scenarios: ScenarioSummary[];
  readonly loading: boolean;
}

/**
 * Application sidebar with navigation and active scenario summary.
 * @param root0 - Sidebar properties.
 * @param root0.scenarios - Scenario summaries to display.
 * @param root0.loading - Whether scenarios are still loading.
 * @returns Sidebar element.
 */
export function AppSidebar({ scenarios, loading }: AppSidebarProperties) {
  const activeScenario = resolveActiveScenario(scenarios);
  const subtitle = loading
    ? 'Loading scenario data...'
    : (activeScenario?.description ?? 'No scenario metadata yet');
  const branchLabel = activeScenario
    ? `${activeScenario.branch} - updated ${formatDate(activeScenario.updatedAt)}`
    : 'Add a scenario to begin';

  return (
    <>
      <SidebarHeader className="gap-3 px-4 py-5">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-full bg-sidebar-accent/60 text-lg font-semibold text-sidebar-accent-foreground">
            PX
          </div>
          <div>
            <p className="text-sm uppercase tracking-wide text-sidebar-foreground/70">
              Praxis Workspace
            </p>
            <p className="text-base font-semibold text-sidebar-foreground">Digital Twin</p>
          </div>
        </div>
      </SidebarHeader>
      <SidebarSeparator />
      <SidebarContent className="px-2 py-3">
        <SidebarGroup>
          <SidebarGroupLabel>Navigation</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {NAV_ITEMS.map((item) => (
                <SidebarMenuItem key={item.label}>
                  <SidebarMenuButton isActive={item.active}>
                    <item.icon />
                    <span>{item.label}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>Active Scenario</SidebarGroupLabel>
          <SidebarGroupContent>
            <div className="rounded-lg border border-sidebar-border/60 bg-sidebar-accent/40 p-3 text-xs text-sidebar-foreground/70">
              <p className="text-sm font-semibold text-sidebar-foreground">
                {loading
                  ? 'Resolving scenario...'
                  : (activeScenario?.name ?? 'No scenario selected')}
              </p>
              <p className="text-xs text-sidebar-foreground/75">{subtitle}</p>
              <p className="mt-2 text-[0.65rem] uppercase tracking-[0.35em] text-sidebar-foreground/60">
                {branchLabel}
              </p>
            </div>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter className="border-t border-sidebar-border/60 p-3">
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton>
              <Settings2 className="h-4 w-4" />
              <span>Workspace Settings</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </>
  );
}

/**
 * Resolve the first default scenario or fall back to the first entry.
 * @param scenarios - Available scenarios.
 * @returns Scenario to treat as active.
 */
function resolveActiveScenario(scenarios: ScenarioSummary[]): ScenarioSummary | undefined {
  return scenarios.find((scenario) => scenario.isDefault) ?? scenarios[0];
}

/**
 * Format an ISO date string to a readable date or default text.
 * @param value - ISO date string.
 * @returns Localized date string or fallback.
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
