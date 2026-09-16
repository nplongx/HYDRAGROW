import { describe, it, expect, vi } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
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

vi.mock('../hooks/useDeviceControl', () => ({
  useDeviceControl: () => ({ forceOn: vi.fn() }),
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

vi.mock('../contexts/StationContext', () => ({
  useStationContext: () => ({
    status: 'Selected', selectedDeviceId: 'dev-001', selectedDevice: null,
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
  it('hiển thị QuickActionBar và DosingSummaryCard', () => {
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>
    );

    expect(screen.getByText('Thao tác nhanh')).toBeInTheDocument();
    expect(screen.getByText('Châm dinh dưỡng')).toBeInTheDocument();
    expect(screen.getByText('Tạm dừng bơm')).toBeInTheDocument();
    expect(screen.getByText('Xem cảnh báo')).toBeInTheDocument();
    expect(screen.getByText('Châm dinh dưỡng hôm nay')).toBeInTheDocument();
    expect(screen.getByText(/3 lần/)).toBeInTheDocument();
  });
});
