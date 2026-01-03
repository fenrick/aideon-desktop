import { isTauri } from 'praxis/platform';
import { listScenarios, type ScenarioSummary } from 'praxis/praxis-api';
import { BUILT_IN_TEMPLATES, type CanvasTemplate } from 'praxis/templates';
import { invokeIpc } from '../../adapters/ipc';
import { toErrorMessage } from './lib/errors';

interface ProjectPayload {
  readonly id?: string;
  readonly name?: string;
  readonly scenarios?: ScenarioSummary[];
}

interface TemplatePayload extends Partial<CanvasTemplate> {
  readonly id?: string;
  readonly name?: string;
}

export interface ProjectSummary {
  readonly id: string;
  readonly name: string;
  readonly scenarios: ScenarioSummary[];
}

const COMMANDS = {
  listProjects: 'workspace.projects.list',
  listTemplates: 'workspace.templates.list',
} as const;

/**
 * Fetch projects (with scenarios) from the host, falling back to a derived default project.
 */
export async function listProjectsWithScenarios(): Promise<ProjectSummary[]> {
  if (!isTauri()) {
    const fallback = await fallbackProjects();
    return fallback;
  }

  try {
    const payload = await invokeIpc<ProjectPayload[]>(COMMANDS.listProjects, {});
    const projects = Array.isArray(payload) ? payload : [];
    if (projects.length === 0) {
      const fallback = await fallbackProjects();
      return fallback;
    }
    return projects.map((entry) => normaliseProject(entry));
  } catch (error) {
    toErrorMessage(error);
    const fallback = await fallbackProjects();
    return fallback;
  }
}

/**
 * Load template definitions from the host; defaults to built-in templates for dev/preview.
 */
export async function listTemplatesFromHost(): Promise<CanvasTemplate[]> {
  if (!isTauri()) {
    return BUILT_IN_TEMPLATES;
  }
  try {
    const payload = await invokeIpc<TemplatePayload[]>(COMMANDS.listTemplates, {});
    const templates = Array.isArray(payload) ? payload : [];
    if (templates.length === 0) {
      return BUILT_IN_TEMPLATES;
    }
    return templates.map((template) => normaliseTemplate(template));
  } catch (error) {
    toErrorMessage(error);
    return BUILT_IN_TEMPLATES;
  }
}

/**
 *
 * @param payload
 */
function normaliseProject(payload: ProjectPayload): ProjectSummary {
  return {
    id: payload.id ?? cryptoRandomId('project'),
    name: payload.name?.trim() ?? 'Project',
    scenarios: Array.isArray(payload.scenarios) ? payload.scenarios : [],
  };
}

/**
 *
 * @param payload
 */
function normaliseTemplate(payload: TemplatePayload): CanvasTemplate {
  const fallback = BUILT_IN_TEMPLATES[0] ?? {
    id: 'template-default',
    name: 'Template',
    description: '',
    widgets: [],
  };
  return {
    id: payload.id ?? cryptoRandomId('template'),
    name: payload.name?.trim() ?? fallback.name,
    description: payload.description ?? fallback.description,
    widgets:
      Array.isArray(payload.widgets) && payload.widgets.length > 0
        ? payload.widgets
        : fallback.widgets,
  } satisfies CanvasTemplate;
}

/**
 *
 * @param prefix
 */
function cryptoRandomId(prefix: string): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return `${prefix}-${crypto.randomUUID()}`;
  }
  return `${prefix}-${Date.now().toString(36)}`;
}

/**
 *
 */
async function fallbackProjects(): Promise<ProjectSummary[]> {
  const scenarios = await listScenarios();
  return [
    {
      id: 'default-project',
      name: 'Praxis Workspace',
      scenarios,
    },
  ];
}
