import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { useDeviceStore } from '../store/useDeviceStore';
import { FleetView } from './FleetView';
import { ConfigBackup } from './ConfigBackup';
import { UserManagement } from './UserManagement';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn().mockResolvedValue({ data: {} }),
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

describe('Orphaned pages mount via App routes', () => {
  it('FleetView render tiêu đề Tổng Quan Thiết Bị', () => {
    useDeviceStore.setState({ deviceId: null, isLoading: false });
    render(
      <MemoryRouter>
        <FleetView />
      </MemoryRouter>,
    );
    expect(screen.getByText('Tổng Quan Thiết Bị')).toBeInTheDocument();
    expect(screen.getAllByText(/Liên kết thiết bị mới/).length).toBeGreaterThan(0);
  });

  it('ConfigBackup render tiêu đề Backup & Restore Cấu Hình', () => {
    render(
      <MemoryRouter>
        <ConfigBackup />
      </MemoryRouter>,
    );
    expect(screen.getByText('Backup & Restore Cấu Hình')).toBeInTheDocument();
  });

  it('UserManagement render tiêu đề Quản Lý Người Dùng & Quyền', () => {
    render(
      <MemoryRouter>
        <UserManagement />
      </MemoryRouter>,
    );
    expect(screen.getByText('Quản Lý Người Dùng & Quyền')).toBeInTheDocument();
  });
});