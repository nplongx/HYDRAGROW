import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { FleetView } from './FleetView';

vi.mock('../hooks/useFleetStatus', () => ({
  useFleetStatus: () => ({ devices: [], loading: false, error: null, refresh: vi.fn() }),
}));
vi.mock('../contexts/StationContext', () => ({
  useStationContext: () => ({ selectDevice: vi.fn() }),
}));
vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn().mockResolvedValue({ data: [] }),
}));

describe('FleetView navigation', () => {
  it('falls back to dashboard on direct entry with no in-app history', () => {
    render(
      <MemoryRouter initialEntries={['/fleet']}>
        <Routes>
          <Route path="/fleet" element={<FleetView />} />
          <Route path="/dashboard" element={<div>Dashboard</div>} />
        </Routes>
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Quay lại' }));
    expect(screen.getByText('Dashboard')).toBeInTheDocument();
  });

  it('uses in-app history when Fleet was reached from another route', () => {
    render(
      <MemoryRouter initialEntries={['/dashboard', '/fleet']} initialIndex={1}>
        <Routes>
          <Route path="/fleet" element={<FleetView />} />
          <Route path="/dashboard" element={<div>Dashboard</div>} />
        </Routes>
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Quay lại' }));
    expect(screen.getByText('Dashboard')).toBeInTheDocument();
  });
});
