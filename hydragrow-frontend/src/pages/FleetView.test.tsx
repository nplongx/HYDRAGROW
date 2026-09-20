import { fireEvent, render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { FleetView } from './FleetView';

vi.mock('../hooks/useDashboardFleet', () => ({
  useDashboardFleet: () => ({ stations: [], isLoading: false, isFetching: false, error: null, refresh: vi.fn() }),
  useFleetComparison: () => ({ data: null, refetch: vi.fn() }),
}));
vi.mock('../contexts/StationContext', () => ({
  useStationContext: () => ({ selectDevice: vi.fn(), availableDevices: [] }),
}));
const renderFleet = (initialEntries: string[], initialIndex?: number) => render(
  <QueryClientProvider client={new QueryClient()}>
    <MemoryRouter initialEntries={initialEntries} initialIndex={initialIndex}>
      <Routes>
        <Route path="/fleet" element={<FleetView />} />
        <Route path="/dashboard" element={<div>Dashboard</div>} />
      </Routes>
    </MemoryRouter>
  </QueryClientProvider>
);

describe('FleetView navigation', () => {
  it('falls back to dashboard on direct entry with no in-app history', () => {
    renderFleet(['/fleet']);

    fireEvent.click(screen.getByRole('button', { name: 'Quay lại' }));
    expect(screen.getByText('Dashboard')).toBeInTheDocument();
  });

  it('uses in-app history when Fleet was reached from another route', () => {
    renderFleet(['/dashboard', '/fleet'], 1);

    fireEvent.click(screen.getByRole('button', { name: 'Quay lại' }));
    expect(screen.getByText('Dashboard')).toBeInTheDocument();
  });
});
