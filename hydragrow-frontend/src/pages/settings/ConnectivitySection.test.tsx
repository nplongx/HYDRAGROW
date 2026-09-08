import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ConnectivitySection } from './ConnectivitySection';
import type { WifiConfigStatus } from '../../types/models';

const baseProps = {
  openSection: 'firmware' as string | null,
  onToggleSection: () => {},
  nodeRedEditorUrl: 'http://localhost:1880',
  integrationTopic: 'hydragrow/dev/integrations/out',
  ctxDeviceId: 'dev-123',
  appSettings: { api_key: 'test-key', backend_url: 'http://localhost:8080' },
  setAppSettings: () => {},
  handleForgetApiKey: () => {},
  otaStatus: null,
  isTriggeringOta: false,
  handleTriggerOta: () => {},
  isProvisioningOta: false,
  handleTriggerOtaWifi: () => {},
  wifiCandidates: [],
  setWifiCandidates: () => {},
  updateWifiCandidate: () => {},
  isSavingWifi: false,
  handleSaveWifiList: () => {},
  wifiConfig: null,
  knownSsids: [] as string[],
};

describe('ConnectivitySection', () => {
  it('render đủ 4 accordion con: Tích hợp, Thiết bị & Kết nối, Cập nhật Firmware, Mạng WiFi', () => {
    render(<ConnectivitySection {...baseProps} openSection={null} />);
    expect(screen.getByText('Tích hợp & Node-RED')).toBeInTheDocument();
    expect(screen.getByText('Thiết bị & Kết nối')).toBeInTheDocument();
    expect(screen.getByText('Cập nhật Firmware')).toBeInTheDocument();
    expect(screen.getByText('Mạng WiFi thiết bị (ưu tiên)')).toBeInTheDocument();
  });

  it('OTA-only button is enabled when update exists', () => {
    render(
      <ConnectivitySection
        {...baseProps}
        otaStatus={{
          device_id: 'dev-123',
          current_version: 'v0.8.0',
          latest_version: 'v0.9.0',
          update_available: true,
        }}
      />
    );
    const otaOnly = screen.getByRole('button', { name: /giữ WiFi hiện tại/i });
    expect(otaOnly).toBeEnabled();
  });

  it('combined button is enabled when update exists and wifi fields validate', () => {
    const handleTriggerOtaWifi = vi.fn();
    render(
      <ConnectivitySection
        {...baseProps}
        otaStatus={{
          device_id: 'dev-123',
          current_version: 'v0.8.0',
          latest_version: 'v0.9.0',
          update_available: true,
        }}
        handleTriggerOtaWifi={handleTriggerOtaWifi}
        wifiCandidates={[{ ssid: 'Farm-A', password: 'secret', priority: 0 }]}
      />
    );
    const combined = screen.getByRole('button', { name: /áp dụng WiFi/i });
    expect(combined).toBeEnabled();
    fireEvent.click(combined);
    expect(handleTriggerOtaWifi).toHaveBeenCalledTimes(1);
  });

  it('existing SSIDs render with blank masked passwords', () => {
    const wifiConfig: WifiConfigStatus = {
      device_id: 'dev-123',
      ssids: [{ ssid: 'Farm-A', priority: 0 }],
      config_version: 7,
      state: 'applied',
    };
    render(
      <ConnectivitySection
        {...baseProps}
        openSection="wifi"
        wifiConfig={wifiConfig}
        knownSsids={['Farm-A']}
        wifiCandidates={[{ ssid: 'Farm-A', password: '', priority: 0 }]}
      />
    );
    expect(screen.getByDisplayValue('Farm-A')).toBeInTheDocument();
    const passwordInput = screen.getByPlaceholderText(/giữ mật khẩu hiện tại/i);
    expect(passwordInput).toHaveValue('');
  });

  it.each([
    ['applied', 'WiFi config v7 — Đã áp dụng'],
    ['pending', 'WiFi config v8 — Đang áp dụng...'],
    ['rolled_back', 'WiFi config v8 — Rollback, giữ cấu hình cũ'],
  ] as Array<[WifiConfigStatus['state'], string]>)(
    'renders %s provisioning status',
    (state, label) => {
      const version = state === 'applied' ? 7 : 8;
      render(
        <ConnectivitySection
          {...baseProps}
          openSection="wifi"
          wifiConfig={{ device_id: 'dev-123', ssids: [], config_version: version, state }}
          knownSsids={[]}
        />
      );
      expect(screen.getByTestId('wifi-config-state')).toHaveTextContent(label);
    }
  );
});
