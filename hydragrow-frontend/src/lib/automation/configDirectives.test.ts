import { describe, expect, it } from 'vitest';
import { collectConfigDirectives } from './configDirectives';
import type { GraphNode } from './contextVariables';

function configNode(id: string, data: Record<string, unknown>): GraphNode {
  return { id, type: 'config', data };
}

describe('collectConfigDirectives', () => {
  it('collects every config_read node as a contextRead', () => {
    const nodes: GraphNode[] = [
      configNode('a', { variant: 'read', configKey: 'ph_target', saveToVariable: 'ph_target_now' }),
      configNode('b', { variant: 'read', configKey: 'ec_target', saveToVariable: 'ec_target_now' }),
    ];
    expect(collectConfigDirectives(nodes).contextReads).toEqual([
      { configKey: 'ph_target', saveToVariable: 'ph_target_now' },
      { configKey: 'ec_target', saveToVariable: 'ec_target_now' },
    ]);
  });

  it('ignores a config_read node missing configKey or saveToVariable', () => {
    const nodes: GraphNode[] = [
      configNode('a', { variant: 'read', configKey: '', saveToVariable: 'x' }),
      configNode('b', { variant: 'read', configKey: 'ph_target', saveToVariable: '' }),
    ];
    expect(collectConfigDirectives(nodes).contextReads).toEqual([]);
  });

  it('collects the first config_overwrite node as configOverwrite, ignoring later ones', () => {
    const nodes: GraphNode[] = [
      configNode('a', { variant: 'overwrite', configKey: 'ec_target', overrideValue: '1.8', readOriginalBeforeWrite: true }),
      configNode('b', { variant: 'overwrite', configKey: 'ph_target', overrideValue: '6.0' }),
    ];
    expect(collectConfigDirectives(nodes).configOverwrite).toEqual({
      configKey: 'ec_target',
      value: '1.8',
      readOriginalBeforeWrite: true,
      restoreMode: 'on_condition_false',
    });
  });

  it('returns configOverwrite undefined when no overwrite node is configured', () => {
    const nodes: GraphNode[] = [configNode('a', { variant: 'read', configKey: 'ph_target', saveToVariable: 'x' })];
    expect(collectConfigDirectives(nodes).configOverwrite).toBeUndefined();
  });
});
