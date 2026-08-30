import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ReactFlowProvider } from '@xyflow/react';
import { FlowSummaryNode } from './FlowSummaryNode';
import type { UserScript } from '../../../types/automation';

function nodeProps(script: UserScript) {
  return { id: script.id, data: { script } } as unknown as Parameters<typeof FlowSummaryNode>[0];
}

describe('FlowSummaryNode', () => {
  it('shows name, kind badge, and condition/action counts for a Blockly-authored flow', () => {
    const script: UserScript = {
      id: 's1',
      device_id: 'd1',
      kind: 'alert',
      name: 'pH cao',
      source: '',
      enabled: true,
      ir_json: {
        kind: 'alert',
        trigger: { type: 'sensor' },
        conditions: [{ sensor: 'ph', operator: '>', value: 7.5 }],
        actions: [{ type: 'alert', level: 'warning', message: 'pH cao' }],
        nodes: [],
        edges: [],
      },
      created_at: '',
      updated_at: '',
    };
    render(
      <ReactFlowProvider>
        <FlowSummaryNode {...nodeProps(script)} />
      </ReactFlowProvider>,
    );
    expect(screen.getByText('pH cao')).toBeInTheDocument();
    expect(screen.getByText('alert')).toBeInTheDocument();
    expect(screen.getByText('1 điều kiện → 1 hành động')).toBeInTheDocument();
  });

  it('shows "Script viết tay" for scripts without ir_json', () => {
    const script: UserScript = {
      id: 's2',
      device_id: 'd1',
      kind: 'recipe_override',
      name: 'Manual script',
      source: '',
      enabled: false,
      ir_json: null,
      created_at: '',
      updated_at: '',
    };
    render(
      <ReactFlowProvider>
        <FlowSummaryNode {...nodeProps(script)} />
      </ReactFlowProvider>,
    );
    expect(screen.getByText('Script viết tay')).toBeInTheDocument();
  });
});
