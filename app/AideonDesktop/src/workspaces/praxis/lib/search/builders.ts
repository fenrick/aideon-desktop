import type {
  CatalogEntitySummary,
  CatalogSelectHandler,
  SearchIndexItem,
  SearchResultKind,
  SidebarSelectHandler,
  SidebarTreeNode,
  TemporalCommitSummary,
} from './types';
import { normalize, tokenize } from './utilities';

const basePriority: Record<SearchResultKind, number> = {
  sidebar: 1,
  catalog: 0.75,
  commit: 0.5,
};

const createIndexItem = (
  result: Omit<SearchIndexItem, 'tokens' | 'tokenSet' | 'titleValue' | 'priority' | 'searchValue'>,
  tokens: readonly string[],
  priority: number,
  titleValue: string,
): SearchIndexItem => {
  const tokenList = [...tokens];
  return {
    ...result,
    tokens: tokenList,
    tokenSet: new Set(tokenList),
    searchValue: tokenList.join(' '),
    priority,
    titleValue,
  };
};

const createSidebarEntry = (
  item: SidebarTreeNode,
  path: string[],
  onSelect?: SidebarSelectHandler,
): SearchIndexItem => {
  const title = item.label;
  const subtitle = path.slice(0, -1).join(' • ');
  const searchTokens = [title, subtitle, path.join(' ')].filter(Boolean).join(' ');
  const tokens = tokenize(searchTokens);
  const titleValue = normalize(title);
  const priority = basePriority.sidebar + (path.length > 1 ? 0.1 : 0);
  return createIndexItem(
    {
      id: `sidebar:${item.id}`,
      title,
      subtitle: subtitle || undefined,
      kind: 'sidebar',
      run: onSelect ? () => onSelect(item.id) : undefined,
    },
    tokens,
    priority,
    titleValue,
  );
};

export const buildSidebarIndex = (
  items: SidebarTreeNode[],
  onSelect?: SidebarSelectHandler,
  parents: string[] = [],
): SearchIndexItem[] =>
  items.flatMap((item) => {
    const path = [...parents, item.label];
    const entry = createSidebarEntry(item, path, onSelect);
    if (!item.children?.length) {
      return [entry];
    }
    return [entry, ...buildSidebarIndex(item.children, onSelect, path)];
  });

export const buildCommitIndex = (
  commits: TemporalCommitSummary[],
  onSelect?: (id: string) => void | Promise<void>,
): SearchIndexItem[] =>
  commits.map((commit) => {
    const subtitleParts = [commit.time, commit.author].filter(Boolean);
    const subtitle = subtitleParts.join(' • ') || undefined;
    const keywords = [commit.id, commit.branch, commit.message, ...commit.tags];
    const tokens = tokenize(keywords.join(' '));
    const titleValue = normalize(commit.message);
    const priority = basePriority.commit + Math.min(commit.changeCount / 10, 0.5);
    return createIndexItem(
      {
        id: `commit:${commit.id}`,
        title: commit.message,
        subtitle,
        kind: 'commit',
        run: onSelect
          ? () => {
              const outcome = onSelect(commit.id);
              if (outcome instanceof Promise) {
                outcome.catch(() => false);
              }
            }
          : undefined,
      },
      tokens,
      priority,
      titleValue,
    );
  });

export const buildCatalogIndex = (
  entities: CatalogEntitySummary[],
  onSelect?: CatalogSelectHandler,
): SearchIndexItem[] =>
  entities.map((entity) => {
    const subtitleParts = [entity.type, entity.description].filter(Boolean);
    const subtitle = subtitleParts.join(' • ') || undefined;
    const keywords = [entity.name, entity.type];
    if (entity.description) {
      keywords.push(entity.description);
    }
    if (entity.sidebarId) {
      keywords.push(entity.sidebarId);
    }
    const tokens = tokenize(keywords.join(' '));
    const titleValue = normalize(entity.name);
    const priority = basePriority.catalog;
    return createIndexItem(
      {
        id: `catalog:${entity.id}`,
        title: entity.name,
        subtitle,
        kind: 'catalog',
        run: onSelect ? () => onSelect(entity) : undefined,
      },
      tokens,
      priority,
      titleValue,
    );
  });
