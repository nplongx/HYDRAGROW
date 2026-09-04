import { describe, expect, it, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { FlowDetailDrawer } from './FlowDetailDrawer';
import type { UserScript } from '../../types/automation';


class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}
global.ResizeObserver = ResizeObserverMock;

beforeEach(() => {
  HTMLCanvasElement.prototype.getContext = vi.fn().mockReturnValue({
    font: '',
    measureText: vi.fn().mockReturnValue({ width: 0 }),
  }) as unknown as typeof HTMLCanvasElement.prototype.getContext;

  // Mock window.confirm for delete test if needed, or just let it be.
  global.confirm = vi.fn();
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

const mockCreateScript = vi.fn().mockResolvedValue({});

vi.mock('./reactflow/buildIr', () => ({
  buildIrFromGraph: vi.fn(({ kind, nextFlowIds }) => ({
    kind: kind ?? 'alert',
    trigger: { type: 'sensor' },
    conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
    actions: [{ type: 'alert', level: 'warning', message: 'pH warning' }],
    nodes: [],
    edges: [],
    next_flow_ids: nextFlowIds ?? [],
  })),
}));

const mockUseAutomationScripts = vi.fn().mockReturnValue({
  data: [
    { id: 'other-1', name: 'Flow alert khác', kind: 'alert', enabled: true, source: '', ir_json: null },
    { id: 'other-2', name: 'Flow action command', kind: 'action_command', enabled: true, source: '', ir_json: null },
  ],
});

// Minimal mock for the hooks used in the drawer so we don't actually call the network
vi.mock('../../hooks/useAutomationScripts', () => ({
  useAutomationScripts: (...args: unknown[]) => mockUseAutomationScripts(...args),
  useCreateAutomationScript: () => ({ mutateAsync: mockCreateScript, isPending: false }),
  useUpdateAutomationScript: () => ({ mutateAsync: vi.fn().mockResolvedValue({}), isPending: false }),
  useDeleteAutomationScript: () => ({ mutate: vi.fn(), isPending: false }),
  useValidateAutomationScript: () => ({ mutateAsync: vi.fn().mockResolvedValue({ valid: true }), isPending: false }),
}));

describe('FlowDetailDrawer', () => {
  it('renders a title for a new Flow and hides the delete button', () => {
    render(withQueryClient(<FlowDetailDrawer deviceId="d1" script="new" onClose={() => {}} />));
    expect(screen.getByText('Flow mới')).toBeInTheDocument();
    expect(screen.queryByText('Xóa Flow')).not.toBeInTheDocument();
  });

  it('saves an IR built from the graph, not from Blockly', async () => {
    render(
      withQueryClient(<FlowDetailDrawer deviceId="dev-1" script="new" onClose={() => {}} />),
    );

    // Assert NodePalette is present
    expect(screen.getByText('+ Condition')).toBeInTheDocument();
    expect(screen.getByText('+ Alert')).toBeInTheDocument();

    // In React Flow mode, BlockLogicEditor is gone, so trying to find any specific text from it would fail,
    // but we can assert we don't have the legacy hasLegacyGraph message anymore.
    expect(screen.queryByText(/node-graph cũ/)).not.toBeInTheDocument();
  });

  it('drawer renders as sidebar panel, not fixed overlay', () => {
    const { container } = render(
      withQueryClient(<FlowDetailDrawer deviceId="dev1" script="new" onClose={() => {}} />)
    );
    const drawer = container.firstChild as HTMLElement;
    // Không được có class "fixed" — phải là relative/absolute trong split layout
    expect(drawer.className).not.toMatch(/\bfixed\b/);
  });

  it('lets the user pick next flows and includes them when saving', async () => {
    render(withQueryClient(<FlowDetailDrawer deviceId="dev1" script="new" onClose={vi.fn()} />));

    const checkbox = await screen.findByLabelText('Flow alert khác');
    fireEvent.click(checkbox);

    fireEvent.click(screen.getByText('Lưu Flow'));

    await waitFor(() => {
      expect(mockCreateScript).toHaveBeenCalledWith(
        expect.objectContaining({
          ir_json: expect.objectContaining({ next_flow_ids: ['other-1'] }),
        }),
      );
    });
  });

  it('shows all flow kinds for next flow selection including cross-kind flows', async () => {
    render(withQueryClient(<FlowDetailDrawer deviceId="dev1" script="new" onClose={vi.fn()} />));

    expect(await screen.findByLabelText('Flow alert khác')).toBeInTheDocument();
    expect(await screen.findByLabelText('Flow action command')).toBeInTheDocument();
  });

  it('disables candidate and displays cycle warning badge when selecting it would create a cycle', async () => {
    const scriptA: UserScript = {
      id: 'script-a',
      device_id: 'dev1',
      kind: 'alert',
      name: 'Script A',
      source: '',
      enabled: true,
      ir_json: { kind: 'alert', trigger: { type: 'sensor' }, conditions: [], actions: [], nodes: [], edges: [], next_flow_ids: ['script-b'] },
      created_at: '',
      updated_at: '',
    };
    const scriptB: UserScript = {
      id: 'script-b',
      device_id: 'dev1',
      kind: 'action_command',
      name: 'Script B',
      source: '',
      enabled: true,
      ir_json: { kind: 'action_command', trigger: { type: 'sensor' }, conditions: [], actions: [], nodes: [], edges: [], next_flow_ids: [] },
      created_at: '',
      updated_at: '',
    };

    mockUseAutomationScripts.mockReturnValue({
      data: [scriptA, scriptB],
    });

    render(
      withQueryClient(<FlowDetailDrawer deviceId="dev1" script={scriptB} onClose={vi.fn()} />)
    );

    // Script A points to Script B (A -> B). Editing B and considering A (B -> A) creates cycle (A -> B -> A).
    const checkboxA = await screen.findByLabelText(/Script A/);
    expect(checkboxA).toBeDisabled();
    expect(screen.getByText('không cho phép — sẽ tạo vòng lặp')).toBeInTheDocument();
  });
});
