import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, it, expect, vi } from 'vitest';
import MainLayout from './MainLayout';

vi.mock('../../hooks/useDeviceSync', () => ({ useDeviceSync: () => {} }));
vi.mock('../../hooks/useDeviceTelemetry', () => ({
  useDeviceTelemetry: () => ({ data: { availability: 'ONLINE' } }),
}));
vi.mock('../../hooks/useDeviceConfig', () => ({
  useDeviceConfig: () => ({ isLoading: false, error: null }),
}));
vi.mock('../../hooks/useSystemEvents', () => ({
  useSystemEvents: () => ({ data: [] }),
}));

vi.mock('../../contexts/StationContext', () => ({
  useStationContext: () => ({
    status: 'Selected', selectedDeviceId: 'test-device-123', selectedDevice: null,
    availableDevices: [], error: null, selectDevice: vi.fn(), switchDevice: vi.fn(),
    clearSelection: vi.fn(), refreshAvailableDevices: vi.fn(),
  }),
}));

describe('MainLayout sidebar', () => {
  it('đánh dấu mục Tổng quan là active khi ở /dashboard', () => {
    render(
      <MemoryRouter initialEntries={['/dashboard']}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/dashboard" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    );
    const activeItem = screen.getAllByRole('button', { name: /Tổng quan/i })[0];
    expect(activeItem.className).toContain('bg-emerald-50');
  });

  it('hiển thị brand HydraGrow và thông tin thiết bị trong sidebar', () => {
    render(
      <MemoryRouter initialEntries={['/dashboard']}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/dashboard" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    );
    expect(screen.getAllByText('HydraGrow').length).toBeGreaterThan(0);
    expect(screen.getByText('Trạm Online')).toBeInTheDocument();
    expect(screen.getByText('ID: test-device-123')).toBeInTheDocument();
  });

  it('header mobile hiển thị pill trạng thái kết nối (bg-pill/text-status) theo chuẩn Figma', () => {
    render(
      <MemoryRouter initialEntries={['/dashboard']}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/dashboard" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    );
    const pill = screen.getByText('Đang kết nối');
    expect(pill.className).toContain('bg-pill');
    expect(pill.className).toContain('text-status');
    expect(pill.className).toContain('rounded-full');
  });

  it('bottom nav mobile là dải pill nổi (bg-line/80) với 5 tab dạng chấm + nhãn', () => {
    render(
      <MemoryRouter initialEntries={['/dashboard']}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/dashboard" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    );
    const pill = document.querySelector('nav [class*="bg-line"]');
    expect(pill).toBeDefined();
    expect(pill!.className).toContain('bg-line/80');
    expect(pill!.className).toContain('rounded-full');
    ['Tổng quan', 'Vận hành', 'Canh tác', 'Nhật ký', 'Cài đặt'].forEach((label) => {
      expect(screen.getAllByText(label).length).toBeGreaterThanOrEqual(1);
    });
  });

  it('đánh dấu route con của trang chính là active theo router', () => {
    render(
      <MemoryRouter initialEntries={['/settings/integration']}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/settings/*" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getAllByRole('button', { name: /Cài đặt/i })[0].className).toContain('bg-emerald-50');
  });

  it('desktop và mobile dùng cùng route target cho mục chính', () => {
    render(
      <MemoryRouter initialEntries={['/dashboard']}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="*" element={<div>content</div>} />
          </Route>
          <Route path="/operations" element={<div>operations</div>} />
        </Routes>
      </MemoryRouter>
    );

    const operations = screen.getAllByRole('button', { name: /Vận hành/i });
    fireEvent.click(operations[0]);
    expect(screen.getByText('operations')).toBeInTheDocument();
  });
});
