import type { SearchIndexItem } from './types';

export const normalize = (value: string): string =>
  value
    .normalize('NFKD')
    .replaceAll(/\p{Diacritic}/gu, '')
    .toLowerCase();

export const tokenize = (value: string): readonly string[] => {
  const normalized = normalize(value)
    .split(/\s+/)
    .map((token) => token.trim())
    .filter(Boolean);
  const uniqueTokens: string[] = [];
  const seen = new Set<string>();
  for (const token of normalized) {
    if (!seen.has(token)) {
      seen.add(token);
      uniqueTokens.push(token);
    }
  }
  return uniqueTokens;
};

export const scoreItem = (item: SearchIndexItem, tokens: readonly string[]): number => {
  if (tokens.length === 0) {
    return 0;
  }
  const hasAllTokens = tokens.every(
    (token) => item.tokenSet.has(token) || item.searchValue.includes(token),
  );
  if (!hasAllTokens) {
    return 0;
  }
  const scoreToken = (token: string) => {
    let score = 1;
    if (item.titleValue.startsWith(token)) {
      score += 1.25;
    }
    if (item.kind === 'commit' && item.titleValue.includes(token)) {
      score += 0.5;
    }
    return score;
  };
  return tokens.reduce((total, token) => total + scoreToken(token), item.priority);
};
