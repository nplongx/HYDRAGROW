import React from 'react';
import { Network, Zap } from 'lucide-react';
import toast from 'react-hot-toast';
import { AccordionSection } from '../../components/ui/AccordionSection';
import { InputGroup } from '../../components/ui/InputGroup';
import type { OtaStatus, WifiCandidate } from '../../types/models';

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
  wifiCandidates: WifiCandidate[];
  setWifiCandidates: React.Dispatch<React.SetStateAction<WifiCandidate[]>>;
  updateWifiCandidate: (index: number, patch: Partial<WifiCandidate>) => void;
  isSavingWifi: boolean;
  handleSaveWifiList: () => void;
}

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
  wifiCandidates,
  setWifiCandidates,
  updateWifiCandidate,
  isSavingWifi,
  handleSaveWifiList,
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
            <label className="text-sm font-medium text-emerald-950">Node-RED Editor URL</label>
            <div className="flex items-center gap-2">
              <a
                href={nodeRedEditorUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm font-mono text-sky-600 underline hover:text-sky-800 break-all"
              >
                {nodeRedEditorUrl}
              </a>
            </div>
            <p className="text-xs text-emerald-700/75">
              Truy cập trình thiết kế luồng tự động hoá Node-RED để nhận alert và chuyển tiếp tới Telegram / Email / Home Assistant.
            </p>
          </div>
          <div className="space-y-1">
            <label className="text-sm font-medium text-emerald-950">MQTT Integration Topic (Outbound)</label>
            <div className="flex items-center gap-2">
              <p className="flex-1 text-sm text-emerald-800 bg-emerald-50 px-3 py-2 rounded-lg font-mono break-all border border-emerald-100">
                {integrationTopic}
              </p>
              <button
                type="button"
                onClick={() => {
                  navigator.clipboard.writeText(integrationTopic);
                  toast.success('Đã sao chép topic MQTT tích hợp!');
                }}
                className="rounded-xl border border-emerald-200 bg-white px-3 py-2 text-xs font-semibold text-emerald-800 transition-colors hover:bg-emerald-50 flex-shrink-0"
              >
                Sao chép
              </button>
            </div>
            <p className="text-xs text-emerald-700/75">
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
            <label className="text-sm font-medium text-emerald-950">Device ID (đang hoạt động)</label>
            <p className="text-sm text-emerald-800 bg-emerald-50 px-3 py-2 rounded-lg font-mono">
              {ctxDeviceId ?? <span className="text-gray-400 italic">Chưa chọn thiết bị — vào "Thiết Bị Của Tôi"</span>}
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
              <div className="flex items-center justify-between rounded-xl border border-emerald-100 bg-white/85 p-3">
                <div>
                  <p className="text-xs text-emerald-700/75">Phiên bản hiện tại</p>
                  <p className="text-sm font-semibold text-emerald-950">{otaStatus.current_version}</p>
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
                disabled={!otaStatus.update_available || isTriggeringOta}
                onClick={handleTriggerOta}
                className="w-full rounded-xl border border-amber-300 bg-amber-500 px-3 py-2 text-sm font-semibold text-white transition-colors hover:bg-amber-600 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {isTriggeringOta
                  ? 'Đang gửi lệnh cập nhật...'
                  : otaStatus.update_available
                  ? 'Cập nhật ngay (thiết bị sẽ khởi động lại)'
                  : 'Đã ở phiên bản mới nhất'}
              </button>
            </>
          ) : (
            <p className="text-xs text-emerald-700/75">Đang tải thông tin firmware...</p>
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
          {wifiCandidates.map((candidate, index) => (
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
          ))}
          <button
            type="button"
            onClick={() => setWifiCandidates((current) => [...current, { ssid: '', password: '', priority: current.length }])}
            className="text-xs font-medium text-emerald-700"
          >
            + Thêm mạng WiFi
          </button>
          <button
            type="button"
            disabled={isSavingWifi}
            onClick={handleSaveWifiList}
            className="w-full rounded-xl border border-emerald-300 bg-emerald-600 px-3 py-2 text-sm font-semibold text-white hover:bg-emerald-700 disabled:opacity-50"
          >
            {isSavingWifi ? 'Đang gửi...' : 'Lưu danh sách WiFi (áp dụng sau khi khởi động lại)'}
          </button>
        </div>
      </AccordionSection>
    </div>
  );
};
