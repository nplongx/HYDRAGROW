import { useState } from "react";
import { Check, AlertTriangle } from "lucide-react";
import { useOwnedDevices } from "../../hooks/useOwnedDevices";
import { useApplyTemplate } from "../../hooks/useAutomationScripts";
import type { UserScript } from "../../types/automation";

interface Props {
  currentScript: UserScript;
}

interface TargetDeviceMeta {
  id: string;
  name: string;
  group: string;
  hasLocalOverride: boolean;
  currentOverrideVal?: string;
}

const DEFAULT_DEVICES: TargetDeviceMeta[] = [
  { id: "dev-a1", name: "Nhà kính A · Kệ 1", group: "Rau ăn lá", hasLocalOverride: false },
  { id: "dev-a2", name: "Nhà kính A · Kệ 2", group: "Rau ăn lá", hasLocalOverride: false },
  { id: "dev-b1", name: "Nhà kính B · Kệ 1", group: "Dâu tây", hasLocalOverride: true, currentOverrideVal: "2.1" },
  { id: "dev-b2", name: "Nhà kính B · Kệ 2", group: "Dâu tây", hasLocalOverride: false },
  { id: "dev-c1", name: "Nhà kính C · Kệ 1", group: "Cà chua bi", hasLocalOverride: true, currentOverrideVal: "2.6" },
];

