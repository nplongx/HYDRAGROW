import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  DevicePairing,
  parseDeviceIdFromQr,
} from './DevicePairing';
import {
  ScanConfirmOverlay,
  deriveConfirmationCode,
} from '../components/pairing/ScanConfirmOverlay';
import * as apiClient from '../lib/apiClient';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn(),
  apiPost: vi.fn(),
  apiDelete: vi.fn(),
  apiPut: vi.fn(),
}));

const mockSetDeviceId = vi.fn();
let mockActiveDeviceId: string | null = 'dev-1';

vi.mock('../store/useDeviceStore', () => ({
  useDeviceStore: vi.fn((selector) =>
    selector({
      deviceId: mockActiveDeviceId,
      setDeviceId: mockSetDeviceId,
    })
  ),
}));

const mockDevices = [
  {
    id: 1,
    user_id: 10,
    device_id: 'dev-1',
    label: 'Giàn Ban Công',
    claimed_at: '2026-09-01T00:00:00Z',
  },
  {
    id: 2,
    user_id: 10,
    device_id: 'dev-2',
    label: null,
    claimed_at: '2026-09-02T00:00:00Z',
  },
];

vi.mock('../hooks/useOwnedDevices', () => ({
  useOwnedDevices: vi.fn(() => ({
    devices: mockDevices,
    loading: false,
    error: null,
    refresh: vi.fn(),
  })),
}));

describe('DevicePairing & QR Scan Flow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockActiveDeviceId = 'dev-1';
    vi.mocked(apiClient.apiGet).mockImplementation(async (path: string) => {
      if (path.includes('/status')) {
        return { is_online: true, last_seen: '2026-09-10T10:00:00Z' };
      }
      return {};
    });
  });

  describe('parseDeviceIdFromQr', () => {
    it('trích xuất device_id từ URI scheme hydragrow://claim/{id}', () => {
      expect(parseDeviceIdFromQr('hydragrow://claim/station-alpha-01')).toBe('station-alpha-01');
      expect(parseDeviceIdFromQr('hydragrow://claim/station-beta?token=abc')).toBe('station-beta');
    });

    it('trích xuất device_id từ URL web https://...', () => {
      expect(parseDeviceIdFromQr('https://app.hydragrow.dev/claim/HG-84KX-01')).toBe('HG-84KX-01');
    });

    it('giữ nguyên chuỗi nếu là Device ID thô', () => {
      expect(parseDeviceIdFromQr('hydra_box_99')).toBe('hydra_box_99');
    });
  });

  describe('deriveConfirmationCode', () => {
    it('tạo mã 4 ký tự có định dạng số và chữ đồng nhất', () => {
      const code1 = deriveConfirmationCode('station-alpha-01');
      const code2 = deriveConfirmationCode('station-alpha-01');
      expect(code1).toBe(code2);
      expect(code1).toMatch(/^\d{2} • [A-Z]{2}$/);
    });
  });

  describe('ScanConfirmOverlay Component', () => {
    it('hiển thị mã đối chiếu và thông tin thiết bị', () => {
      const onConfirm = vi.fn();
      const onCancel = vi.fn();

      render(
        <ScanConfirmOverlay
          deviceId="station-test-01"
          onConfirm={onConfirm}
          onCancel={onCancel}
        />
      );

      expect(screen.getByTestId('confirm-device-id')).toHaveTextContent('station-test-01');
      expect(screen.getByTestId('derived-confirm-code')).toBeInTheDocument();
      expect(screen.getByText(/So khớp với mã 4 ký tự/i)).toBeInTheDocument();
    });

    it('bấm "Mã không khớp" gọi onCancel', () => {
      const onConfirm = vi.fn();
      const onCancel = vi.fn();

      render(
        <ScanConfirmOverlay
          deviceId="station-test-01"
          onConfirm={onConfirm}
          onCancel={onCancel}
        />
      );

      fireEvent.click(screen.getByRole('button', { name: /Mã không khớp/i }));
      expect(onCancel).toHaveBeenCalledOnce();
      expect(onConfirm).not.toHaveBeenCalled();
    });

    it('bấm "Mã khớp & Ghép" gọi onConfirm kèm deviceId và label nhập vào', () => {
      const onConfirm = vi.fn();
      const onCancel = vi.fn();

      render(
        <ScanConfirmOverlay
          deviceId="station-test-01"
          onConfirm={onConfirm}
          onCancel={onCancel}
        />
      );

      const labelInput = screen.getByPlaceholderText(/Ví dụ: Giàn rau ban công tầng 2/i);
      fireEvent.change(labelInput, { target: { value: 'Giàn Tầng Thượng' } });

      fireEvent.click(screen.getByRole('button', { name: /Mã khớp & Ghép/i }));
      expect(onConfirm).toHaveBeenCalledWith('station-test-01', 'Giàn Tầng Thượng');
    });
  });

  describe('DevicePairing Page Integration', () => {
    it('hiển thị danh sách thiết bị đã ghép với trạng thái và mã bảo mật', async () => {
      render(<DevicePairing />);

      expect(screen.getByText('Giàn Ban Công')).toBeInTheDocument();
      expect(screen.getByTestId('device-card-dev-2')).toBeInTheDocument();

      await waitFor(() => {
        const onlineBadges = screen.getAllByText('Trực tuyến');
        expect(onlineBadges.length).toBeGreaterThan(0);
      });
    });

    it('nhập Device ID thủ công và mở confirmation overlay để ghép thiết bị', async () => {
      vi.mocked(apiClient.apiPost).mockResolvedValue({
        device_id: 'station-manual-01',
        label: 'Giàn mới',
        qr_payload: 'hydragrow://claim/station-manual-01',
      });

      render(<DevicePairing />);

      const idInput = screen.getByPlaceholderText(/Ví dụ: hydra_station_01/i);
      const labelInput = screen.getByPlaceholderText(/Ví dụ: Giàn dâu tây trong nhà/i);

      fireEvent.change(idInput, { target: { value: 'station-manual-01' } });
      fireEvent.change(labelInput, { target: { value: 'Giàn mới' } });

      fireEvent.click(screen.getByRole('button', { name: /Tiếp tục xác nhận ghép nối/i }));

      // Overlay opens
      expect(screen.getByTestId('scan-confirm-overlay')).toBeInTheDocument();
      expect(screen.getByTestId('confirm-device-id')).toHaveTextContent('station-manual-01');

      // Click confirm in overlay
      fireEvent.click(screen.getByRole('button', { name: /Mã khớp & Ghép/i }));

      await waitFor(() => {
        expect(apiClient.apiPost).toHaveBeenCalledWith('/devices/claim', {
          device_id: 'station-manual-01',
          label: 'Giàn mới',
        });
      });
    });
  });
});
