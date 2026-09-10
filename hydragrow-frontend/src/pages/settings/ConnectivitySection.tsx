import React from 'react';
import { Network, Zap } from 'lucide-react';
import toast from 'react-hot-toast';
import { AccordionSection } from '../../components/ui/AccordionSection';
import { InputGroup } from '../../components/ui/InputGroup';
import type { OtaStatus, WifiCandidate, WifiConfigStatus } from '../../types/models';

type InputEvent = React.ChangeEvent<HTMLInputElement | HTMLSelectElement>;

export interface ConnectivitySectionProps {
  openSection: string | null;
  onToggleSection: (id: string) => void;
  nodeRedEditorUrl: string;
  integrationTopic: string;
  ctxDeviceId: string | null | undefined;
  appSettings: { api_key: string; backend_url: string };
  setAppSettings: React.Dispatch<React.SetStateAction<{ api_key: string; backend_url: string }>>;
  handleForgetApiKey: () => void;
  otaStatus: OtaStatus | null;
  isTriggeringOta: boolean;
  handleTriggerOta: () => void;
  isProvisioningOta: boolean;
  handleTriggerOtaWifi: () => void;
  wifiCandidates: WifiCandidate[];
  setWifiCandidates: React.Dispatch<React.SetStateAction<WifiCandidate[]>>;
  updateWifiCandidate: (index: number, patch: Partial<WifiCandidate>) => void;
  isSavingWifi: boolean;
  handleSaveWifiList: () => void;
  wifiConfig: WifiConfigStatus | null;
  knownSsids: string[];
}

const WIFI_STATE_LABEL: Record<string, string> = {
  applied: 'Đã áp dụng',
  pending: 'Đang áp dụng...',
  rolled_back: 'Rollback, giữ cấu hình cũ',
  rejected: 'Bị từ chối',
  unknown: 'Chưa có thông tin',
};

