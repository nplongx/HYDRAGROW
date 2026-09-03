import { useState } from 'react';
import type { OwnedDevice } from '../../types/models';

export interface AutomationMultiDeviceTemplatePanelProps {
  devices: OwnedDevice[];
  currentFlowName: string;
}

export function AutomationMultiDeviceTemplatePanel({ devices, currentFlowName }: AutomationMultiDeviceTemplatePanelProps) {
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  const toggleDevice = (id: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <div className="ui-card flex flex-col gap-4 border-2 border-emerald-500 shadow-xl overflow-hidden mt-6">
      <div className="flex items-center justify-between border-b border-emerald-100 pb-3 -mx-4 -mt-4 px-4 pt-4 bg-emerald-50/50">
        <h2 className="text-sm font-bold text-emerald-950">Áp Flow template cho nhiều thiết bị</h2>
      </div>

      <div className="rounded-lg border border-slate-200 divide-y divide-slate-100 overflow-hidden">
        {devices.map(device => {
          // Mock override state check - in reality, we'd compare device local flow against template flow
          const hasOverride = device.device_id.includes('1'); // Mock logic for visual testing

          return (
            <div key={device.device_id} className="flex items-center justify-between p-3 bg-white hover:bg-slate-50">
              <label className="flex items-center gap-3 cursor-pointer">
                <input
                  type="checkbox"
                  className="rounded border-slate-300 text-emerald-600 focus:ring-emerald-500 w-4 h-4"
                  checked={selectedIds.has(device.device_id)}
                  onChange={() => toggleDevice(device.device_id)}
                />
                <span className="text-sm font-semibold text-emerald-900">{device.label || device.device_id}</span>
              </label>

              <div className="flex items-center gap-3">
                <span className="text-xs font-mono text-slate-500 truncate max-w-32 hidden sm:inline-block">
                  {currentFlowName}
                </span>
                {hasOverride ? (
                  <span className="inline-flex px-2 py-0.5 rounded text-[10px] font-bold tracking-wider bg-amber-100 text-amber-700">
                    OVERRIDE
                  </span>
                ) : (
                  <span className="inline-flex px-2 py-0.5 rounded text-[10px] font-bold tracking-wider bg-slate-100 text-slate-600">
                    GIỐNG GỐC
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>

      <div className="farm-muted-panel bg-fuchsia-50/70 border-fuchsia-100 mt-2">
        <p className="text-xs font-medium text-fuchsia-900 leading-relaxed">
          <strong>Đồng bộ Flow:</strong> Thay đổi cấu hình này sẽ áp dụng đè lên các thiết bị. Flow được tạo/sửa thủ công trên thiết bị (local overrides) sẽ được giữ lại (local overrides are preserved) nhưng Flow có cùng Tên/ID từ Template sẽ bị ghi đè.
        </p>
      </div>

      <div className="flex flex-col gap-2 mt-2">
        <button
          className="ui-btn-primary bg-emerald-700 w-full"
          disabled={true}
        >
          Áp dụng cho {selectedIds.size} thiết bị đã chọn
        </button>
        <p className="text-center text-[10px] font-semibold text-red-500">
          Không có API hỗ trợ bulk-apply Automation Flow.
        </p>
      </div>
    </div>
  );
}
