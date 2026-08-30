import { describe, expect, it, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { buildAutomationIr, FlowDetailDrawer } from './FlowDetailDrawer';

beforeEach(() => {
  HTMLCanvasElement.prototype.getContext = vi.fn().mockReturnValue({
    font: '',
    measureText: vi.fn().mockReturnValue({ width: 0 }),
  }) as unknown as typeof HTMLCanvasElement.prototype.getContext;
});

function withQueryClient(children: React.ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe('buildAutomationIr (pure)', () => {
  it('builds a sensor-triggered IR for kind=alert', () => {
    const ir = buildAutomationIr('alert', {
      conditions: [{ sensor: 'ph', operator: '>', value: 7.5 }],
      actions: [{ type: 'alert', level: 'warning', message: 'pH cao' }],
    });
    expect(ir.trigger).toEqual({ type: 'sensor' });
    expect(ir.nodes).toEqual([]);
    expect(ir.edges).toEqual([]);
  });

  it('builds an fsm-triggered IR for kind=recipe_override', () => {
    const ir = buildAutomationIr('recipe_override', {
      conditions: [{ sensor: 'elapsed_sec', operator: '>', value: 86400 }],
      actions: [{ type: 'advance_stage', targetStageOffset: 1, reason: 'Đủ 24h' }],
    });
    expect(ir.trigger).toEqual({ type: 'fsm' });
  });

  it('builds a sensor-triggered IR for kind=action_command', () => {
    const ir = buildAutomationIr('action_command', {
      conditions: [{ sensor: 'ph', operator: '>', value: 8.0 }],
      actions: [{ type: 'dose', pump: 'PH_DOWN', doseMl: 3, pwm: 80 }],
    });
    expect(ir.trigger).toEqual({ type: 'sensor' });
  });
});

describe('FlowDetailDrawer', () => {
  it('renders a title for a new Flow and hides the delete button', () => {
    render(withQueryClient(<FlowDetailDrawer deviceId="d1" script="new" onClose={() => {}} />));
    expect(screen.getByText('Flow mới')).toBeInTheDocument();
    expect(screen.queryByText('Xóa Flow')).not.toBeInTheDocument();
  });

  it('renders the existing name and a legacy-graph notice when ir_json has nodes', () => {
    render(
      withQueryClient(
        <FlowDetailDrawer
          deviceId="d1"
          script={{
            id: 's1',
            device_id: 'd1',
            kind: 'alert',
            name: 'Flow cũ',
            source: '',
            enabled: true,
            ir_json: {
              kind: 'alert',
              trigger: { type: 'sensor' },
              conditions: [{ sensor: 'ph', operator: '>', value: 7.5 }],
              actions: [{ type: 'alert', level: 'warning', message: 'x' }],
              nodes: [{ id: '1', type: 'sensor', position: { x: 0, y: 0 }, data: {} }],
              edges: [],
              next_flow_ids: [],
            },
            created_at: '',
            updated_at: '',
          }}
          onClose={() => {}}
        />,
      ),
    );
    expect(screen.getByDisplayValue('Flow cũ')).toBeInTheDocument();
    expect(screen.getByText(/node-graph cũ/)).toBeInTheDocument();
    expect(screen.getByText('Xóa Flow')).toBeInTheDocument();
  });

  it('drawer renders as sidebar panel, not fixed overlay', () => {
    const { container } = render(
      withQueryClient(<FlowDetailDrawer deviceId="dev1" script="new" onClose={() => {}} />)
    );
    const drawer = container.firstChild as HTMLElement;
    // Không được có class "fixed" — phải là relative/absolute trong split layout
    expect(drawer.className).not.toMatch(/\bfixed\b/);
  });
});
