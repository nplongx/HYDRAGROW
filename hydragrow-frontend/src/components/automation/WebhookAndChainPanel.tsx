import { useState } from "react";
import { Copy, Check, AlertTriangle, ArrowRight, Plus } from "lucide-react";
import toast from "react-hot-toast";
import type { UserScript } from "../../types/automation";

interface FieldMapping {
  bodyPath: string;
  targetField: string;
  isConfig?: boolean;
}

interface Props {
  currentScriptId?: string;
  scripts: UserScript[];
  selectedNextFlowIds: string[];
  onToggleNextFlow: (id: string) => void;
  webhookUrl?: string;
}

const DEFAULT_MAPPINGS: FieldMapping[] = [
  { bodyPath: "body.ph", targetField: "ph" },
  { bodyPath: "body.ec", targetField: "ec" },
  { bodyPath: "body.sensors.water_level", targetField: "water_level" },
  { bodyPath: "body.night_ec_target", targetField: "ec_target", isConfig: true },
];

export function WebhookAndChainPanel({
  currentScriptId = "curr-flow",
  scripts,
  selectedNextFlowIds,
  onToggleNextFlow,
  webhookUrl,
}: Props) {
  const [mode, setMode] = useState<"flow" | "direct">("flow");
  const [mappings, setMappings] = useState<FieldMapping[]>(DEFAULT_MAPPINGS);
  const [newBodyPath, setNewBodyPath] = useState("");
  const [newTarget, setNewTarget] = useState("");
  const [isNewConfig, setIsNewConfig] = useState(false);
  const [copied, setCopied] = useState(false);

  const fallbackWebhookUrl = typeof window !== "undefined"
    ? `${window.location.origin}/api/webhook/devices/${scripts[0]?.device_id || "device"}/flow-event`
    : "https://api.hydragrow.app/api/webhook/devices/device/flow-event";
  const effectiveWebhookUrl = webhookUrl ?? fallbackWebhookUrl;

  const handleCopy = () => {
    navigator.clipboard.writeText(effectiveWebhookUrl);
    setCopied(true);
    toast.success("Đã sao chép Webhook URL!");
    setTimeout(() => setCopied(false), 2000);
  };

  const handleAddMapping = () => {
    if (!newBodyPath.trim() || !newTarget.trim()) return;
    setMappings((prev) => [
      ...prev,
      { bodyPath: newBodyPath.trim(), targetField: newTarget.trim(), isConfig: isNewConfig },
    ]);
    setNewBodyPath("");
    setNewTarget("");
    setIsNewConfig(false);
  };

  const candidateFlows = scripts;


  return (
    <div className="bg-white rounded-3xl border border-emerald-100 p-6 shadow-sm space-y-6">
      {/* Header */}
      <div>
        <div className="flex items-center gap-2 mb-1">
          <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded-full bg-indigo-100 text-indigo-800">
            NODE CẤU HÌNH
          </span>
        </div>
        <h2 className="text-xl font-bold text-emerald-950">
          Webhook & Chuỗi Flow kế tiếp
        </h2>
        <p className="text-xs text-emerald-800/70 mt-1 max-w-4xl leading-relaxed">
          Cấu hình cách Flow xử lý dữ liệu Webhook đến (ánh xạ trường hoặc gọi lệnh trực tiếp), và chọn các Flow sẽ tự động chạy tiếp theo sau khi Flow này hoàn tất — có kiểm tra vòng lặp.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        {/* Left Column: Inbound Webhook */}
        <div className="lg:col-span-6 space-y-4">
          <div className="text-xs font-bold text-emerald-950 uppercase tracking-wider">
            Webhook đến (Inbound)
          </div>

          <div className="space-y-1">
            <label className="text-[11px] font-semibold text-emerald-900/80 uppercase">
              WEBHOOK URL
            </label>
            <div className="flex items-center gap-2 bg-slate-50 border border-slate-200 rounded-xl p-2.5">
              <input
                type="text"
                readOnly
                value={webhookUrl}
                className="bg-transparent text-xs font-mono text-slate-700 flex-1 outline-none select-all"
              />
              <button
                type="button"
                onClick={handleCopy}
                className="inline-flex items-center gap-1 text-xs font-semibold text-emerald-700 hover:text-emerald-900 bg-white border border-emerald-200 rounded-lg px-2 py-1 shadow-2xs transition-colors cursor-pointer"
              >
                {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                <span>{copied ? "Đã chép" : "Sao chép"}</span>
              </button>
            </div>
            <div className="text-[10px] text-slate-400 mt-1">
              Token bí mật chỉ hiển thị đầy đủ một lần lúc tạo — hiện tại: <span className="font-mono">********3f21</span>
            </div>
          </div>

          {/* Mode Selection */}
          <div className="space-y-1">
            <label className="text-[11px] font-semibold text-emerald-900/80 uppercase">
              CHẾ ĐỘ XỬ LÝ
            </label>
            <div className="grid grid-cols-2 gap-3">
              <label
                className={`p-3 rounded-2xl border text-xs cursor-pointer transition-all ${
                  mode === "flow"
                    ? "border-emerald-500 bg-emerald-50/40 text-emerald-950 font-medium"
                    : "border-slate-200 text-slate-600 bg-white"
                }`}
              >
                <div className="flex items-center gap-2 font-bold mb-1">
                  <input
                    type="radio"
                    name="mode"
                    checked={mode === "flow"}
                    onChange={() => setMode("flow")}
                    className="text-emerald-600"
                  />
                  <span>Chạy qua Flow</span>
                </div>
                <p className="text-[11px] text-slate-500 leading-tight">
                  Payload được ánh xạ vào các trường (ph, ec, config...) rồi chạy toàn bộ Condition/Action như một Flow bình thường.
                </p>
              </label>

              <label
                className={`p-3 rounded-2xl border text-xs cursor-pointer transition-all ${
                  mode === "direct"
                    ? "border-emerald-500 bg-emerald-50/40 text-emerald-950 font-medium"
                    : "border-slate-200 text-slate-600 bg-white"
                }`}
              >
                <div className="flex items-center gap-2 font-bold mb-1">
                  <input
                    type="radio"
                    name="mode"
                    checked={mode === "direct"}
                    onChange={() => setMode("direct")}
                    className="text-emerald-600"
                  />
                  <span>Gọi lệnh trực tiếp</span>
                </div>
                <p className="text-[11px] text-slate-500 leading-tight">
                  Bỏ qua Condition — thực thi ngay Action đầu tiên khi Webhook tới. Dùng cho nút dừng khẩn cấp ngoài hệ thống.
                </p>
              </label>
            </div>
          </div>

          {/* Field Mapping */}
          <div className="space-y-2">
            <label className="text-[11px] font-semibold text-emerald-900/80 uppercase">
              ÁNH XẠ DỮ LIỆU (FIELD MAPPING)
            </label>

            <div className="space-y-1.5">
              {mappings.map((m, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between p-2.5 rounded-xl border border-slate-200 bg-slate-50/50 text-xs font-mono"
                >
                  <span className="text-slate-800">{m.bodyPath}</span>
                  <span className="text-slate-400">&rarr;</span>
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-emerald-900">{m.targetField}</span>
                    {m.isConfig && (
                      <span className="text-[9px] font-bold px-1.5 py-0.2 rounded bg-indigo-100 text-indigo-800 font-sans">
                        CONFIG
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>

            {/* Add Mapping Row */}
            <div className="flex items-center gap-2 pt-2">
              <input
                type="text"
                placeholder="body.custom_path"
                value={newBodyPath}
                onChange={(e) => setNewBodyPath(e.target.value)}
                className="ui-input text-xs flex-1"
              />
              <input
                type="text"
                placeholder="target_var"
                value={newTarget}
                onChange={(e) => setNewTarget(e.target.value)}
                className="ui-input text-xs flex-1"
              />
              <label className="flex items-center gap-1 text-[11px] text-slate-600 cursor-pointer">
                <input
                  type="checkbox"
                  checked={isNewConfig}
                  onChange={(e) => setIsNewConfig(e.target.checked)}
                  className="rounded text-indigo-600"
                />
                <span>Config</span>
              </label>
              <button
                type="button"
                onClick={handleAddMapping}
                className="p-2 rounded-xl bg-emerald-600 text-white hover:bg-emerald-700 transition-colors cursor-pointer"
              >
                <Plus className="w-4 h-4" />
              </button>
            </div>

            <div className="bg-emerald-50/50 rounded-xl p-3 text-[11px] text-emerald-900 border border-emerald-100/80 leading-relaxed">
              Chỉ áp dụng khi Chế độ = "Chạy qua Flow". Ánh xạ tới trường CONFIG sẽ đi thẳng vào node Ghi đè Config — vẫn tuân theo giới hạn Min/Max. Trường không khớp sẽ bị bỏ qua và ghi log cảnh báo.
            </div>
          </div>
        </div>

        {/* Right Column: Flow Chaining */}
        <div className="lg:col-span-6 space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold text-emerald-950 uppercase tracking-wider">
              CHUỖI FLOW KẾ TIẾP
            </span>
            <span className="bg-indigo-600 text-white text-[9px] font-semibold px-1.5 py-0.2 rounded">
              MỚI
            </span>
          </div>

          <div className="space-y-2">
            {/* Current flow node card */}
            <div className="p-3 rounded-2xl border border-indigo-200 bg-indigo-50/40 text-xs flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-[10px] font-bold px-1.5 py-0.2 rounded bg-amber-100 text-amber-800 uppercase">
                  ALERT
                </span>
                <span className="font-semibold text-indigo-950">Webhook: Cảnh báo bơm ngoài (Flow hiện tại)</span>
              </div>
            </div>

            {/* Candidate flows */}
            {candidateFlows.length === 0 ? (
              <div className="py-4 text-center text-xs text-slate-500 bg-slate-50 rounded-xl border border-dashed border-slate-200">
                Chưa có Flow nào khác trong hệ thống để xâu chuỗi.
              </div>
            ) : (
              candidateFlows.map((cf) => {
                const isSelected = selectedNextFlowIds.includes(cf.id);
                const isSelf = cf.id === currentScriptId;
                return (
                  <div
                    key={cf.id}
                    onClick={() => {
                      if (!isSelf) onToggleNextFlow(cf.id);
                    }}
                    className={`p-3 rounded-2xl border text-xs flex items-center justify-between transition-all ${
                      isSelf
                        ? "border-red-200 bg-red-50/40 opacity-70 cursor-not-allowed"
                        : isSelected
                        ? "border-indigo-300 bg-indigo-50/50 cursor-pointer"
                        : "border-slate-200 bg-white hover:border-slate-300 cursor-pointer"
                    }`}
                  >

                  <div className="flex items-center gap-2.5">
                    <input
                      type="checkbox"
                      disabled={isSelf}
                      checked={isSelected}
                      onChange={() => {}} // Handled by parent div
                      className="rounded text-indigo-600 cursor-pointer"
                    />
                    <span className="font-semibold text-slate-800">{cf.name}</span>
                    <span className="text-[10px] font-bold px-1.5 py-0.2 rounded bg-slate-100 text-slate-700 uppercase">
                      {cf.kind}
                    </span>
                  </div>

                  {isSelf && (
                    <span className="text-[10px] font-bold text-red-600 flex items-center gap-1">
                      <AlertTriangle className="w-3.5 h-3.5" />
                      không cho phép — sẽ tạo vòng lặp
                    </span>
                  )}
                </div>
              );
            })
          )}
          </div>

          {/* Execution Chain Preview */}
          <div className="bg-indigo-50/40 border border-indigo-100 rounded-2xl p-4 space-y-3">
            <div className="text-[10px] uppercase font-bold text-indigo-900 tracking-wider">
              XEM TRƯỚC CHUỖI THỰC THI
            </div>
            <div className="flex flex-wrap items-center gap-2 text-xs font-semibold text-indigo-950">
              <span className="bg-white px-2.5 py-1 rounded-lg border border-indigo-200">Webhook</span>
              <ArrowRight className="w-3.5 h-3.5 text-indigo-400" />
              <span className="bg-white px-2.5 py-1 rounded-lg border border-indigo-200">Ghi đè Config (ec_target 2.4 &rarr; 1.8)</span>
              <ArrowRight className="w-3.5 h-3.5 text-indigo-400" />
              <span className="bg-white px-2.5 py-1 rounded-lg border border-indigo-200">Auto dose PH_DOWN</span>
            </div>

            <div className="pt-2 border-t border-indigo-100/80 space-y-2 text-[11px] leading-relaxed text-indigo-950/80">
              <div>
                <span className="font-bold text-indigo-900 block">THỨ TỰ THỰC THI</span>
                Config được ghi đè NGAY khi Flow này chạy xong. Flow kế tiếp trong chuỗi luôn đọc được giá trị MỚI — không phải giá trị gốc trước khi ghi đè.
              </div>
              <div className="pt-2 border-t border-indigo-100/60">
                <span className="font-bold text-indigo-900 block">KHI FLOW KẾ TIẾP BỊ TẮT HOẶC LỖI</span>
                Flow kế tiếp đang tắt &rarr; bỏ qua, ghi log. Flow kế tiếp lỗi &rarr; dừng chuỗi và ghi log; các Flow trước KHÔNG rollback. Giới hạn độ sâu tối đa 5 Flow.
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
