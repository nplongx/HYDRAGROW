import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { FleetView } from './FleetView';
import { ConfigBackup } from './ConfigBackup';
import { UserManagement } from './UserManagement';
import { resolveRoute } from '../routes';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn().mockImplementation((path: string) =>
    path === '/admin/scopes' ? Promise.resolve([]) : Promise.resolve({ data: [] }),
  ),
  apiPost: vi.fn().mockResolvedValue({ data: {} }),
}));

vi.mock('../hooks/useFleetStatus', () => ({
  useFleetStatus: () => ({
    devices: [],
    loading: false,
    error: null,
    refresh: vi.fn(),
  }),
}));

vi.mock('../contexts/StationContext', () => ({
  useStationContext: () => ({
    status: 'NoSelection', selectedDeviceId: null, selectedDevice: null,
    availableDevices: [], error: null, selectDevice: vi.fn(), switchDevice: vi.fn(),
    clearSelection: vi.fn(), refreshAvailableDevices: vi.fn(),
  }),
}));

function renderPage(page: React.ReactNode) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>{page}</MemoryRouter>
    </QueryClientProvider>,
  );
}

describe('Utility route owners mount', () => {
  it('FleetView render tiêu đề Tổng Quan Thiết Bị', () => {
    renderPage(<FleetView />);
    expect(screen.getByText('Tổng Quan Thiết Bị')).toBeInTheDocument();
    expect(screen.getAllByText(/Liên kết thiết bị mới/).length).toBeGreaterThan(0);
  });

  it('ConfigBackup render tiêu đề Backup & Restore Cấu Hình', () => {
    renderPage(<ConfigBackup />);
    expect(screen.getByText('Backup & Restore Cấu Hình')).toBeInTheDocument();
  });

  it('maps /user-management to UserManagement and renders its page', () => {
    expect(resolveRoute('/user-management')?.id).toBe('user-management');
    renderPage(<UserManagement />);
    expect(screen.getByText('Quản Lý Người Dùng & Quyền')).toBeInTheDocument();
  });
});