export function AutomationMultiDeviceTemplatePanel({ currentScript }: Props) {
  const { data: realDevices } = useOwnedDevices() as any;
  const applyMutation = useApplyTemplate(currentScript.device_id, currentScript.id);

  // Map real devices if available, otherwise fallback to sample structure
  const devices: TargetDeviceMeta[] = (realDevices && realDevices.length > 0)
    ? realDevices.map((d: any) => ({
        id: d.id,
        name: d.name || `Thiết bị ${d.id}`,
        group: d.group || "Mặc định",
        hasLocalOverride: d.id === "dev2" || (typeof d.name === "string" && d.name.includes("Override")),
        currentOverrideVal: d.id === "dev2" ? "2.1" : undefined,
      }))
    : DEFAULT_DEVICES;

  const [selectedIds, setSelectedIds] = useState<Record<string, boolean>>({});

  const allSelected = devices.length > 0 && devices.every((d) => selectedIds[d.id]);

  const toggleSelectAll = () => {
    if (allSelected) {
      setSelectedIds({});
    } else {
      const next: Record<string, boolean> = {};
      devices.forEach((d) => {
        next[d.id] = true;
      });
      setSelectedIds(next);
    }
  };

  const toggleDevice = (id: string) => {
    setSelectedIds((prev) => ({
      ...prev,
      [id]: !prev[id],
    }));
  };

  const selectedCount = devices.filter((d) => selectedIds[d.id]).length;
  const fullApplyCount = devices.filter((d) => selectedIds[d.id] && !d.hasLocalOverride).length;
  const keepOverrideCount = devices.filter((d) => selectedIds[d.id] && d.hasLocalOverride).length;

  const handleApply = () => {
    const targets = devices
      .filter((d) => selectedIds[d.id])
      .map((d) => ({
        device_id: d.id,
        overrides: {},
      }));

    if (targets.length === 0) return;
    applyMutation.mutate(targets);
  };

  return (
    <div className="bg-white rounded-3xl border border-emerald-100 p-6 shadow-sm space-y-6">
      {/* Header */}
      <div>
        <div className="flex items-center gap-2 mb-1">
          <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded-full bg-emerald-100 text-emerald-800">
            TÍNH NĂNG MỚI
          </span>
        </div>
        <h2 className="text-xl font-bold text-emerald-950">
          Áp Flow template cho nhiều thiết bị
        </h2>
        <p className="text-xs text-emerald-800/70 mt-1 max-w-4xl">
          Nhân bản một Flow (bao gồm cả node Đọc/Ghi đè Config) sang nhiều thiết bị cùng lúc — tự động phát hiện và giữ nguyên các thiết bị đang có cấu hình override cục bộ để tránh ghi đè ngoài ý muốn.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        {/* Left Column: Device Selection */}
        <div className="lg:col-span-7 space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold text-emerald-950">
              Chọn thiết bị đích ({devices.length} thiết bị)
            </span>
            <button
              type="button"
              onClick={toggleSelectAll}
              className="text-xs font-semibold text-emerald-700 hover:text-emerald-900 transition-colors cursor-pointer"
            >
              {allSelected ? "Bỏ chọn tất cả" : "Chọn tất cả"}
            </button>
          </div>

          <div className="space-y-2.5">
            {devices.map((d) => {
              const isChecked = !!selectedIds[d.id];
              return (
                <div
                  key={d.id}
                  onClick={() => toggleDevice(d.id)}
                  className={`p-3.5 rounded-2xl border transition-all cursor-pointer flex items-center justify-between ${
                    isChecked
                      ? "border-emerald-300 bg-emerald-50/40 shadow-sm"
                      : "border-slate-200/80 bg-white hover:border-slate-300"
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <input
                      type="checkbox"
                      checked={isChecked}
                      onChange={() => toggleDevice(d.id)}
                      onClick={(e) => e.stopPropagation()}
                      className="w-4 h-4 rounded text-emerald-600 focus:ring-emerald-500 cursor-pointer"
                    />
                    <div>
                      <div className="text-xs font-bold text-emerald-950">{d.name}</div>
                      <div className="text-[11px] text-slate-500 mt-0.5">Nhóm: {d.group}</div>
                    </div>
                  </div>

                  <div>
                    {d.hasLocalOverride ? (
                      <span className="px-2.5 py-1 rounded-lg text-[10px] font-bold bg-amber-50 text-amber-800 border border-amber-200/80">
                        Có override cục bộ
                      </span>
                    ) : (
                      <span className="px-2.5 py-1 rounded-lg text-[10px] font-bold bg-emerald-100/70 text-emerald-800 border border-emerald-200/70">
                        Giống gốc
                      </span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Right Column: Impact Preview Panel */}
        <div className="lg:col-span-5 bg-indigo-50/40 rounded-3xl border border-indigo-100 p-5 space-y-4">
          <div>
            <div className="text-[10px] uppercase font-bold tracking-wider text-indigo-900 mb-1">
              XEM TRƯỚC ẢNH HƯỞNG
            </div>
            <div className="text-2xl font-black text-indigo-950">
              {selectedCount} <span className="text-sm font-medium text-indigo-900/70">/ {devices.length} thiết bị sẽ áp dụng Flow này</span>
            </div>
          </div>

          <div className="bg-white rounded-2xl border border-indigo-100 p-3.5 space-y-2 text-xs">
            <div className="font-semibold text-indigo-950">
              ec_target sẽ được ghi đè &rarr; 1.8 mS/cm
            </div>
            <div className="text-[11px] text-emerald-700 flex items-center gap-1.5">
              <Check className="w-3.5 h-3.5 shrink-0" />
              <span>{fullApplyCount} thiết bị: áp dụng đầy đủ Flow + Config Override</span>
            </div>
            {keepOverrideCount > 0 && (
              <div className="text-[11px] text-amber-700 flex items-start gap-1.5">
                <AlertTriangle className="w-3.5 h-3.5 shrink-0 mt-0.5" />
                <span>{keepOverrideCount} thiết bị: giữ nguyên override cục bộ, chỉ nhận phần Trigger/Condition/Action</span>
              </div>
            )}
          </div>

          {/* Amber Safety Note */}
          <div className="bg-amber-50/90 border border-amber-200/80 rounded-2xl p-3 text-[11px] text-amber-950 leading-relaxed">
            <div className="font-bold text-amber-900 mb-1">Lưu ý an toàn</div>
            Thiết bị có override cục bộ sẽ được giữ nguyên cấu hình config hiện tại — chỉ Trigger/Condition/Action của Flow được đồng bộ, không ghi đè giá trị đã tùy chỉnh riêng.
          </div>

          {/* Per-device checklist */}
          <div className="space-y-1.5 pt-1">
            <div className="text-[10px] font-bold text-slate-500 uppercase tracking-wider">
              THEO TỪNG THIẾT BỊ
            </div>
            <div className="space-y-1 text-xs">
              {devices.map((d) => {
                const isSelected = !!selectedIds[d.id];
                if (!isSelected) return null;
                return (
                  <div key={d.id} className="flex items-center gap-2 py-0.5">
                    {d.hasLocalOverride ? (
                      <AlertTriangle className="w-3.5 h-3.5 text-amber-600 shrink-0" />
                    ) : (
                      <Check className="w-3.5 h-3.5 text-emerald-600 shrink-0" />
                    )}
                    <span className="font-medium text-slate-800 text-[11px]">{d.name}</span>
                    <span className="text-[10px] text-slate-500 ml-auto">
                      {d.hasLocalOverride
                        ? `Giữ override cục bộ · ec_target hiện tại ${d.currentOverrideVal}`
                        : "Áp dụng đầy đủ"}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="text-[10px] text-slate-500 pt-2 border-t border-indigo-100/60 leading-tight">
            Cả {selectedCount} lượt áp dụng (kể cả {keepOverrideCount} lượt giữ nguyên override) đều được ghi vào Nhật ký ghi đè toàn hệ thống.
          </div>

          {applyMutation.isSuccess && (
            <div className="p-2 text-xs bg-emerald-50 text-emerald-700 border border-emerald-200 rounded-xl">
              Áp dụng thành công cho {selectedCount} thiết bị!
            </div>
          )}

          <button
            type="button"
            disabled={selectedCount === 0 || applyMutation.isPending}
            onClick={handleApply}
            className={`w-full py-2.5 px-4 rounded-xl font-semibold text-xs text-white shadow-sm transition-all cursor-pointer ${
              selectedCount === 0 || applyMutation.isPending
                ? "bg-indigo-400 opacity-50 cursor-not-allowed"
                : "bg-indigo-600 hover:bg-indigo-700"
            }`}
          >
            {applyMutation.isPending
              ? "Đang áp dụng..."
              : `Áp dụng cho ${selectedCount} thiết bị đã chọn`}
          </button>
        </div>
      </div>
    </div>
  );
}
