import { describe, it, expect, vi } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import Dashboard from './Dashboard';

vi.mock('../hooks/useFCM', () => ({
  useFCM: () => ({ permission: 'granted', enableNotifications: vi.fn() }),
}));

vi.mock('../hooks/useSystemHealthSummary', () => ({
  useSystemHealthSummary: () => ({
    data: { ec_dosing_count: 2, ph_dosing_count: 1, latest_ph_dosing_at: 1700000000000 },
  }),
}));

vi.mock('../hooks/useDeviceTelemetry', () => ({
  useDeviceTelemetry: () => ({
    data: {
      device_id: 'dev-001',
      availability: 'ONLINE',
      controller_health: null,
      fsm: { state: 'Monitoring' },
      axes: [
        { name: 'ec', value: 1.2, quality: 'VALID' },
        { name: 'ph', value: 6, quality: 'VALID' },
        { name: 'temp', value: 25, quality: 'VALID' },
        { name: 'water_level', value: 80, quality: 'VALID' },
      ],
      actuator: { pump_status: {} },
    },
    isLoading: false,
    error: null,
  }),
}));

vi.mock('../hooks/useDeviceConfig', () => ({
  useDeviceConfig: () => ({
    data: {
      control_mode: 'auto',
      min_ec_limit: 0.5,
      max_ec_limit: 3,
      min_ph_limit: 4,
      max_ph_limit: 8,
      min_temp_limit: 15,
      max_temp_limit: 35,
      water_level_min: 20,
      water_level_max: 90,
    },
  }),
}));

vi.mock('../hooks/useSystemEvents', () => ({
  useSystemEvents: () => ({ data: [] }),
}));

vi.mock('../hooks/useDashboardFleet', () => ({
  useDashboardFleet: () => ({
    stations: [
      {
        device_id: 'dev-001',
        label: 'Trạm Alpha',
        is_online: true,
        last_seen: null,
        operational_state: { contact: 'CONTACTED', freshness: 'FRESH' },
        crop: 'Rau xà lách',
        ec_latest: 1.2,
        ph_latest: 6,
        warning_count: 0,
      },
      {
        device_id: 'dev-002',
        label: 'Trạm Beta',
        is_online: false,
        last_seen: '2026-09-19T10:00:00Z',
        operational_state: { contact: 'NOT_CONTACTED', freshness: 'STALE' },
        crop: null,
        ec_latest: null,
        ph_latest: null,
        warning_count: 2,
      },
    ],
    isLoading: false,
    isFetching: false,
    error: null,
    refresh: vi.fn(),
  }),
}));

vi.mock('../hooks/useDeviceControl', () => ({
  useDeviceControl: () => ({ forceOn: vi.fn(), commandStatus: {}, commandIds: {} }),
}));

vi.mock('../contexts/AuthContext', () => ({
  useAuth: () => ({
    status: 'authenticated',
    user: { displayName: 'Nam', email: 'nam@hydragrow.dev' },
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
  }),
}));

let stationSelected = false;

vi.mock('../contexts/StationContext', () => ({
  useStationContext: () => ({
    status: stationSelected ? 'Selected' : 'NoSelection', selectedDeviceId: stationSelected ? 'dev-001' : null, selectedDevice: null,
    availableDevices: [], error: null, selectDevice: vi.fn(), switchDevice: vi.fn(),
    clearSelection: vi.fn(), refreshAvailableDevices: vi.fn(),
  }),
}));

describe('Dashboard pumpColors token', () => {
  it('không dùng bg-indigo-* / text-indigo-* (vi phạm CHUAN-GIAO-DIEN mục 1.1)', () => {
    const filePath = path.resolve(process.cwd(), 'src/pages/Dashboard.tsx');
    const src = fs.readFileSync(filePath, 'utf-8');
    expect(src).not.toMatch(/indigo/);
  });
});

describe('Dashboard component wiring', () => {
  it('hiển thị All Stations Overview với summary, filter và station cards', () => {
    stationSelected = false;
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter initialEntries={['/dashboard?station=']}>
          <Dashboard />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    expect(screen.getByRole('heading', { name: 'Tổng quan', level: 1 })).toBeInTheDocument();
    expect(screen.getByText('Tổng số trạm')).toBeInTheDocument();
    expect(screen.getByText('Đang online')).toBeInTheDocument();
    expect(screen.getByText('Cảnh báo')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Mở trạm Trạm Alpha/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Mở trạm Trạm Beta/i })).toBeInTheDocument();
    expect(screen.getByText('2 cảnh báo')).toBeInTheDocument();
    const cards = screen.getByTestId('dashboard-station-grid').querySelectorAll('button');
    expect(cards[0]).toHaveAccessibleName('Mở trạm Trạm Beta');
    expect(cards[1]).toHaveAccessibleName('Mở trạm Trạm Alpha');
    expect(screen.queryByText(/độ ẩm|CO2|áp suất|lưu lượng/i)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Cần chú ý/i }));
    expect(screen.queryByRole('button', { name: /Mở trạm Trạm Alpha/i })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Mở trạm Trạm Beta/i })).toBeInTheDocument();
  });

  it('hiển thị Selected Station Detail khi StationContext có trạm được chọn', () => {
    stationSelected = true;
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter initialEntries={['/dashboard']}>
          <Dashboard />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    expect(screen.getByText('ID: dev-001')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /tất cả trạm/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Thông số thời gian thực' })).toBeInTheDocument();
  });
});
