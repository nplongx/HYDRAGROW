import { useState } from "react";
import { Play, Check, X } from "lucide-react";
import { useTestAutomationScript } from "../../../hooks/useAutomationScripts";
import {
  DEVICE_CONFIG_BOUNDS,
  clampConfigValue,
  type AutomationIr,
} from "../../../lib/automation/ir";
import { useDeviceConfig } from "../../../hooks/useDeviceConfig";
import { useDeviceTelemetry } from "../../../hooks/useDeviceTelemetry";
import type { ConditionTraceEntry } from "../../../types/automation";

interface TestPanelProps {
  deviceId: string;
  ir: AutomationIr;
  fields: readonly string[];
}

function findFieldMode(ir: AutomationIr, field: string): string {
  const check = (item: any): string | null => {
    if (!item) return null;
    if (item.sensor === field && item.mode && item.mode !== "instant") {
      return item.mode;
    }
    if (item.children && Array.isArray(item.children)) {
      for (const child of item.children) {
        const res = check(child);
        if (res) return res;
      }
    }
    return null;
  };
  if (ir.conditions && Array.isArray(ir.conditions)) {
    for (const cond of ir.conditions) {
      const res = check(cond);
      if (res) return res;
    }
  }
  return "instant";
}

export function TestPanel({ deviceId, ir, fields }: TestPanelProps) {
  const [sampleRaw, setSampleRaw] = useState<Record<string, string>>({});
  const testMutation = useTestAutomationScript(deviceId);
  const { data: settings } = useDeviceConfig(deviceId);
  const { data: telemetry } = useDeviceTelemetry(deviceId);

  const targetKey = (ir.configOverwrite?.configKey ??
    (ir.actions?.find((a) => a.type === "config_override") as any)?.key ??
    "ec_target") as string;
  const bound = DEVICE_CONFIG_BOUNDS[targetKey] ?? {
    min: 0.8,
    max: 3.2,
    unit: "mS/cm",
    defaultVal: 2.4,
  };
  const actualBeforeVal =
    settings && typeof (settings as any)[targetKey] === "number"
      ? (settings as any)[targetKey]
      : bound.defaultVal;
  const overrideVal =
    ir.configOverwrite?.value ??
    (ir.actions?.find((a) => a.type === "config_override") as any)?.value ??
    1.8;

  const handleRun = () => {
    const samplePayload: Record<string, number | number[]> = {};
    for (const field of fields) {
      const raw = sampleRaw[field];
      if (!raw || raw.trim() === "") continue;
      const mode = findFieldMode(ir, field);
      if (mode !== "instant") {
        const parts = raw
          .split(",")
          .map((s) => parseFloat(s.trim()))
          .filter((n) => !isNaN(n));
        if (parts.length > 0) {
          samplePayload[field] = parts;
        }
      } else {
        const num = parseFloat(raw.trim());
        if (!isNaN(num)) {
          samplePayload[field] = num;
        }
      }
    }
    testMutation.mutate({ ir_json: ir, sample: samplePayload });
  };

  const handleFieldChange = (field: string, value: string) => {
    setSampleRaw((prev) => ({
      ...prev,
      [field]: value,
    }));
  };

  return (
    <div className="flex h-full flex-col bg-white overflow-hidden shadow-xl sm:w-96 rounded-l-xl z-20 border-l border-line">
      <div className="flex items-center justify-between border-b px-4 py-3 bg-pill">
        <h2 className="text-lg font-semibold text-primary-deep">
          Chạy thử (Dry Run)
        </h2>
      </div>
      <div className="mx-4 mt-3 rounded-xl border border-primary bg-info-bg px-3 py-2 text-xs font-semibold text-primary">
        Mô phỏng — không gửi lệnh
      </div>
      <div className="mx-4 mt-3 rounded-xl border border-line bg-white p-3 text-xs">
        <div className="font-semibold text-primary-deep">Giá trị hiện tại</div>
        <div className="mt-2 grid grid-cols-2 gap-2 text-text-muted">
          {(telemetry?.axes ?? []).slice(0, 4).map((axis) => (
            <div key={axis.name} className="flex justify-between gap-2">
              <span>{axis.name}</span>
              <span className="font-mono text-primary-deep">{axis.value ?? "Unknown"}</span>
            </div>
          ))}
          {!telemetry && <span className="col-span-2 text-faint">Chưa có telemetry authoritative; giữ Unknown.</span>}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-6">
        <div>
          <h3 className="text-sm font-medium text-primary-deep mb-3">
            Giá trị mô phỏng (Input)
          </h3>
          <div className="space-y-3">
            {fields.map((field) => {
              const mode = findFieldMode(ir, field);
              const isWindow = mode !== "instant";
              return (
                <div key={field} className="flex flex-col gap-1">
                  <div className="flex items-center justify-between">
                    <label className="text-sm font-medium text-primary-deep">
                      {field}{" "}
                      {isWindow && (
                        <span className="text-xs text-primary font-normal">
                          ({mode})
                        </span>
                      )}
                    </label>
                    <input
                      type={isWindow ? "text" : "number"}
                      step={isWindow ? undefined : "0.01"}
                      className={`ui-input ${isWindow ? "w-44 text-left" : "w-24 text-right"}`}
                      value={sampleRaw[field] ?? ""}
                      onChange={(e) => handleFieldChange(field, e.target.value)}
                      placeholder={isWindow ? "vd: 7.0, 7.5, 8.5" : "0.0"}
                    />
                  </div>
                  {isWindow && (
                    <span className="text-[11px] text-text-muted/60 text-right">
                      Nhập nhiều điểm, cách nhau bởi dấu phẩy
                    </span>
                  )}
                </div>
              );
            })}
          </div>
          <div className="rounded border border-line bg-info-bg p-3 mt-4 mb-4 text-xs text-info">
            <strong>Lưu ý: Đối với điều kiện time-window</strong>
            <br />
            Các điều kiện lấy mẫu theo thời gian (mean/min/max) nhận chuỗi số
            cách nhau bởi dấu phẩy để tính toán cửa sổ giả lập.
          </div>
          <button
            type="button"
            className="ui-btn-primary mt-4 w-full flex justify-center items-center gap-2"
            onClick={handleRun}
            disabled={testMutation.isPending}
          >
            <Play className="h-4 w-4" />
            {testMutation.isPending ? "Đang chạy..." : "Chạy thử"}
          </button>
        </div>

        {testMutation.data && (
          <div className="border-t pt-4">
            <h3 className="text-sm font-medium text-primary-deep mb-3">
              Kết quả (Output)
            </h3>

            <div
              className={`mb-4 p-3 rounded-lg flex items-center gap-2 font-medium ${
                testMutation.data.will_fire
                  ? "bg-pill text-status border border-line"
                  : "bg-warning-bg text-warning border border-warning"
              }`}
            >
              {testMutation.data.will_fire ? (
                <>
                  <Check className="h-5 w-5" />
                  Flow SẼ kích hoạt
                </>
              ) : (
                <>
                  <X className="h-5 w-5" />
                  Flow SẼ KHÔNG kích hoạt
                </>
              )}
            </div>

            <div className="space-y-2">
              <h4 className="text-xs font-semibold text-text-muted/70 uppercase">
                Trace Điều Kiện
              </h4>
              {testMutation.data.trace.map(
                (entry: ConditionTraceEntry, idx: number) => (
                  <div key={idx} className="flex items-start gap-2 text-sm">
                    {entry.passed ? (
                      <Check className="h-4 w-4 text-primary mt-0.5 shrink-0" />
                    ) : (
                      <X className="h-4 w-4 text-error mt-0.5 shrink-0" />
                    )}
                    <div className="flex-1">
                      <div className="font-mono text-xs text-primary-deep">
                        {entry.description}
                      </div>
                      <div className="text-xs text-text-muted/70">
                        Actual:{" "}
                        {entry.actual_value !== null
                          ? entry.actual_value
                          : "null"}
                      </div>
                    </div>
                  </div>
                ),
              )}
              {testMutation.data.trace.length === 0 && (
                <div className="text-sm text-text-muted/70 italic">
                  Không có điều kiện.
                </div>
              )}
            </div>

            {/* Config Diff Comparison */}
            {testMutation.data.will_fire &&
              (ir.configOverwrite ||
                ir.actions.some((a) => a.type === "config_override")) && (
                <div className="space-y-2 mt-4">
                  <div className="flex items-center justify-between">
                    <h4 className="text-xs font-semibold text-config uppercase">
                      SO SÁNH CONFIG (DIFF)
                    </h4>
                    <span className="bg-config text-white text-[9px] font-semibold px-1 rounded">
                      MỚI
                    </span>
                  </div>
                  <div className="rounded-xl border border-line bg-config-soft/40 p-3 text-xs space-y-2">
                    <div className="flex items-center justify-between text-[11px] text-text-muted font-mono">
                      <span>CONFIG KEY</span>
                      <span className="font-bold text-primary-deep">
                        {targetKey}
                      </span>
                    </div>
                    <div className="flex items-baseline justify-between pt-1 border-t border-line/80">
                      <div>
                        <span className="text-[10px] text-text-muted block uppercase">
                          TRƯỚC
                        </span>
                        <span className="line-through text-text-muted font-medium">
                          {actualBeforeVal} {bound.unit}
                        </span>
                      </div>
                      <span className="text-config font-bold">&rarr;</span>
                      <div className="text-right">
                        <span className="text-[10px] text-config block uppercase font-semibold">
                          SAU KHI GHI ĐÈ
                        </span>
                        <span className="text-sm font-bold text-config">
                          {overrideVal} {bound.unit}
                        </span>
                      </div>
                    </div>
                    {(() => {
                      const parsed = parseFloat(String(overrideVal));
                      const clamp = clampConfigValue(
                        targetKey,
                        Number.isNaN(parsed) ? bound.defaultVal : parsed,
                      );
                      return clamp.clamped ? (
                        <div className="text-[11px] text-warning flex items-center gap-1 font-medium">
                          <X className="w-3.5 h-3.5" />
                          <span>
                            Vượt giới hạn — giá trị sẽ bị kẹp (clamp) về{" "}
                            {clamp.value} {bound.unit} (cho phép {bound.min} –{" "}
                            {bound.max} {bound.unit})
                          </span>
                        </div>
                      ) : (
                        <div className="text-[11px] text-status flex items-center gap-1 font-medium">
                          <Check className="w-3.5 h-3.5" />
                          <span>
                            Trong giới hạn cho phép ({bound.min} – {bound.max}{" "}
                            {bound.unit})
                          </span>
                        </div>
                      );
                    })()}
                    <div className="text-[11px] text-text-muted">
                      &circlearrowright; Tự động khôi phục {actualBeforeVal}{" "}
                      {bound.unit} khi điều kiện hết đúng
                    </div>
                  </div>
                </div>
              )}

            <div className="space-y-2 mt-4">
              <h4 className="text-xs font-semibold text-text-muted/70 uppercase">
                Actions Preview
              </h4>
              {testMutation.data.actions_preview.map(
                (action: Record<string, unknown>, idx: number) => (
                  <pre
                    key={idx}
                    className="text-xs font-mono bg-pill/50 p-2 rounded border border-line overflow-x-auto text-primary-deep"
                  >
                    {JSON.stringify(action, null, 2)}
                  </pre>
                ),
              )}
              {testMutation.data.actions_preview.length === 0 && (
                <div className="text-sm text-text-muted/70 italic">
                  Không có hành động (hoặc điều kiện không thỏa).
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
