import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { ConnectivitySection } from './ConnectivitySection';

describe('ConnectivitySection', () => {
  it('render đủ 4 accordion con: Tích hợp, Thiết bị & Kết nối, Cập nhật Firmware, Mạng WiFi', () => {
    render(
      <ConnectivitySection
        openSection={null}
        onToggleSection={() => {}}
        nodeRedEditorUrl="http://localhost:1880"
        integrationTopic="hydragrow/dev/integrations/out"
        ctxDeviceId="dev-123"
        appSettings={{ api_key: 'test-key', backend_url: 'http://localhost:8080' }}
        setAppSettings={() => {}}
        handleForgetApiKey={() => {}}
        otaStatus={null}
        isTriggeringOta={false}
        handleTriggerOta={() => {}}
        wifiCandidates={[]}
        setWifiCandidates={() => {}}
        updateWifiCandidate={() => {}}
        isSavingWifi={false}
        handleSaveWifiList={() => {}}
      />
    );
    expect(screen.getByText('Tích hợp & Node-RED')).toBeInTheDocument();
    expect(screen.getByText('Thiết bị & Kết nối')).toBeInTheDocument();
    expect(screen.getByText('Cập nhật Firmware')).toBeInTheDocument();
    expect(screen.getByText('Mạng WiFi thiết bị (ưu tiên)')).toBeInTheDocument();
  });
});
