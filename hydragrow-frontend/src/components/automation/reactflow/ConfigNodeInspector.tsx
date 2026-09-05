import { useState } from "react";
import { AlertTriangle, ShieldCheck, X, RefreshCw } from "lucide-react";
import { DEVICE_CONFIG_BOUNDS, clampConfigValue } from "../../../lib/automation/ir";
import { useDeviceStore } from "../../../store/useDeviceStore";
import type { ConfigAuditLogEntry } from "../../../types/automation";

interface Props {
  initialKey?: string;
  initialValue?: number;
  auditLogs?: ConfigAuditLogEntry[];
  onSave?: (data: { configKey: string; overrideValue: number; applyMode: string; autoRestore: boolean }) => void;
  onClose: () => void;
}

export function ConfigNodeInspector({
  initialKey = "ec_target",
  initialValue = 1.8,
  auditLogs = [],
  onSave,
  onClose,
}: Props) {
  const [configKey, setConfigKey] = useState(initialKey);
  const [overrideValue, setOverrideValue] = useState<number>(initialValue);
  const [applyMode, setApplyMode] = useState<string>("during_true");
  const [autoRestore, setAutoRestore] = useState(true);

  const settings = useDeviceStore((s) => s.settings);

  const bound = DEVICE_CONFIG_BOUNDS[configKey] ?? {
    min: 0.8,
    max: 3.2,
    unit: "mS/cm",
    label: configKey,
    sourceGroup: "Recipe hiện tại",
    defaultVal: 2.4,
  };

  const currentOriginalVal = (settings && typeof (settings as any)[configKey] === "number")
    ? (settings as any)[configKey]
    : bound.defaultVal;


  const handleValueChange = (valStr: string) => {
    const n = parseFloat(valStr);
    if (!isNaN(n)) {
      setOverrideValue(n);
    }
  };

  const { value: clampedVal, clamped } = clampConfigValue(configKey, overrideValue);

  const percentage = Math.max(
    0,
    Math.min(100, ((clampedVal - bound.min) / (bound.max - bound.min)) * 100),
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div className="bg-white rounded-3xl shadow-2xl border border-emerald-100 max-w-5xl w-full max-h-[92vh] overflow-y-auto p-6 space-y-6">
        {/* Header */}
        <div className="flex items-start justify-between border-b border-emerald-50 pb-4">
          <div>
            <div className="flex items-center gap-2 mb-1">
              <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded-md bg-indigo-100 text-indigo-800">
                NODE MỚI
              </span>
              <span className="text-xs text-emerald-800/60 font-medium">
                Panel chi tiết trong Flow Editor · Thay thế NodeEditorPanel khi chọn node Config
              </span>
            </div>
            <h2 className="text-xl font-bold text-emerald-950">
              Đọc & Ghi đè Config theo điều kiện
            </h2>
            <p className="text-xs text-emerald-800/70 mt-1">
              Cho phép Flow đọc giá trị cấu hình hiện tại của thiết bị, và ghi đè có kiểm soát khi điều kiện của Flow đúng — có giới hạn an toàn, chế độ áp dụng, và tự động khôi phục.
            </p>
          </div>

          <button
            type="button"
            onClick={onClose}
            className="p-1.5 rounded-xl text-emerald-800/60 hover:text-emerald-950 hover:bg-emerald-50 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* 3 Panels */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
          {/* Panel 1: Đọc Config */}
          <div className="bg-emerald-50/20 rounded-2xl border border-emerald-100 p-4 flex flex-col justify-between">
            <div>
              <div className="flex items-center justify-between mb-3">
                <span className="text-xs font-bold text-emerald-950 flex items-center gap-1.5">
                  (1) Đọc Config
                </span>
                <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-indigo-100 text-indigo-800">
                  NGUỒN DỮ LIỆU
                </span>
              </div>

              <div className="space-y-3">
                <div>
                  <label className="text-[11px] font-semibold text-emerald-900/80 block mb-1 uppercase tracking-wider">
                    CONFIG KEY
                  </label>
                  <select
                    value={configKey}
                    onChange={(e) => setConfigKey(e.target.value)}
                    className="ui-input text-xs w-full font-mono font-medium"
                  >
                    {Object.keys(DEVICE_CONFIG_BOUNDS).map((k) => (
                      <option key={k} value={k}>
                        {k}
                      </option>
                    ))}
                  </select>
                </div>

                <div className="text-[11px] text-emerald-800/70">
                  Thuộc nhóm: <span className="font-medium text-emerald-950">{bound.sourceGroup}</span>
                </div>

                <div className="bg-white rounded-xl p-3 border border-emerald-100">
                  <div className="text-[10px] font-semibold text-emerald-800/60 uppercase">
                    GIÁ TRỊ HIỆN TẠI (LIVE)
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-bold text-emerald-950">
                      {currentOriginalVal} {bound.unit}
                    </span>
                    <span className="text-[11px] text-emerald-700 bg-emerald-50 px-2 py-0.5 rounded">
                      nguồn: {settings ? "Cấu hình thiết bị" : "Mặc định"}
                    </span>
                  </div>
                </div>

                <div className="bg-white rounded-xl p-3 border border-emerald-100">
                  <div className="text-[10px] font-semibold text-emerald-800/60 uppercase mb-1">
                    GIỚI HẠN CHO PHÉP (TỪ SCHEMA THIẾT BỊ)
                  </div>
                  <div className="flex items-center justify-between text-xs font-medium text-emerald-950">
                    <span>Min {bound.min}</span>
                    <span>Max {bound.max} {bound.unit}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Panel 2: Điều kiện áp dụng */}
          <div className="bg-emerald-50/20 rounded-2xl border border-emerald-100 p-4 flex flex-col justify-between">
            <div>
              <div className="flex items-center justify-between mb-3">
                <span className="text-xs font-bold text-emerald-950">
                  (2) Điều kiện áp dụng
                </span>
                <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-amber-100 text-amber-800">
                  DÙNG CHUNG FLOW
                </span>
              </div>

              <div className="bg-amber-50/60 border border-amber-200/80 rounded-xl p-3 mb-4">
                <div className="font-semibold text-xs text-amber-950">Khung giờ ban đêm</div>
                <div className="text-[11px] text-amber-900/80 font-mono mt-0.5">
                  time.hour &isin; [22,24) &cup; [0,5)
                </div>
              </div>

              <div className="space-y-2">
                <label className="text-[11px] font-semibold text-emerald-900/80 block uppercase tracking-wider mb-1">
                  ÁP DỤNG GHI ĐÈ KHI
                </label>

                <label className="flex items-center gap-2.5 p-2 rounded-xl border border-emerald-100/80 bg-white text-xs text-emerald-950 cursor-pointer hover:bg-emerald-50/40">
                  <input
                    type="radio"
                    name="applyMode"
                    value="once"
                    checked={applyMode === "once"}
                    onChange={(e) => setApplyMode(e.target.value)}
                    className="text-emerald-600"
                  />
                  <span>Điều kiện vừa chuyển sang đúng (once)</span>
                </label>

                <label className="flex items-center gap-2.5 p-2 rounded-xl border border-emerald-200 bg-emerald-50/40 text-xs font-medium text-emerald-950 cursor-pointer">
                  <input
                    type="radio"
                    name="applyMode"
                    value="during_true"
                    checked={applyMode === "during_true"}
                    onChange={(e) => setApplyMode(e.target.value)}
                    className="text-emerald-600"
                  />
                  <span>Trong suốt thời gian điều kiện đúng</span>
                </label>

                <label className="flex items-center gap-2.5 p-2 rounded-xl border border-emerald-100/80 bg-white text-xs text-emerald-950 cursor-pointer hover:bg-emerald-50/40">
                  <input
                    type="radio"
                    name="applyMode"
                    value="until_next_flow"
                    checked={applyMode === "until_next_flow"}
                    onChange={(e) => setApplyMode(e.target.value)}
                    className="text-emerald-600"
                  />
                  <span>Cho đến khi có Flow khác thay đổi</span>
                </label>
              </div>

              <div className="mt-4 pt-3 border-t border-emerald-100">
                <label className="text-[11px] font-semibold text-emerald-900/80 block uppercase tracking-wider mb-2">
                  KHI ĐIỀU KIỆN SAI
                </label>
                <label className="flex items-center gap-2.5 p-2 rounded-xl bg-white border border-emerald-100 text-xs text-emerald-950 cursor-pointer hover:bg-emerald-50/40">
                  <input
                    type="checkbox"
                    checked={autoRestore}
                    onChange={(e) => setAutoRestore(e.target.checked)}
                    className="text-emerald-600 rounded"
                  />
                  <span>Tự động khôi phục giá trị gốc ({bound.defaultVal} {bound.unit})</span>
                </label>
              </div>
            </div>
          </div>

          {/* Panel 3: Ghi đè giá trị */}
          <div className="bg-emerald-50/20 rounded-2xl border border-emerald-100 p-4 flex flex-col justify-between">
            <div>
              <div className="flex items-center justify-between mb-3">
                <span className="text-xs font-bold text-emerald-950">
                  (3) Ghi đè giá trị
                </span>
                <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-sky-100 text-sky-800">
                  AN TOÀN
                </span>
              </div>

              <div>
                <label className="text-[11px] font-semibold text-emerald-900/80 block uppercase tracking-wider mb-1">
                  GIÁ TRỊ GHI ĐÈ
                </label>
                <div className="relative">
                  <input
                    type="number"
                    step={bound.step}
                    value={overrideValue}
                    onChange={(e) => handleValueChange(e.target.value)}
                    className="ui-input text-lg font-bold text-indigo-900 w-full pr-14"
                  />
                  <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs font-semibold text-slate-500">
                    {bound.unit}
                  </span>
                </div>

                {clamped && (
                  <div className="mt-1.5 flex items-center gap-1.5 text-[11px] text-amber-700 bg-amber-50 p-2 rounded-lg border border-amber-200">
                    <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                    <span>Giá trị vượt cận biên! Tự động kẹp về {clampedVal} {bound.unit}</span>
                  </div>
                )}

                {/* Range bar */}
                <div className="mt-4">
                  <div className="w-full bg-slate-200 rounded-full h-2 overflow-hidden flex">
                    <div
                      className="bg-indigo-600 h-2 rounded-full transition-all duration-300"
                      style={{ width: `${percentage}%` }}
                    />
                  </div>
                  <div className="text-[11px] text-emerald-800/60 mt-1.5">
                    Khoảng cho phép: {bound.min} – {bound.max} {bound.unit} (kẹp cứng theo schema thiết bị)
                  </div>
                </div>

                {/* Conflict warning */}
                <div className="mt-4 bg-amber-50 border border-amber-200/80 rounded-xl p-3 text-[11px] text-amber-950 leading-relaxed">
                  <div className="font-bold flex items-center gap-1.5 text-amber-800 mb-1">
                    <AlertTriangle className="w-3.5 h-3.5" />
                    CẢNH BÁO AN TOÀN
                  </div>
                  Nếu 2 Flow cùng ghi đè 1 config key, Flow ưu tiên cao hơn (theo thứ tự trong danh sách) sẽ thắng — xung đột được ghi vào nhật ký.
                </div>
              </div>
            </div>

            <button
              type="button"
              onClick={() => {
                onSave?.({
                  configKey,
                  overrideValue: clampedVal,
                  applyMode,
                  autoRestore,
                });
                onClose();
              }}
              className="mt-4 w-full ui-btn-primary py-2 text-xs font-semibold"
            >
              Lưu cấu hình Node
            </button>
          </div>
        </div>

        {/* Panel 4: Nhật ký ghi đè của Node */}
        <div className="border-t border-emerald-50 pt-5">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-xs font-bold text-emerald-950 flex items-center gap-2">
              (4) Nhật ký ghi đè (audit log) — minh bạch & có thể truy vết
              <span className="bg-emerald-600 text-white text-[9px] font-semibold px-1.5 py-0.2 rounded">
                MỚI
              </span>
            </h3>
          </div>

          <div className="overflow-x-auto rounded-xl border border-emerald-100">
            <table className="w-full text-left text-xs">
              <thead className="bg-emerald-50/40 text-slate-500 font-semibold uppercase text-[10px]">
                <tr>
                  <th className="py-2.5 px-3">THỜI GIAN</th>
                  <th className="py-2.5 px-3">GIÁ TRỊ GỐC</th>
                  <th className="py-2.5 px-3">GIÁ TRỊ GHI ĐÈ</th>
                  <th className="py-2.5 px-3">LÝ DO KÍCH HOẠT</th>
                  <th className="py-2.5 px-3">TRẠNG THÁI</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-emerald-50 text-[11px]">
                {((auditLogs ?? []).filter((l) => l.configKey === configKey)).length === 0 ? (
                  <tr>
                    <td colSpan={5} className="py-6 text-center text-slate-500">
                      Chưa có nhật ký ghi đè nào cho tham số này. Nhật ký sẽ được ghi nhận khi Flow kích hoạt.
                    </td>
                  </tr>
                ) : (
                  (auditLogs ?? [])
                    .filter((l) => l.configKey === configKey)
                    .map((log) => (
                      <tr key={log.id} className="hover:bg-emerald-50/20">
                        <td className="py-2 px-3 font-mono text-slate-500">{log.timestamp}</td>
                        <td className="py-2 px-3 text-slate-600">{log.originalValue} {bound.unit}</td>
                        <td className="py-2 px-3 font-bold text-indigo-700">{log.overrideValue} {bound.unit}</td>
                        <td className="py-2 px-3 text-slate-700">{log.reason}</td>
                        <td className="py-2 px-3">
                          {log.status === "applied" && (
                            <span className="text-emerald-700 font-semibold inline-flex items-center gap-1">
                              <ShieldCheck className="w-3.5 h-3.5" /> Đã áp dụng
                            </span>
                          )}
                          {log.status === "restored" && (
                            <span className="text-sky-700 font-semibold inline-flex items-center gap-1">
                              <RefreshCw className="w-3 h-3" /> Đã khôi phục
                            </span>
                          )}
                          {log.status === "clamped_warning" && (
                            <span className="text-amber-700 font-semibold inline-flex items-center gap-1">
                              <AlertTriangle className="w-3.5 h-3.5" /> Cảnh báo - Đã kẹp
                            </span>
                          )}
                        </td>
                      </tr>
                    ))
                )}
              </tbody>

            </table>
          </div>
        </div>
      </div>
    </div>
  );
}
