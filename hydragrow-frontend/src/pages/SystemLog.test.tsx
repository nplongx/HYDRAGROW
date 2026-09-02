import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import SystemLog from './SystemLog';
import { useDeviceStore } from '../store/useDeviceStore';
import type { AppSettings } from '../types/models';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn((path: string) => {
    if (path.includes('health-summary')) {
      return Promise.resolve({
        status: 'success',
        data: { window_seconds: 3600, ec_dosing_count: 1, ph_dosing_count: 0, water_operation_count: 0, warning_count: 0, critical_count: 0, latest_ph_dosing_at: null },
      });
    }
    return Promise.resolve({ status: 'success', data: [] });
  }),
}));

vi.mock('../platform/http', () => ({ httpFetch: vi.fn() }));

function withQueryClient(children: React.ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

beforeEach(() => {
  useDeviceStore.setState({
    deviceId: 'device-1',
    settings: { backend_url: 'http://localhost:8080', api_key: 'k', device_id: 'device-1' } as AppSettings,
  });
});

describe('SystemLog page', () => {
  it('mặc định ở chế độ Quan trọng và hiện thanh tóm tắt sức khoẻ', async () => {
    render(withQueryClient(<SystemLog />));
    await waitFor(() => expect(screen.getByText(/1 lần châm EC/)).toBeInTheDocument());
    expect(screen.getByText('Quan trọng')).toBeInTheDocument();
  });

  it('bấm toggle chuyển sang Toàn bộ kỹ thuật', async () => {
    render(withQueryClient(<SystemLog />));
    await waitFor(() => expect(screen.getByRole('switch')).toBeInTheDocument());
    fireEvent.click(screen.getByRole('switch'));
    expect(screen.getByText('Toàn bộ kỹ thuật')).toBeInTheDocument();
  });

  it('gõ vào ô tìm kiếm cập nhật giá trị input', async () => {
    render(withQueryClient(<SystemLog />));
    await waitFor(() => expect(screen.getByLabelText('Tìm kiếm nhật ký')).toBeInTheDocument());
    fireEvent.change(screen.getByLabelText('Tìm kiếm nhật ký'), { target: { value: 'châm ec' } });
    expect(screen.getByLabelText('Tìm kiếm nhật ký')).toHaveValue('châm ec');
  });

  it('có link mở Grafana', async () => {
    render(withQueryClient(<SystemLog />));
    await waitFor(() => expect(screen.getByText('Mở Grafana')).toBeInTheDocument());
    expect(screen.getByText('Mở Grafana').closest('a')).toHaveAttribute('href', 'http://localhost:3000');
  });
});
