import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import Analytics, { formatUptime, formatHeap, formatRssi } from './Analytics';
import * as apiClient from '../lib/apiClient';
import * as settingsPlatform from '../platform/settings';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn(),
  apiPut: vi.fn(),
}));

vi.mock('../platform/settings', () => ({
  loadAppSettings: vi.fn(),
}));

vi.mock('../store/useDeviceStore', () => ({
  useDeviceStore: vi.fn((selector) =>
    selector({
      deviceId: 'hydra-001',
      sensorData: null,
      isSensorOnline: true,
      settings: null,
    })
  ),
}));

vi.mock('../hooks/useWhoami', () => ({
  useWhoami: vi.fn(() => ({
    data: {
      id: 1,
      firebase_uid: 'uid-1',
      email: 'test@farm.vn',
      role: 'admin',
      preferences: { weekly_report: false },
    },
    refetch: vi.fn(),
  })),
}));

const mockHealth = {
  free_heap_bytes: 184320, // 180 KB
  wifi_rssi_dbm: -64.0,
  uptime_seconds: 7200, // 2h 0m
  backend_process_cpu_percent: 1.25,
  last_updated_at: '2026-09-10T10:00:00Z',
};

describe('Analytics Page', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    vi.clearAllMocks();
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    vi.mocked(apiClient.apiGet).mockResolvedValue(mockHealth);
    vi.mocked(settingsPlatform.loadAppSettings).mockResolvedValue({
      api_key: 'test-key',
      backend_url: 'http://localhost:8080',
      device_id: 'hydra-001',
      grafana_url: '',
    });
  });

  const renderWithProviders = () =>
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <Analytics />
        </MemoryRouter>
      </QueryClientProvider>
    );

  describe('Formatters', () => {
    it('formatUptime định dạng chính xác thời gian hoạt động', () => {
      expect(formatUptime(null)).toBe('—');
      expect(formatUptime(undefined)).toBe('—');
      expect(formatUptime(0)).toBe('—');
      expect(formatUptime(120)).toBe('2m');
      expect(formatUptime(3600)).toBe('1h 0m');
      expect(formatUptime(90000)).toBe('1d 1h');
    });

    it('formatHeap định dạng byte thành KB', () => {
      expect(formatHeap(null)).toBe('—');
      expect(formatHeap(102400)).toBe('100.0 KB');
    });

    it('formatRssi định dạng dBm', () => {
      expect(formatRssi(null)).toBe('—');
      expect(formatRssi(-72.4)).toBe('-72 dBm');
    });
  });

  it('hiển thị 4 thẻ thông số phần cứng (RAM, WiFi, Uptime, CPU)', async () => {
    renderWithProviders();

    await waitFor(() => {
      expect(screen.getByTestId('health-free-heap')).toHaveTextContent('180.0 KB');
      expect(screen.getByTestId('health-wifi-rssi')).toHaveTextContent('-64 dBm');
      expect(screen.getByTestId('health-uptime')).toHaveTextContent('2h 0m');
      expect(screen.getByTestId('health-cpu')).toHaveTextContent('1.3%');
    });
  });

  it('hiển thị placeholder khi chưa cấu hình Grafana URL', async () => {
    renderWithProviders();

    await waitFor(() => {
      expect(screen.getByTestId('grafana-placeholder')).toBeInTheDocument();
      expect(screen.getByText(/Chưa thiết lập URL Grafana Dashboard/i)).toBeInTheDocument();
    });
  });

  it('hiển thị iframe Grafana khi đã có grafana_url', async () => {
    vi.mocked(settingsPlatform.loadAppSettings).mockResolvedValue({
      api_key: 'test-key',
      backend_url: 'http://localhost:8080',
      device_id: 'hydra-001',
      grafana_url: 'http://grafana.farm.vn:3000/d/hydragrow',
    });

    renderWithProviders();

    await waitFor(() => {
      const iframe = screen.getByTestId('grafana-iframe');
      expect(iframe).toBeInTheDocument();
      expect(iframe).toHaveAttribute('src', 'http://grafana.farm.vn:3000/d/hydragrow');
    });
  });

  it('bật toggle báo cáo tuần qua email gọi apiPut lưu preferences', async () => {
    vi.mocked(apiClient.apiPut).mockResolvedValue({ status: 'success' });
    renderWithProviders();

    const checkbox = screen.getByRole('checkbox', { name: 'Báo cáo tuần qua email' });
    expect(checkbox).not.toBeChecked();

    fireEvent.click(checkbox);

    await waitFor(() => {
      expect(apiClient.apiPut).toHaveBeenCalledWith(
        '/admin/me/preferences',
        expect.objectContaining({
          preferences: expect.objectContaining({ weekly_report: true }),
        })
      );
    });
  });
});
