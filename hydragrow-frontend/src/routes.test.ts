import { describe, expect, it } from 'vitest';
import {
  CANONICAL_ROUTES,
  LEGACY_ROUTES,
  PRIMARY_ROUTE_IDS,
  ROUTE_MANIFEST,
  canonicalPathFor,
  getRouteById,
  resolveRoute,
} from './routes';

describe('route manifest', () => {
  it('gives every canonical destination a unique stable id and path', () => {
    expect(new Set(CANONICAL_ROUTES.map((route) => route.id)).size).toBe(CANONICAL_ROUTES.length);
    expect(new Set(CANONICAL_ROUTES.map((route) => route.path)).size).toBe(CANONICAL_ROUTES.length);
    expect(CANONICAL_ROUTES.every((route) => route.scope && route.entryPolicy)).toBe(true);
  });

  it('makes user-management and roles separate route owners', () => {
    expect(resolveRoute('/user-management')?.id).toBe('user-management');
    expect(resolveRoute('/roles')?.id).toBe('roles');
    expect(resolveRoute('/user-management')?.capability).toBe('admin');
    expect(resolveRoute('/roles')?.capability).toBe('admin');
  });

  it('keeps primary navigation canonical', () => {
    expect(PRIMARY_ROUTE_IDS.every((id) => getRouteById(id))).toBe(true);
  });

  it('resolves every legacy alias to one canonical target with replace semantics', () => {
    expect(LEGACY_ROUTES.every((route) => route.entryPolicy === 'replace' && resolveRoute(route.canonicalPath))).toBe(true);
  });

  it('preserves alias query/hash and meaningful merged-page sub-view', () => {
    expect(canonicalPathFor('/automation', '?filter=open', '#flows')).toBe('/operations?filter=open&tab=automation#flows');
    expect(canonicalPathFor('/recipes', '?filter=open', '#x')).toBe('/cultivation?filter=open&tab=recipes#x');
    expect(canonicalPathFor('/logs', '?cursor=2', '#events')).toBe('/journal?cursor=2#events');
  });

  it('returns no route for an unknown path', () => {
    expect(resolveRoute('/does-not-exist')).toBeUndefined();
    expect(canonicalPathFor('/does-not-exist')).toBeUndefined();
  });

  it('keeps manifest inventory complete across canonical and legacy routes', () => {
    expect(ROUTE_MANIFEST.length).toBe(CANONICAL_ROUTES.length + LEGACY_ROUTES.length);
  });
});
