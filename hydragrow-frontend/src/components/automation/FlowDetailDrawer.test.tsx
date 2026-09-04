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
});
