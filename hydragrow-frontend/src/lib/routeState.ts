import { LEGACY_ROUTES, type LegacyRouteDefinition } from '../routes';

export const ROUTE_TABS = {
  operations: ['control', 'automation'],
  cultivation: ['seasons', 'recipes', 'dosing-history'],
  journal: ['events', 'analytics'],
} as const;

export type RouteTab = (typeof ROUTE_TABS)[keyof typeof ROUTE_TABS][number];

const TAB_ALIASES: Record<string, RouteTab> = {
  control: 'control',
  automation: 'automation',
  seasons: 'seasons',
  recipes: 'recipes',
  'dosing-history': 'dosing-history',
  events: 'events',
  analytics: 'analytics',
  logs: 'events',
};

export function parseTab(search: string, allowed: readonly string[], defaultTab: string): string {
  const value = new URLSearchParams(search).get('tab');
  return value && allowed.includes(value) ? value : defaultTab;
}

export function serializeTab(search: string, tab: string, defaultTab: string): string {
  const params = new URLSearchParams(search);
  if (tab === defaultTab) params.delete('tab');
  else params.set('tab', tab);
  return params.toString();
}

export function legacyTarget(pathname: string, search: string, hash: string): string | null {
  const route = LEGACY_ROUTES.find((candidate) => candidate.path === pathname) as LegacyRouteDefinition | undefined;
  if (!route) return null;

  const params = new URLSearchParams(search);
  if (route.tab === 'events' || route.tab === 'control' || route.tab === 'seasons') {
    params.delete('tab');
  } else if (route.tab) {
    params.set('tab', route.tab);
  }
  const query = route.preserveSearch ? params.toString() : '';
  const fragment = route.preserveHash ? hash : '';
  return `${route.canonicalPath}${query ? `?${query}` : ''}${fragment}`;
}

export function normalizeLegacyTab(value: string): RouteTab | null {
  return TAB_ALIASES[value] ?? null;
}
