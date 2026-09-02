import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, it, expect, vi } from 'vitest';
import MainLayout from './MainLayout';

vi.mock('../../hooks/useDeviceSync', () => ({ useDeviceSync: () => {} }));
vi.mock('../../store/useDeviceStore', () => ({
  useDeviceStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      isSensorOnline: true,
      isMissingConfig: false,
      systemEvents: [],
      deviceId: 'test-device-123',
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
});
