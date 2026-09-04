import { useState } from "react";
import { Play, Check, X } from "lucide-react";
import { useTestAutomationScript } from "../../../hooks/useAutomationScripts";
import type { AutomationIr } from "../../../lib/automation/ir";
import type { ConditionTraceEntry } from "../../../types/automation";

interface TestPanelProps {
  deviceId: string;
  ir: AutomationIr;
  fields: readonly string[];
}

function findFieldMode(ir: AutomationIr, field: string): string {
  const check = (item: any): string | null => {
    if (!item) return null;
    if (item.sensor === field && item.mode && item.mode !== 'instant') {
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
  return 'instant';
}

export function TestPanel({ deviceId, ir, fields }: TestPanelProps) {
  const [sampleRaw, setSampleRaw] = useState<Record<string, string>>({});
  const testMutation = useTestAutomationScript(deviceId);

  const handleRun = () => {
    const samplePayload: Record<string, number | number[]> = {};
    for (const field of fields) {
      const raw = sampleRaw[field];
      if (!raw || raw.trim() === '') continue;
      const mode = findFieldMode(ir, field);
      if (mode !== 'instant') {
        const parts = raw
          .split(',')
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
    <div className="flex h-full flex-col bg-white overflow-hidden shadow-xl sm:w-96 rounded-l-xl z-20 border-l border-emerald-100">
      <div className="flex items-center justify-between border-b px-4 py-3 bg-emerald-50">
        <h2 className="text-lg font-semibold text-emerald-900">
          Chạy thử (Dry Run)
        </h2>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-6">
        <div>
          <h3 className="text-sm font-medium text-emerald-950 mb-3">
            Giá trị mẫu (Input)
          </h3>
          <div className="space-y-3">
            {fields.map((field) => {
              const mode = findFieldMode(ir, field);
              const isWindow = mode !== 'instant';
              return (
                <div key={field} className="flex flex-col gap-1">
                  <div className="flex items-center justify-between">
                    <label className="text-sm font-medium text-emerald-900">
                      {field} {isWindow && <span className="text-xs text-emerald-600 font-normal">({mode})</span>}
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
                    <span className="text-[11px] text-emerald-800/60 text-right">
                      Nhập nhiều điểm, cách nhau bởi dấu phẩy
                    </span>
                  )}
                </div>
              );
            })}
          </div>
          <div className="rounded border border-sky-100 bg-sky-50 p-3 mt-4 mb-4 text-xs text-sky-800">
            <strong>Lưu ý: Đối với điều kiện time-window</strong>
            <br />
            Các điều kiện lấy mẫu theo thời gian (mean/min/max) nhận chuỗi số cách nhau bởi dấu phẩy để tính toán cửa sổ giả lập.
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
            <h3 className="text-sm font-medium text-emerald-950 mb-3">
              Kết quả (Output)
            </h3>

            <div
              className={`mb-4 p-3 rounded-lg flex items-center gap-2 font-medium ${
                testMutation.data.will_fire
                  ? "bg-emerald-50 text-emerald-700 border border-emerald-200"
                  : "bg-amber-50 text-amber-700 border border-amber-200"
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
              <h4 className="text-xs font-semibold text-emerald-800/70 uppercase">
                Trace Điều Kiện
              </h4>
              {testMutation.data.trace.map(
                (entry: ConditionTraceEntry, idx: number) => (
                  <div key={idx} className="flex items-start gap-2 text-sm">
                    {entry.passed ? (
                      <Check className="h-4 w-4 text-emerald-500 mt-0.5 shrink-0" />
                    ) : (
                      <X className="h-4 w-4 text-red-500 mt-0.5 shrink-0" />
                    )}
                    <div className="flex-1">
                      <div className="font-mono text-xs text-emerald-900">
                        {entry.description}
                      </div>
                      <div className="text-xs text-emerald-800/70">
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
                <div className="text-sm text-emerald-800/70 italic">
                  Không có điều kiện.
                </div>
              )}
            </div>

            <div className="space-y-2 mt-4">
              <h4 className="text-xs font-semibold text-emerald-800/70 uppercase">
                Actions Preview
              </h4>
              {testMutation.data.actions_preview.map(
                (action: Record<string, unknown>, idx: number) => (
                  <pre
                    key={idx}
                    className="text-xs font-mono bg-emerald-50/50 p-2 rounded border border-emerald-100 overflow-x-auto text-emerald-900"
                  >
                    {JSON.stringify(action, null, 2)}
                  </pre>
                ),
              )}
              {testMutation.data.actions_preview.length === 0 && (
                <div className="text-sm text-emerald-800/70 italic">
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
