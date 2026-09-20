import { useState } from "react";
import { Check, AlertTriangle } from "lucide-react";
import { useOwnedDevices } from "../../hooks/useOwnedDevices";
import { useAllConfigOverrides, useApplyTemplate } from "../../hooks/useAutomationScripts";
import type { UserScript } from "../../types/automation";

interface Props {
  currentScript: UserScript;
}

interface TargetDeviceMeta {
  id: string;
  name: string;
  hasLocalOverride: boolean;
  currentOverrideVal?: string;
}

export function AutomationMultiDeviceTemplatePanel({ currentScript }: Props) {
  const ownedRes = useOwnedDevices() as any;
  const rawDevices = ownedRes?.devices ?? ownedRes?.data ?? [];
  const applyMutation = useApplyTemplate(currentScript.device_id, currentScript.id);

  const targetConfigKey = currentScript.ir_json?.configOverwrite?.configKey;
  const targetConfigValue = currentScript.ir_json?.configOverwrite?.value;

  const candidateIds = (rawDevices ?? [])
    .map((d: any) => d.device_id || d.id)
    .filter((id: string) => id && id !== currentScript.device_id);
  const allOverrides = useAllConfigOverrides(candidateIds);

  // Real override data: only overrides matching this template's configKey count.
  const overrideByDeviceId = new Map(
    (allOverrides.data ?? [])
      .filter(
        (o) => o.deviceId !== currentScript.device_id && (!targetConfigKey || o.configKey === targetConfigKey),
      )
      .map((o) => [o.deviceId, o]),
  );

  // Map real devices (excluding current device — can't apply template to itself)
  const devices: TargetDeviceMeta[] = (rawDevices && rawDevices.length > 0)
    ? rawDevices
        .filter((d: any) => (d.device_id || d.id) !== currentScript.device_id)
        .map((d: any) => {
          const id = d.device_id || d.id;
          const override = overrideByDeviceId.get(id);
          return {
            id,
            name: d.label || d.name || `Thiết bị ${d.device_id || d.id}`,
            hasLocalOverride: override !== undefined,
            currentOverrideVal: override ? String(override.currentValue) : undefined,
          };
        })
      : [];


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
        overrides: d.hasLocalOverride ? { configOverwrite: null } : {},
      }));

    if (targets.length === 0) return;
    applyMutation.mutate(targets);
  };

  return (
    <div className="bg-white rounded-3xl border border-line p-6 shadow-sm space-y-6">
      {/* Header */}
      <div>
        <div className="flex items-center gap-2 mb-1">
          <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded-full bg-pill text-text-muted">
            TÍNH NĂNG MỚI
          </span>
        </div>
        <h2 className="text-xl font-bold text-primary-deep">
          Áp Flow template cho nhiều thiết bị
        </h2>
        <p className="text-xs text-text-muted/70 mt-1 max-w-4xl">
          Nhân bản một Flow (bao gồm cả node Đọc/Ghi đè Config) sang nhiều thiết bị cùng lúc — tự động phát hiện và giữ nguyên các thiết bị đang có cấu hình override cục bộ để tránh ghi đè ngoài ý muốn.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        {/* Left Column: Device Selection */}
        <div className="lg:col-span-7 space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold text-primary-deep">
              Chọn thiết bị đích ({devices.length} thiết bị)
            </span>
            <button
              type="button"
              onClick={toggleSelectAll}
              className="text-xs font-semibold text-status hover:text-primary-deep transition-colors cursor-pointer"
            >
              {allSelected ? "Bỏ chọn tất cả" : "Chọn tất cả"}
            </button>
          </div>

          <div className="space-y-2.5">
            {devices.length === 0 && (
              <div className="py-6 text-center text-xs text-text-muted bg-surface-muted rounded-2xl border border-dashed border-line p-4">
                Không tìm thấy thiết bị nào trong tài khoản để triển khai mẫu.
              </div>
            )}
            {devices.map((d) => {
              const isChecked = !!selectedIds[d.id];
              return (
                <div
                  key={d.id}
                  onClick={() => toggleDevice(d.id)}
                  className={`p-3.5 rounded-2xl border transition-all cursor-pointer flex items-center justify-between ${
                    isChecked
                      ? "border-line bg-pill/40 shadow-sm"
                      : "border-line/80 bg-white hover:border-line"
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <input
                      type="checkbox"
                      checked={isChecked}
                      onChange={() => toggleDevice(d.id)}
                      onClick={(e) => e.stopPropagation()}
                      className="w-4 h-4 rounded text-primary focus:ring-primary cursor-pointer"
                    />
                    <div>
                      <div className="text-xs font-bold text-primary-deep">{d.name}</div>
                    </div>
                  </div>

                  <div>
                    {d.hasLocalOverride ? (
                      <span className="px-2.5 py-1 rounded-lg text-[10px] font-bold bg-warning-bg text-warning border border-warning">
                        Có override cục bộ
                      </span>
                    ) : (
                      <span className="px-2.5 py-1 rounded-lg text-[10px] font-bold bg-pill/70 text-text-muted border border-line/70">
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
        <div className="lg:col-span-5 bg-config-soft/40 rounded-3xl border border-line p-5 space-y-4">
          <div>
            <div className="text-[10px] uppercase font-bold tracking-wider text-config mb-1">
              XEM TRƯỚC ẢNH HƯỞNG
            </div>
            <div className="text-2xl font-black text-primary-deep">
              {selectedCount} <span className="text-sm font-medium text-config/70">/ {devices.length} thiết bị sẽ áp dụng Flow này</span>
            </div>
          </div>

          <div className="bg-white rounded-2xl border border-line p-3.5 space-y-2 text-xs">
            <div className="font-semibold text-primary-deep">
              {targetConfigKey
                ? `${targetConfigKey} sẽ được ghi đè → ${targetConfigValue}`
                : "Flow này không chứa node Ghi đè Config — các thiết bị sẽ nhận toàn bộ Trigger/Condition/Action"}
            </div>
            <div className="text-[11px] text-status flex items-center gap-1.5">
              <Check className="w-3.5 h-3.5 shrink-0" />
              <span>{fullApplyCount} thiết bị: áp dụng đầy đủ Flow + Config Override</span>
            </div>
            {keepOverrideCount > 0 && (
              <div className="text-[11px] text-warning flex items-start gap-1.5">
                <AlertTriangle className="w-3.5 h-3.5 shrink-0 mt-0.5" />
                <span>{keepOverrideCount} thiết bị: giữ nguyên override cục bộ, chỉ nhận phần Trigger/Condition/Action</span>
              </div>
            )}
          </div>

          {/* Amber Safety Note */}
          <div className="bg-warning-bg/90 border border-warning rounded-2xl p-3 text-[11px] text-warn-deep leading-relaxed">
            <div className="font-bold text-warn-deep mb-1">Lưu ý an toàn</div>
            Thiết bị có override cục bộ sẽ được giữ nguyên cấu hình config hiện tại — chỉ Trigger/Condition/Action của Flow được đồng bộ, không ghi đè giá trị đã tùy chỉnh riêng.
          </div>

          {/* Per-device checklist */}
          <div className="space-y-1.5 pt-1">
            <div className="text-[10px] font-bold text-text-muted uppercase tracking-wider">
              THEO TỪNG THIẾT BỊ
            </div>
            <div className="space-y-1 text-xs">
              {devices.map((d) => {
                const isSelected = !!selectedIds[d.id];
                if (!isSelected) return null;
                return (
                  <div key={d.id} className="flex items-center gap-2 py-0.5">
                    {d.hasLocalOverride ? (
                      <AlertTriangle className="w-3.5 h-3.5 text-warning shrink-0" />
                    ) : (
                      <Check className="w-3.5 h-3.5 text-primary shrink-0" />
                    )}
                    <span className="font-medium text-text text-[11px]">{d.name}</span>
                    <span className="text-[10px] text-text-muted ml-auto">
                      {d.hasLocalOverride
                        ? `Giữ override cục bộ · ${targetConfigKey ?? "config"} hiện tại ${d.currentOverrideVal}`
                        : "Áp dụng đầy đủ"}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="text-[10px] text-text-muted pt-2 border-t border-line/60 leading-tight">
            Cả {selectedCount} lượt áp dụng (kể cả {keepOverrideCount} lượt giữ nguyên override) đều được ghi vào Nhật ký ghi đè toàn hệ thống.
          </div>

          {applyMutation.isSuccess && (
            <div className="p-2 text-xs bg-pill text-status border border-line rounded-xl">
              Áp dụng thành công cho {selectedCount} thiết bị!
            </div>
          )}

          <button
            type="button"
            disabled={selectedCount === 0 || applyMutation.isPending}
            onClick={handleApply}
            className={`w-full py-2.5 px-4 rounded-xl font-semibold text-xs text-white shadow-sm transition-all cursor-pointer ${
              selectedCount === 0 || applyMutation.isPending
                ? "bg-config opacity-50 cursor-not-allowed"
                : "bg-config hover:bg-config"
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
