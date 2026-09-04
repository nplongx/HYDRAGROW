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

export function TestPanel({ deviceId, ir, fields }: TestPanelProps) {
  const [sample, setSample] = useState<Record<string, number>>({});
  const testMutation = useTestAutomationScript(deviceId);

  const handleRun = () => {
    testMutation.mutate({ ir_json: ir, sample });
  };

  const handleFieldChange = (field: string, value: string) => {
    const num = parseFloat(value);
    setSample((prev) => {
      const next = { ...prev };
      if (isNaN(num)) {
        delete next[field];
      } else {
        next[field] = num;
      }
      return next;
    });
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
          <h3 className="text-sm font-medium text-slate-700 mb-3">
            Giá trị mẫu (Input)
          </h3>
          <div className="space-y-3">
            {fields.map((field) => (
              <div key={field} className="flex items-center justify-between">
                <label className="text-sm font-medium text-slate-600">
                  {field}
                </label>
                <input
                  type="number"
                  step="0.01"
                  className="ui-input w-24 text-right"
                  value={sample[field] ?? ""}
                  onChange={(e) => handleFieldChange(field, e.target.value)}
                  placeholder="0.0"
                />
              </div>
            ))}
          </div>
          <div className="rounded border border-sky-100 bg-sky-50 p-3 mt-4 mb-4 text-xs text-sky-800">
            <strong>Lưu ý: Đối với điều kiện time-window</strong>
            <br />
            Các điều kiện lấy mẫu theo thời gian (mean/min/max) sẽ được giả lập
            với giá trị mẫu này làm giá trị duy nhất trong cửa sổ.
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
            <h3 className="text-sm font-medium text-slate-700 mb-3">
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
              <h4 className="text-xs font-semibold text-slate-500 uppercase">
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
                      <div className="font-mono text-xs text-slate-700">
                        {entry.description}
                      </div>
                      <div className="text-xs text-slate-500">
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
                <div className="text-sm text-slate-500 italic">
                  Không có điều kiện.
                </div>
              )}
            </div>

            <div className="space-y-2 mt-4">
              <h4 className="text-xs font-semibold text-slate-500 uppercase">
                Actions Preview
              </h4>
              {testMutation.data.actions_preview.map(
                (action: Record<string, unknown>, idx: number) => (
                  <pre
                    key={idx}
                    className="text-xs font-mono bg-slate-50 p-2 rounded border border-slate-100 overflow-x-auto text-slate-700"
                  >
                    {JSON.stringify(action, null, 2)}
                  </pre>
                ),
              )}
              {testMutation.data.actions_preview.length === 0 && (
                <div className="text-sm text-slate-500 italic">
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
