import { useState } from "react";
import { useOwnedDevices } from "../../hooks/useOwnedDevices";
import { useApplyTemplate } from "../../hooks/useAutomationScripts";
import { UserScript } from "../../types/automation";

interface AutomationMultiDeviceTemplatePanelProps {
  currentScript: UserScript;
}

export function AutomationMultiDeviceTemplatePanel({ currentScript }: AutomationMultiDeviceTemplatePanelProps) {
  const { data: devices, isLoading } = useOwnedDevices() as any;
  const [selectedDevices, setSelectedDevices] = useState<Record<string, boolean>>({});
  const applyMutation = useApplyTemplate(currentScript.device_id, currentScript.id);

  if (isLoading) return <div>Đang tải...</div>;

  const toggleDevice = (id: string) => {
    setSelectedDevices((prev) => ({
      ...prev,
      [id]: !prev[id],
    }));
  };

  const selectedCount = Object.values(selectedDevices).filter(Boolean).length;

  const handleApply = () => {
    const targets = Object.keys(selectedDevices)
      .filter((id) => selectedDevices[id])
      .map((id) => ({
        device_id: id,
        overrides: {},
      }));

    if (targets.length === 0) return;
    applyMutation.mutate(targets);
  };

  return (
    <div className="ui-card p-4 mt-6">
      <h3 className="farm-section-title text-lg font-bold mb-4">
        Áp Flow template cho nhiều thiết bị
      </h3>

      <div className="farm-muted-panel bg-purple-50 border border-purple-100 p-3 rounded mb-4 text-purple-800 text-sm">
        <p>
          Lưu ý: Các thiết bị có override cục bộ sẽ được giữ nguyên (giống gốc).
          Các thiết bị khác sẽ được áp dụng Flow này.
        </p>
      </div>

      <div className="space-y-2 mb-4">
        {devices?.map((device: any) => (
          <label
            key={device.id}
            className="flex items-center gap-3 p-2 border rounded hover:bg-gray-50 cursor-pointer"
          >
            <input
              type="checkbox"
              className="w-4 h-4 text-emerald-600 rounded"
              checked={!!selectedDevices[device.id]}
              onChange={() => toggleDevice(device.id)}
            />
            <span className="font-medium">{device.name}</span>
            {device.id === "dev2" ? (
              <span className="farm-status-pill bg-amber-100 text-amber-800 text-xs px-2 py-0.5 rounded">
                override
              </span>
            ) : (
              <span className="farm-status-pill bg-gray-100 text-gray-800 text-xs px-2 py-0.5 rounded">
                giống gốc
              </span>
            )}
          </label>
        ))}
      </div>

      {applyMutation.isSuccess && (
        <div className="p-2 mb-3 text-xs bg-emerald-50 text-emerald-700 border border-emerald-200 rounded">
          Áp dụng Flow template thành công!
        </div>
      )}

      {applyMutation.isError && (
        <div className="p-2 mb-3 text-xs bg-red-50 text-red-700 border border-red-200 rounded">
          Lỗi áp dụng template: {String(applyMutation.error)}
        </div>
      )}

      <button
        className={`ui-btn-md ui-btn-primary w-full ${selectedCount === 0 || applyMutation.isPending ? "opacity-50 cursor-not-allowed" : ""}`}
        disabled={selectedCount === 0 || applyMutation.isPending}
        onClick={handleApply}
      >
        {applyMutation.isPending
          ? "Đang áp dụng..."
          : `Áp dụng cho ${selectedCount} thiết bị đã chọn`}
      </button>
    </div>
  );
}
