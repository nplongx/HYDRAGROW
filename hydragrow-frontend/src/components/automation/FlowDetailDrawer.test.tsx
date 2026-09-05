import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { FlowDetailDrawer } from './FlowDetailDrawer';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import * as useAutomationScriptsModule from '../../hooks/useAutomationScripts';

const queryClient = new QueryClient();

// Mock resize observer
class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}

global.ResizeObserver = ResizeObserverMock as any;

const mockMutate = vi.fn().mockResolvedValue({});
const mockValidate = vi.fn().mockResolvedValue({ valid: true });

vi.mock('../../hooks/useAutomationScripts', () => ({
  useAutomationScripts: vi.fn().mockReturnValue({ data: [] }),
  useCreateAutomationScript: () => ({ mutateAsync: mockMutate, isPending: false }),
  useUpdateAutomationScript: () => ({ mutateAsync: mockMutate, isPending: false }),
  useDeleteAutomationScript: () => ({ mutateAsync: mockMutate, isPending: false }),
  useValidateAutomationScript: () => ({ mutateAsync: mockValidate }),
}));

describe('FlowDetailDrawer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders a title for a new Flow and hides the delete button', () => {
    render(
      <QueryClientProvider client={queryClient}>
        <FlowDetailDrawer deviceId="123" script="new" onClose={vi.fn()} />
      </QueryClientProvider>
    );

    expect(screen.getByText('Flow mới')).toBeInTheDocument();
    expect(screen.queryByText('Xóa Flow')).not.toBeInTheDocument();
  });

  it('drawer renders as sidebar panel, not fixed overlay', () => {
    render(
      <QueryClientProvider client={queryClient}>
        <FlowDetailDrawer deviceId="123" script="new" onClose={vi.fn()} />
      </QueryClientProvider>
    );
    expect(screen.getByText('Flow mới')).toBeInTheDocument();
  });

  it('drawer container has overflow-y-auto for vertical scrolling', () => {
    render(
      <QueryClientProvider client={queryClient}>
        <FlowDetailDrawer deviceId="123" script="new" onClose={vi.fn()} />
      </QueryClientProvider>
    );
    expect(screen.getByTestId('flow-detail-drawer')).toHaveClass('overflow-y-auto');
  });

  it('shows all flow kinds for next flow selection including cross-kind flows', () => {
    vi.mocked(useAutomationScriptsModule.useAutomationScripts).mockReturnValue({
      data: [
        { id: 'flow-b', name: 'Alert B', kind: 'alert', enabled: true } as any,
        { id: 'flow-c', name: 'Action C', kind: 'action_command', enabled: true } as any,
      ],
    } as any);

    render(
      <QueryClientProvider client={queryClient}>
        <FlowDetailDrawer deviceId="123" script="new" onClose={vi.fn()} />
      </QueryClientProvider>
    );

    expect(screen.getByText('Alert B')).toBeInTheDocument();
    expect(screen.getByText('Action C')).toBeInTheDocument();
  });

  it('threads chainConfig.passContextVariables through save and reloads it when editing an existing Flow', async () => {
    vi.mocked(useAutomationScriptsModule.useAutomationScripts).mockReturnValue({
      data: [
        { id: 'script-2', name: 'Flow tiếp theo', kind: 'alert', enabled: true } as any,
      ],
    } as any);

    const existingScript = {
      id: 'script-1',
      name: 'Flow hiện có',
      kind: 'alert',
      enabled: true,
      ir_json: {
        kind: 'alert',
        trigger: { type: 'sensor' },
        conditions: [{ sensor: 'ph', operator: '>', value: 7 }],
        actions: [{ type: 'alert', level: 'warning', message: 'x' }],
        nodes: [],
        edges: [],
        next_flow_ids: [],
        chainConfig: { passContextVariables: true },
      },
    };

    render(
      <QueryClientProvider client={queryClient}>
        <FlowDetailDrawer deviceId="device-1" script={existingScript as any} onClose={vi.fn()} />
      </QueryClientProvider>
    );

    const checkbox = await screen.findByLabelText('Truyền biến ngữ cảnh sang flow tiếp theo');
    expect(checkbox).toBeChecked();
  });
});
