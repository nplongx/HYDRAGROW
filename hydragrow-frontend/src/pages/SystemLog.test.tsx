import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import SystemLog from './SystemLog';
import { useDeviceStore } from '../store/useDeviceStore';
import { httpFetch } from '../platform/http';
import type { AppSettings } from '../types/models';
import type { SystemEvent } from '../components/logs/EventLogCard';

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

function makeEvents(count: number, prefix: string, startTimestamp: number): SystemEvent[] {
  return Array.from({ length: count }, (_, i) => ({
    id: startTimestamp - i,
    device_id: 'device-1',
    level: 'info',
    category: 'dosing',
    title: `${prefix} ${i}`,
    message: `msg ${i}`,
    timestamp: startTimestamp - i * 1000,
  }));
}

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

describe('SystemLog pagination', () => {
  const page1 = makeEvents(200, 'Dosing event', 2_000_000_000_000);
  const page2 = makeEvents(50, 'Older dosing event', 1_000_000_000_000);

  beforeEach(() => {
    vi.mocked(httpFetch).mockReset();
  });

  it('tải thêm sự kiện cũ hơn khi bấm nút "Tải thêm sự kiện cũ hơn"', async () => {
    vi.mocked(httpFetch).mockImplementation((url) => {
      const target = String(url);
      const data = target.includes('before_timestamp') ? page2 : page1;
      return Promise.resolve({ ok: true, json: async () => ({ status: 'success', data }) } as Response);
    });

    render(withQueryClient(<SystemLog />));

    await waitFor(() => expect(screen.getByText('Dosing event 0')).toBeInTheDocument());
    expect(screen.queryByText('Older dosing event 0')).not.toBeInTheDocument();

    fireEvent.click(screen.getByText('Tải thêm sự kiện cũ hơn'));

    await waitFor(() => expect(screen.getByText('Older dosing event 0')).toBeInTheDocument());

    const secondCallUrl = vi
      .mocked(httpFetch)
      .mock.calls.map((call) => String(call[0]))
      .find((url) => url.includes('before_timestamp'));
    expect(secondCallUrl).toContain(`before_timestamp=${page1[page1.length - 1].timestamp}`);
  }, 15000);

  it('ẩn nút "Tải thêm" khi trang cuối trả về ít hơn PAGE_SIZE sự kiện', async () => {
    vi.mocked(httpFetch).mockResolvedValue({
      ok: true,
      json: async () => ({ status: 'success', data: page2 }),
    } as Response);

    render(withQueryClient(<SystemLog />));

    await waitFor(() => expect(screen.getByText('Older dosing event 0')).toBeInTheDocument());
    expect(screen.queryByText('Tải thêm sự kiện cũ hơn')).not.toBeInTheDocument();
  });
});

describe('SystemLog date grouping', () => {
  const DAY = 86400000;
  const now = Date.now();
  const todayEvent = { id: 3, device_id: 'device-1', level: 'info', category: 'dosing', title: 'Sự kiện hôm nay', message: 'msg', timestamp: now };
  const yesterdayEvent = { id: 2, device_id: 'device-1', level: 'info', category: 'dosing', title: 'Sự kiện hôm qua', message: 'msg', timestamp: now - DAY };
  const oldEvent = { id: 1, device_id: 'device-1', level: 'warning', category: 'dosing', title: 'Sự kiện cũ', message: 'msg', timestamp: now - 3 * DAY };

  beforeEach(() => {
    vi.mocked(httpFetch).mockReset();
  });

  it('nhóm sự kiện theo ngày với header HÔM NAY / HÔM QUA / ngày cụ thể', async () => {
    vi.mocked(httpFetch).mockResolvedValue({
      ok: true,
      json: async () => ({ status: 'success', data: [todayEvent, yesterdayEvent, oldEvent] }),
    } as Response);

    render(withQueryClient(<SystemLog />));

    await waitFor(() => expect(screen.getByText('Sự kiện hôm nay')).toBeInTheDocument());
    expect(screen.getByText('HÔM NAY')).toBeInTheDocument();
    expect(screen.getByText('HÔM QUA')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText('Sự kiện cũ')).toBeInTheDocument());
  });
});
