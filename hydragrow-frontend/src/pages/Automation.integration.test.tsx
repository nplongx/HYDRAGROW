import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { Automation } from './Automation';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter } from 'react-router-dom';
import { useDeviceStore } from '../store/useDeviceStore';

const queryClient = new QueryClient();

// Mock useAutomationScripts
vi.mock('../hooks/useAutomationScripts', () => ({
  useAutomationScripts: () => ({
    data: [
      { id: '1', name: 'Saved Alert Node', kind: 'alert', enabled: true, device_id: 'dev1', source: '', ir_json: { kind: 'alert', nodes: [], edges: [], next_flow_ids: [] } },
    ],
    isLoading: false,
    isError: false
  }),
  useCreateAutomationScript: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useUpdateAutomationScript: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useDeleteAutomationScript: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useValidateAutomationScript: () => ({ mutateAsync: vi.fn().mockResolvedValue({ valid: true }), isPending: false }),
  useTestAutomationScript: () => ({ mutateAsync: vi.fn(), isPending: false, data: null }),
  useApplyTemplate: () => ({ mutate: vi.fn(), isPending: false, isSuccess: false, isError: false }),
}));

class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}

global.ResizeObserver = ResizeObserverMock as any;

describe('Automation Integration', () => {
  it('covers the complete navigation path', async () => {
    useDeviceStore.setState({ deviceId: 'dev1' });
    // We mock media query to ensure desktop view
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: vi.fn().mockImplementation(query => ({
        matches: true, // Desktop view
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });

    render(
      <QueryClientProvider client={queryClient}>
        <BrowserRouter>
          <Automation />
        </BrowserRouter>
      </QueryClientProvider>
    );

    // -> overview
    expect(screen.getByText('Tự động hóa')).toBeInTheDocument();

    // -> open Flow (create new)
    const newFlowBtn = screen.getByRole('button', { name: /Flow mới/i });
    fireEvent.click(newFlowBtn);

    // Check drawer opened
    expect(screen.getByRole('heading', { name: 'Flow mới' })).toBeInTheDocument();

    // -> select trigger
    const sensorBtn = screen.getByRole('button', { name: /Sensor/i });
    fireEvent.click(sensorBtn);

    // Wait for the UI to update to Sensor
    await waitFor(() => {
      // Look for condition add button which proves editor is open
      const addConditionBtn = screen.getByRole('button', { name: '+ Condition' });
      fireEvent.click(addConditionBtn);
    });

    // -> run dry-run
    const testBtn = screen.getByRole('button', { name: /Chạy thử/i });
    fireEvent.click(testBtn);

    // Check panel opened
    expect(screen.getByText('Chạy thử (Dry Run)')).toBeInTheDocument();

    // -> save
    const saveBtn = screen.getByRole('button', { name: /Lưu Flow/i });
    fireEvent.click(saveBtn);
  }, 15000);
});
