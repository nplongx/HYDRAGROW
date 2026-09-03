import { describe, it, expect, vi } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import Dashboard from './Dashboard';
import { useDeviceStore } from '../store/useDeviceStore';

vi.mock('../hooks/useFCM', () => ({
  useFCM: () => ({ permission: 'granted', enableNotifications: vi.fn() }),
}));

vi.mock('../hooks/useSystemHealthSummary', () => ({
  useSystemHealthSummary: () => ({
    data: { ec_dosing_count: 2, ph_dosing_count: 1, latest_ph_dosing_at: 1700000000000 },
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
    useDeviceStore.setState({
      deviceId: 'dev-001',
      isLoading: false,
      isSensorOnline: true,
      deviceStatus: { is_online: true, last_seen: '2026-09-02T00:00:00Z' },
      sensorData: {
        device_id: 'dev-001',
        time: '2026-09-02T00:00:00Z',
        ec: 1.2,
        ph: 6.0,
        temp: 25.0,
        water_level: 80,
        pump_status: {
          pump_a: false,
          pump_b: false,
          ph_up: false,
          ph_down: false,
          osaka_pump: false,
          mist_valve: false,
          mix_valve: false,
          water_pump_in: false,
          water_pump_out: false,
        },
      },
      fsmState: 'Monitoring',
      settings: { backend_url: 'http://localhost:1420', api_key: 'test', device_id: 'dev-001' },
    });

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
