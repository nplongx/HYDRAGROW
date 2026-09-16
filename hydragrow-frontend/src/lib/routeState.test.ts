import { describe, expect, it } from 'vitest';
import { legacyTarget, parseTab, serializeTab } from './routeState';

describe('route URL state', () => {
  it('uses defaults and ignores invalid tabs', () => {
    expect(parseTab('', ['control', 'automation'], 'control')).toBe('control');
    expect(parseTab('?tab=bad', ['control', 'automation'], 'control')).toBe('control');
    expect(parseTab('?tab=automation', ['control', 'automation'], 'control')).toBe('automation');
  });

  it('serializes defaults canonically and preserves unrelated search state', () => {
    expect(serializeTab('?filter=open', 'automation', 'control')).toBe('filter=open&tab=automation');
    expect(serializeTab('?filter=open&tab=automation', 'control', 'control')).toBe('filter=open');
  });

  it('maps legacy deep links to canonical tab URLs and preserves search/hash', () => {
    expect(legacyTarget('/automation', '?filter=open', '#flows')).toBe('/operations?filter=open&tab=automation#flows');
    expect(legacyTarget('/recipes', '?filter=open', '#x')).toBe('/cultivation?filter=open&tab=recipes#x');
    expect(legacyTarget('/analytics', '', '')).toBe('/journal?tab=analytics');
    expect(legacyTarget('/logs', '?cursor=2', '#events')).toBe('/journal?cursor=2#events');
    expect(legacyTarget('/unknown', '', '')).toBeNull();
  });

  it('uses the central route manifest as the alias source of truth', () => {
    expect(legacyTarget('/control', '', '')).toBe('/operations');
    expect(legacyTarget('/crop-seasons', '', '')).toBe('/cultivation');
  });
});
