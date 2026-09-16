import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, expect, it, vi } from 'vitest';
import { ConfigBackup } from './ConfigBackup';
import { apiPost } from '../lib/apiClient';

vi.mock('../contexts/StationContext', () => ({
  useStationContext: () => ({ selectedDeviceId: 'device-1' }),
}));

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn(),
  apiPost: vi.fn(),
}));

const artifact = {
  format: 'hydragrow-backup',
  schema_version: 1,
  created_at: '2026-09-16T00:00:00Z',
  producer: { application: 'HydraGrow', version: '0.1.0' },
  scope: 'device' as const,
  source: { station_id: null, device_ids: ['device-1'], controller_ids: [] },
  configuration: {
    device_config: { device_id: 'device-1' },
    water_config: { device_id: 'device-1' },
    safety_config: { device_id: 'device-1' },
    sensor_calibration: { device_id: 'device-1' },
    dosing_calibration: { device_id: 'device-1' },
  },
  integrity: { algorithm: 'SHA-256', digest: 'digest' },
};

describe('ConfigBackup', () => {
  it('previews before applying and sends the same validated artifact to restore', async () => {
    vi.stubGlobal('confirm', vi.fn(() => true));
    vi.mocked(apiPost)
      .mockResolvedValueOnce({
        status: 'validated',
        target_device_id: 'device-1',
        source_device_id: 'device-1',
        additions: [],
        changes: ['device_config'],
        unchanged: ['water_config'],
        rejected: [],
        warnings: [],
        apply_permitted: true,
      })
      .mockResolvedValueOnce({ status: 'applied_sync_pending', device_id: 'device-1' });

    const { container } = render(
      <QueryClientProvider client={new QueryClient()}>
        <ConfigBackup />
      </QueryClientProvider>,
    );

    const input = container.querySelector('input[type="file"]');
    expect(input).not.toBeNull();
    const file = new File([JSON.stringify(artifact)], 'backup.json', {
      type: 'application/json',
    });
    fireEvent.change(input!, { target: { files: [file] } });

    expect(await screen.findByText(/đã kiểm tra backup/i)).toBeInTheDocument();
    expect(apiPost).toHaveBeenCalledWith('/devices/device-1/admin/restore/validate', artifact);

    fireEvent.click(screen.getByRole('button', { name: /áp dụng backup/i }));
    await waitFor(() =>
      expect(apiPost).toHaveBeenCalledWith('/devices/device-1/admin/restore', artifact),
    );
    vi.unstubAllGlobals();
  });
});
