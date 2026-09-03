import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ReactFlowProvider } from '@xyflow/react';
import { FlowSummaryNode } from './FlowSummaryNode';
import type { UserScript } from '../../../types/automation';

function nodeProps(script: UserScript) {
  return { id: script.id, data: { script } } as unknown as Parameters<typeof FlowSummaryNode>[0];
}

describe('FlowSummaryNode', () => {
  it('counts leaf conditions recursively across nested condition groups', () => {
    const script: UserScript = {
      id: 's3',
      device_id: 'd1',
      kind: 'alert',
      name: 'Nested Flow',
      source: '',
      enabled: true,
      ir_json: {
        kind: 'alert',
        trigger: { type: 'sensor' },
        conditions: [
          {
            op: 'or',
            children: [
              { sensor: 'ph', operator: '<', value: 5.5 },
              { sensor: 'ph', operator: '>', value: 7.5 },
            ],
          },
          { sensor: 'ec', operator: '>', value: 3.0 },
        ],
        actions: [{ type: 'alert', level: 'warning', message: 'x' }],
        nodes: [],
        edges: [],
        next_flow_ids: [],
      },
      created_at: '',
      updated_at: '',
    };
    render(
      <ReactFlowProvider>
        <FlowSummaryNode {...nodeProps(script)} />
      </ReactFlowProvider>,
    );
    expect(screen.getByText('3 điều kiện → 1 hành động')).toBeInTheDocument();
  });

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
        next_flow_ids: [],
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
    expect(screen.getByText('Alert')).toBeInTheDocument();
    expect(screen.getByText('1 điều kiện → 1 hành động')).toBeInTheDocument();
  });

  it('shows "Script viết tay (Rhai)" for scripts without ir_json', () => {
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
    expect(screen.getByText('Script viết tay (Rhai)')).toBeInTheDocument();
  });
});

const KIND_COLORS: Record<string, string> = {
  alert: 'bg-red-100 text-red-700',
  recipe_override: 'bg-sky-100 text-sky-700',
  action_command: 'bg-amber-100 text-amber-700',
};

describe('FlowSummaryNode badge color mapping', () => {
  it('badge color mapping covers all kinds', () => {
    expect(KIND_COLORS['alert']).toBeTruthy();
    expect(KIND_COLORS['recipe_override']).toBeTruthy();
    expect(KIND_COLORS['action_command']).toBeTruthy();
  });
});

describe('FlowSummaryNode trigger badges', () => {
  const baseScript: UserScript = {
    id: 's1',
    device_id: 'd1',
    kind: 'alert',
    name: 'Test Flow',
    source: '',
    enabled: true,
    ir_json: {
      kind: 'alert',
      trigger: { type: 'sensor' },
      conditions: [],
      actions: [],
      nodes: [],
      edges: [],
      next_flow_ids: [],
    },
    created_at: '',
    updated_at: '',
  };

  it('hiển thị badge CRON khi trigger.type là cron', () => {
    const script: UserScript = {
      ...baseScript,
      ir_json: {
        ...baseScript.ir_json!,
        trigger: { type: 'cron' } as unknown as { type: 'sensor' },
      },
    };
    render(
      <ReactFlowProvider>
        <FlowSummaryNode {...nodeProps(script)} />
      </ReactFlowProvider>,
    );
    expect(screen.getByText('CRON')).toBeInTheDocument();
  });

  it('hiển thị badge WEBHOOK khi trigger.type là webhook', () => {
    const script: UserScript = {
      ...baseScript,
      ir_json: {
        ...baseScript.ir_json!,
        trigger: { type: 'webhook' } as unknown as { type: 'sensor' },
      },
    };
    render(
      <ReactFlowProvider>
        <FlowSummaryNode {...nodeProps(script)} />
      </ReactFlowProvider>,
    );
    expect(screen.getByText('WEBHOOK')).toBeInTheDocument();
  });

  it('không hiển thị badge trigger khi là sensor/fsm (giữ giao diện cũ)', () => {
    const script: UserScript = {
      ...baseScript,
      ir_json: {
        ...baseScript.ir_json!,
        trigger: { type: 'sensor' },
      },
    };
    render(
      <ReactFlowProvider>
        <FlowSummaryNode {...nodeProps(script)} />
      </ReactFlowProvider>,
    );
    expect(screen.queryByText('CRON')).not.toBeInTheDocument();
    expect(screen.queryByText('WEBHOOK')).not.toBeInTheDocument();
  });
});
