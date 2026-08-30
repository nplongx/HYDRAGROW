import { describe, expect, it, beforeEach } from 'vitest';
import * as Blockly from 'blockly/core';
import { registerHydragrowBlocks } from './blocks';
import { extractActions, extractConditions } from './extractIr';
import { hydrateWorkspace } from './hydrateIr';

describe('hydrateWorkspace', () => {
  let workspace: Blockly.Workspace;

  beforeEach(() => {
    registerHydragrowBlocks();
    workspace = new Blockly.Workspace();
  });

  it('round-trips a condition and an alert action through extract', () => {
    hydrateWorkspace(
      workspace,
      [{ sensor: 'ph', operator: '>', value: 7.5 }],
      [{ type: 'alert', level: 'warning', message: 'pH cao' }],
    );
    expect(extractConditions(workspace)).toEqual([{ sensor: 'ph', operator: '>', value: 7.5 }]);
    expect(extractActions(workspace)).toEqual([{ type: 'alert', level: 'warning', message: 'pH cao' }]);
  });

  it('round-trips an advance_stage action', () => {
    hydrateWorkspace(workspace, [], [{ type: 'advance_stage', targetStageOffset: 2, reason: 'Đủ 24h' }]);
    expect(extractActions(workspace)).toEqual([
      { type: 'advance_stage', targetStageOffset: 2, reason: 'Đủ 24h' },
    ]);
  });

  it('does nothing on empty input', () => {
    hydrateWorkspace(workspace, [], []);
    expect(extractConditions(workspace)).toEqual([]);
    expect(extractActions(workspace)).toEqual([]);
  });

  it('round-trips multiple conditions in order', () => {
    hydrateWorkspace(
      workspace,
      [
        { sensor: 'ph', operator: '>', value: 7.5 },
        { sensor: 'ec', operator: '<', value: 1.0 },
      ],
      [{ type: 'alert', level: 'error', message: 'Nguy hiểm' }],
    );
    expect(extractConditions(workspace)).toHaveLength(2);
  });
});
