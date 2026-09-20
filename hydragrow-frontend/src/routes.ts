export type RouteClass =
  | 'public'
  | 'authenticated'
  | 'utility'
  | 'legacy'
  | 'recovery';

export type RouteScope = 'global' | 'station' | 'device' | 'admin';
export type EntryPolicy = 'push' | 'replace' | 'redirect' | 'render';

export interface RouteDefinition {
  id: string;
  path: string;
  class: RouteClass;
  capability?: string;
  scope: RouteScope;
  canonicalPath?: string;
  preserveSearch?: boolean;
  preserveHash?: boolean;
  entryPolicy: EntryPolicy;
}

export interface LegacyRouteDefinition extends RouteDefinition {
  class: 'legacy';
  canonicalPath: string;
  tab?: string;
  semantic: 'preserve-subview' | 'collapse-to-parent' | 'retired-with-explicit-break';
  reason: string;
}

export const CANONICAL_ROUTES = [
  { id: 'dashboard', path: '/dashboard', class: 'authenticated', scope: 'global', entryPolicy: 'render' },
  { id: 'operations', path: '/operations', class: 'authenticated', scope: 'device', entryPolicy: 'render' },
  { id: 'cultivation', path: '/cultivation', class: 'authenticated', scope: 'device', entryPolicy: 'render' },
  { id: 'journal', path: '/journal', class: 'authenticated', scope: 'device', entryPolicy: 'render' },
  { id: 'settings', path: '/settings', class: 'authenticated', scope: 'device', entryPolicy: 'render' },
  { id: 'pairing', path: '/pairing', class: 'utility', scope: 'global', entryPolicy: 'render' },
  { id: 'fleet', path: '/fleet', class: 'utility', scope: 'global', entryPolicy: 'render' },
  { id: 'config-backup', path: '/config-backup', class: 'utility', scope: 'device', entryPolicy: 'render' },
  { id: 'user-management', path: '/user-management', class: 'utility', scope: 'admin', capability: 'admin', entryPolicy: 'render' },
  { id: 'roles', path: '/roles', class: 'utility', scope: 'admin', capability: 'admin', entryPolicy: 'render' },
] as const satisfies readonly RouteDefinition[];

export const LEGACY_ROUTES = [
  { id: 'legacy-root', path: '/', canonicalPath: '/dashboard', class: 'legacy', scope: 'global', entryPolicy: 'replace', preserveSearch: true, preserveHash: true, semantic: 'collapse-to-parent', reason: 'Root entry now resolves to dashboard.' },
  { id: 'legacy-control', path: '/control', canonicalPath: '/operations', tab: 'control', class: 'legacy', scope: 'device', entryPolicy: 'replace', preserveSearch: true, preserveHash: true, semantic: 'preserve-subview', reason: 'Control is now an Operations sub-view.' },
  { id: 'legacy-automation', path: '/automation', canonicalPath: '/operations', tab: 'automation', class: 'legacy', scope: 'device', entryPolicy: 'replace', preserveSearch: true, preserveHash: true, semantic: 'preserve-subview', reason: 'Automation is now an Operations sub-view.' },
  { id: 'legacy-seasons', path: '/seasons', canonicalPath: '/cultivation', tab: 'seasons', class: 'legacy', scope: 'device', entryPolicy: 'replace', preserveSearch: true, preserveHash: true, semantic: 'preserve-subview', reason: 'Seasons is now a Cultivation sub-view.' },
  { id: 'legacy-crop-seasons', path: '/crop-seasons', canonicalPath: '/cultivation', tab: 'seasons', class: 'legacy', scope: 'device', entryPolicy: 'replace', preserveSearch: true, preserveHash: true, semantic: 'preserve-subview', reason: 'Crop seasons is now a Cultivation sub-view.' },
  { id: 'legacy-recipes', path: '/recipes', canonicalPath: '/cultivation', tab: 'recipes', class: 'legacy', scope: 'device', entryPolicy: 'replace', preserveSearch: true, preserveHash: true, semantic: 'preserve-subview', reason: 'Recipes is now a Cultivation sub-view.' },
  { id: 'legacy-dosing-history', path: '/dosing-history', canonicalPath: '/cultivation', tab: 'dosing-history', class: 'legacy', scope: 'device', entryPolicy: 'replace', preserveSearch: true, preserveHash: true, semantic: 'preserve-subview', reason: 'Dosing history is now a Cultivation sub-view.' },
  { id: 'legacy-logs', path: '/logs', canonicalPath: '/journal', tab: 'events', class: 'legacy', scope: 'device', entryPolicy: 'replace', preserveSearch: true, preserveHash: true, semantic: 'preserve-subview', reason: 'Logs is now the Journal events sub-view.' },
  { id: 'legacy-analytics', path: '/analytics', canonicalPath: '/journal', tab: 'analytics', class: 'legacy', scope: 'device', entryPolicy: 'replace', preserveSearch: true, preserveHash: true, semantic: 'preserve-subview', reason: 'Analytics is now a Journal sub-view.' },
] as const satisfies readonly LegacyRouteDefinition[];

export const ROUTE_MANIFEST = [...CANONICAL_ROUTES, ...LEGACY_ROUTES] as const;

export const PRIMARY_ROUTE_IDS = [
  'dashboard',
  'operations',
  'cultivation',
  'journal',
  'settings',
] as const;

export type CanonicalRouteId = (typeof CANONICAL_ROUTES)[number]['id'];

export function routePath(id: CanonicalRouteId): string {
  return getRouteById(id)?.path ?? '/';
}

export function getRouteById(id: string): RouteDefinition | undefined {
  return ROUTE_MANIFEST.find((route) => route.id === id);
}

export function resolveRoute(pathname: string): RouteDefinition | undefined {
  return ROUTE_MANIFEST.find((route) => route.path === pathname);
}

export function canonicalPathFor(pathname: string, search = '', hash = ''): string | undefined {
  const route = resolveRoute(pathname);
  if (!route) return undefined;
  if (route.class !== 'legacy') return `${route.path}${search}${hash}`;

  const params = new URLSearchParams(search);
  const legacy = route as LegacyRouteDefinition;
  if (legacy.tab && legacy.tab !== 'events' && legacy.tab !== 'control' && legacy.tab !== 'seasons') {
    params.set('tab', legacy.tab);
  } else {
    params.delete('tab');
  }

  const query = route.preserveSearch ? params.toString() : '';
  const fragment = route.preserveHash ? hash : '';
  return `${route.canonicalPath}${query ? `?${query}` : ''}${fragment}`;
}
