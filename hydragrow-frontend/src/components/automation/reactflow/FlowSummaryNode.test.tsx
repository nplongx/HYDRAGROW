import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { FlowSummaryNode } from './FlowSummaryNode';
import { ReactFlowProvider } from '@xyflow/react';
import type { UserScript } from '../../../types/automation';

describe('FlowSummaryNode', () => {
  it('shows name, kind badge, and trigger info', () => {
    const script: UserScript = {
      id: '1',
      name: 'pH cao',
      kind: 'alert',
      source: '',
      device_id: 'dev1',
      enabled: true,
      ir_json: {
        kind: 'alert',
        nodes: [{ id: 'trigger', type: 'trigger', data: { kind: 'sensor' } }],
        edges: [],
        next_flow_ids: [],
      } as any,
    } as any;

    render(
      <ReactFlowProvider>
        <FlowSummaryNode data={{ script }} />
      </ReactFlowProvider>
    );
    expect(screen.getByText('pH cao')).toBeInTheDocument();
    expect(screen.getByText('Cảnh báo')).toBeInTheDocument();
    expect(screen.getByText('SENSOR')).toBeInTheDocument();
    expect(screen.getByText('1 nodes')).toBeInTheDocument();
  });

  it('shows name, kind badge, and trigger info for webhook', () => {
    const script: UserScript = {
      id: '1',
      name: 'Webhook flow',
      kind: 'action_command',
      source: '',
      device_id: 'dev1',
      enabled: false,
      ir_json: {
        kind: 'action_command',
        nodes: [{ id: 'trigger', type: 'trigger', data: { kind: 'webhook' } }, { id: '2', type: 'action', data: {} }],
        edges: [],
        next_flow_ids: [],
      } as any,
    } as any;

    render(
      <ReactFlowProvider>
        <FlowSummaryNode data={{ script }} />
      </ReactFlowProvider>
    );
    expect(screen.getByText('Webhook flow')).toBeInTheDocument();
    expect(screen.getByText('Hành động')).toBeInTheDocument();
    expect(screen.getByText('WEBHOOK')).toBeInTheDocument();
    expect(screen.getByText('2 nodes')).toBeInTheDocument();
    expect(screen.getByText('Đã tắt')).toBeInTheDocument();
  });
});
