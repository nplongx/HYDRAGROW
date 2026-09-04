import { useOwnedDevices } from "../../hooks/useOwnedDevices";
import { UserScript } from "../../types/automation";

interface AutomationMultiDeviceTemplatePanelProps {
  currentScript: UserScript;
}

export function AutomationMultiDeviceTemplatePanel(_props: AutomationMultiDeviceTemplatePanelProps) {
  const { data: devices, isLoading } = useOwnedDevices() as any;

  if (isLoading) return <div>Đang tải...</div>;

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
        <p className="mt-2 text-red-600 font-medium">
          Tính năng đang phát triển. Chưa có API hỗ trợ áp dụng Flow hàng loạt.
        </p>
      </div>

      <div className="space-y-2 mb-4">
        {devices?.map((device: any) => (
          <label
            key={device.id}
            className="flex items-center gap-3 p-2 border rounded hover:bg-gray-50"
          >
            <input type="checkbox" className="w-4 h-4" />
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

      <button
        className="ui-btn-md ui-btn-primary w-full opacity-50 cursor-not-allowed"
        disabled
      >
        Áp dụng cho 0 thiết bị đã chọn
      </button>
    </div>
  );
}
