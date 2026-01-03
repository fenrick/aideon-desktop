export type SearchResultKind = 'sidebar' | 'commit' | 'catalog';

export interface SidebarTreeNode {
  id: string;
  label: string;
  children?: SidebarTreeNode[];
}

export interface CatalogEntitySummary {
  id: string;
  name: string;
  type: string;
  description?: string;
  sidebarId?: string;
}

export interface SearchResult {
  id: string;
  title: string;
  subtitle?: string;
  kind: SearchResultKind;
  run?: () => void | Promise<void>;
}

export interface SearchIndexItem extends SearchResult {
  tokens: readonly string[];
  tokenSet: ReadonlySet<string>;
  searchValue: string;
  titleValue: string;
  priority: number;
}

export type SidebarSelectHandler = (id: string) => void | Promise<void>;
export type CommitSelectHandler = (id: string) => void | Promise<void>;
export type CatalogSelectHandler = (entity: CatalogEntitySummary) => void | Promise<void>;

export type SourceKey = 'sidebar' | 'commits' | 'catalog';

import type { TemporalCommitSummary as PraxisTemporalCommitSummary } from '../../praxis-api';

export type TemporalCommitSummary = PraxisTemporalCommitSummary;

export interface SearchSourceBuilders {
  sidebar: (items: SidebarTreeNode[], onSelect?: SidebarSelectHandler) => SearchIndexItem[];
  commits: (commits: TemporalCommitSummary[], onSelect?: CommitSelectHandler) => SearchIndexItem[];
  catalog: (entities: CatalogEntitySummary[], onSelect?: CatalogSelectHandler) => SearchIndexItem[];
}