export const ConnectivitySection: React.FC<ConnectivitySectionProps> = ({
  openSection,
  onToggleSection,
  nodeRedEditorUrl,
  integrationTopic,
  ctxDeviceId,
  appSettings,
  setAppSettings,
  handleForgetApiKey,
  otaStatus,
  isTriggeringOta,
  handleTriggerOta,
  isProvisioningOta,
  handleTriggerOtaWifi,
  wifiCandidates,
  setWifiCandidates,
  updateWifiCandidate,
  isSavingWifi,
  handleSaveWifiList,
  wifiConfig,
  knownSsids,
}) => {
  return (
    <div className="space-y-4">
      {/* INTEGRATIONS */}
      <AccordionSection
        id="integrations"
        title="Tích hợp & Node-RED"
        icon={Network}
        isOpen={openSection === 'integrations'}
        onToggle={() => onToggleSection('integrations')}
      >
        <div className="space-y-4 p-1">
          <div className="space-y-1">
            <label className="text-sm font-medium text-primary-deep">Node-RED Editor URL</label>
            <div className="flex items-center gap-2">
              <a
                href={nodeRedEditorUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm font-mono text-primary underline hover:text-primary-deep break-all"
              >
                {nodeRedEditorUrl}
              </a>
            </div>
            <p className="text-xs text-text-muted">
              Truy cập trình thiết kế luồng tự động hoá Node-RED để nhận alert và chuyển tiếp tới Telegram / Email / Home Assistant.
            </p>
          </div>
          <div className="space-y-1">
            <label className="text-sm font-medium text-primary-deep">MQTT Integration Topic (Outbound)</label>
            <div className="flex items-center gap-2">
              <p className="flex-1 text-sm text-primary-deep bg-surface-muted px-3 py-2 rounded-lg font-mono break-all border border-line">
                {integrationTopic}
              </p>
              <button
                type="button"
                onClick={() => {
                  navigator.clipboard.writeText(integrationTopic);
                  toast.success('Đã sao chép topic MQTT tích hợp!');
                }}
                className="rounded-xl border border-line bg-white px-3 py-2 text-xs font-semibold text-primary-deep transition-colors hover:bg-soft flex-shrink-0"
              >
                Sao chép
              </button>
            </div>
            <p className="text-xs text-text-muted">
              Topic một chiều backend → Node-RED dùng để fan-out các cảnh báo hệ thống (SystemAlert).
            </p>
          </div>
        </div>
      </AccordionSection>

      {/* NETWORK */}
      <AccordionSection
        id="network"
        title="Thiết bị & Kết nối"
        icon={Network}
        isOpen={openSection === 'network'}
        onToggle={() => onToggleSection('network')}
      >
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 p-1">
          <div className="space-y-1">
            <label className="text-sm font-medium text-primary-deep">Device ID (đang hoạt động)</label>
            <p className="text-sm text-primary-deep bg-surface-muted px-3 py-2 rounded-lg font-mono">
              {ctxDeviceId ?? <span className="text-faint italic">Chưa chọn thiết bị — vào "Thiết Bị Của Tôi"</span>}
            </p>
          </div>
          <div className="space-y-2">
            <InputGroup
              label="API Key"
              type="password"
              value={appSettings.api_key}
              onChange={(e: InputEvent) => setAppSettings({ ...appSettings, api_key: e.target.value })}
            />
            <div className="rounded-xl border border-amber-200 bg-amber-50 p-3 text-xs text-amber-900">
              Web build chỉ lưu API key trong phiên hiện tại; Tauri lưu khoá trong OS credential vault.
            </div>
            <button
              type="button"
              onClick={handleForgetApiKey}
              className="w-full rounded-xl border border-red-200 bg-white/90 px-3 py-2 text-xs font-semibold text-red-600 transition-colors hover:bg-red-50"
            >
              Quên / xoá API key
            </button>
          </div>
        </div>
      </AccordionSection>

      {/* FIRMWARE */}
      <AccordionSection
        id="firmware"
        title="Cập nhật Firmware"
        icon={Zap}
        isOpen={openSection === 'firmware'}
        onToggle={() => onToggleSection('firmware')}
      >
        <div className="space-y-3 p-1">
          {otaStatus ? (
            <>
              <div className="flex items-center justify-between rounded-xl border border-line bg-white/85 p-3">
                <div>
                  <p className="text-xs text-text-muted">Phiên bản hiện tại</p>
                  <p className="text-sm font-semibold text-primary-deep">{otaStatus.current_version}</p>
                </div>
                {otaStatus.update_available && (
                  <div className="text-right">
                    <p className="text-xs text-amber-700">Có bản mới</p>
                    <p className="text-sm font-semibold text-amber-800">{otaStatus.latest_version}</p>
                  </div>
                )}
              </div>
              <button
                type="button"
                disabled={!otaStatus.update_available || isProvisioningOta}
                onClick={handleTriggerOtaWifi}
                className="w-full rounded-xl border border-primary bg-primary px-3 py-2 text-sm font-semibold text-white transition-colors hover:bg-primary-deep disabled:cursor-not-allowed disabled:opacity-50"
              >
                {isProvisioningOta
                  ? 'Đang gửi OTA + WiFi...'
                  : 'Cập nhật firmware & áp dụng WiFi'}
              </button>
              <button
                type="button"
                disabled={!otaStatus.update_available || isTriggeringOta}
                onClick={handleTriggerOta}
                className="w-full rounded-xl border border-amber-300 bg-amber-500 px-3 py-2 text-sm font-semibold text-white transition-colors hover:bg-amber-600 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {isTriggeringOta
                  ? 'Đang gửi lệnh cập nhật...'
                  : otaStatus.update_available
                  ? 'Cập nhật firmware, giữ WiFi hiện tại'
                  : 'Đã ở phiên bản mới nhất'}
              </button>
              <p className="text-xs text-text-muted">
                WiFi mới chỉ có hiệu lực sau khi OTA thành công và thiết bị khởi động lại; mật khẩu sai sẽ tự rollback về WiFi cũ.
              </p>
            </>
          ) : (
            <p className="text-xs text-text-muted">Đang tải thông tin firmware...</p>
          )}
        </div>
      </AccordionSection>

      {/* WIFI */}
      <AccordionSection
        id="wifi"
        title="Mạng WiFi thiết bị (ưu tiên)"
        icon={Network}
        isOpen={openSection === 'wifi'}
        onToggle={() => onToggleSection('wifi')}
      >
        <div className="space-y-3 p-1">
          {wifiConfig && (
            <p className="text-xs font-medium text-primary-deep" data-testid="wifi-config-state">
              WiFi config v{wifiConfig.config_version} — {WIFI_STATE_LABEL[wifiConfig.state] ?? wifiConfig.state}
            </p>
          )}
          {wifiCandidates.map((candidate, index) => {
            const isKnown = knownSsids.includes(candidate.ssid.trim()) && candidate.ssid.trim() !== '';
            return (
            <div key={`${index}-${candidate.priority}`} className="grid grid-cols-1 gap-2 md:grid-cols-[1fr_1fr_80px_32px] md:items-end">
              <InputGroup
                label={`SSID #${index + 1}`}
                type="text"
                value={candidate.ssid}
                onChange={(event: InputEvent) => updateWifiCandidate(index, { ssid: event.target.value })}
              />
              <InputGroup
                label="Mật khẩu"
                type="password"
                value={candidate.password}
                placeholder={isKnown ? 'Để trống để giữ mật khẩu hiện tại' : undefined}
                onChange={(event: InputEvent) => updateWifiCandidate(index, { password: event.target.value })}
              />
              <InputGroup
                label="Ưu tiên"
                type="number"
                value={String(candidate.priority)}
                onChange={(event: InputEvent) => updateWifiCandidate(index, { priority: Math.max(0, Math.min(255, Number(event.target.value) || 0)) })}
              />
              <button
                type="button"
                aria-label={`Xóa SSID ${index + 1}`}
                onClick={() => setWifiCandidates((current) => current.filter((_, candidateIndex) => candidateIndex !== index))}
                className="pb-2 text-xs text-red-500"
              >
                ✕
              </button>
            </div>
            );
          })}
          <button
            type="button"
            onClick={() => setWifiCandidates((current) => [...current, { ssid: '', password: '', priority: current.length }])}
            className="text-xs font-medium text-primary"
          >
            + Thêm mạng WiFi
          </button>
          <button
            type="button"
            disabled={isSavingWifi}
            onClick={handleSaveWifiList}
            className="w-full rounded-xl border border-primary bg-primary px-3 py-2 text-sm font-semibold text-white hover:bg-primary-deep disabled:opacity-50"
          >
            {isSavingWifi ? 'Đang gửi...' : 'Lưu WiFi (áp dụng sau khi khởi động lại)'}
          </button>
          <p className="text-xs text-text-muted">
            Mật khẩu chỉ tồn tại trong lúc gửi; hệ thống không bao giờ hiển thị lại mật khẩu đã lưu — để trống nghĩa là giữ nguyên.
          </p>
        </div>
      </AccordionSection>
    </div>
  );
};
