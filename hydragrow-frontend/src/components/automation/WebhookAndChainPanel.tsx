import { Copy, Link2, ArrowRight } from "lucide-react";
import { WebhookFieldMappingEditor } from "./reactflow/WebhookFieldMappingEditor";
import { NextFlowSelector } from "./NextFlowSelector";
import type { WebhookFieldMapping } from "../../lib/automation/ir";
import type { UserScript } from "../../types/automation";

interface Props {
  webhookUrl?: string;
  mode: "flow" | "direct";
  onModeChange: (mode: "flow" | "direct") => void;
  mappings: WebhookFieldMapping[];
  onMappingsChange: (mappings: WebhookFieldMapping[]) => void;
  currentScriptName: string;
  currentScriptKind: string;
  /** vd. "ec_target: 2.4 → 1.8 mS/cm" — undefined nếu Flow này không có node Ghi đè Config. */
  configOverwriteSummary?: string;
  scripts: UserScript[];
  selectedNextFlowIds: string[];
  onToggleNextFlow: (id: string) => void;
}

export function WebhookAndChainPanel({
  webhookUrl,
  mode,
  onModeChange,
  mappings,
  onMappingsChange,
  currentScriptName,
  currentScriptKind,
  configOverwriteSummary,
  scripts,
  selectedNextFlowIds,
  onToggleNextFlow,
}: Props) {
  const selectedNextFlows = scripts.filter((s) => selectedNextFlowIds.includes(s.id));

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 border-t border-emerald-100 bg-white p-4">
      <div className="rounded-2xl border border-emerald-100 bg-emerald-50/20 p-4">
        <div className="mb-3 flex items-center gap-1.5 text-xs font-bold text-emerald-950">
          <Link2 className="h-3.5 w-3.5" />
          WEBHOOK URL
        </div>
        <div className="mb-4 flex items-center gap-1.5 rounded-lg border border-emerald-200 bg-white p-2">
          <input
            readOnly
            value={webhookUrl ?? "(được cấp khi lưu Flow lần đầu)"}
            className="flex-1 bg-transparent text-xs font-mono text-emerald-950 outline-none"
          />
          <button
            type="button"
            className="rounded p-1 text-emerald-600 hover:bg-emerald-100"
            onClick={() => webhookUrl && navigator.clipboard?.writeText(webhookUrl)}
            aria-label="Sao chép webhook URL"
          >
            <Copy className="h-3.5 w-3.5" />
          </button>
        </div>
        <WebhookFieldMappingEditor
          config={{ type: "webhook", mode, fieldMappings: mappings }}
          onChange={(next) => {
            onModeChange(next.mode);
            onMappingsChange(next.fieldMappings);
          }}
        />
      </div>

      <div className="rounded-2xl border border-indigo-100 bg-indigo-50/20 p-4">
        <div className="mb-3 text-xs font-bold text-indigo-950">XEM TRƯỚC CHUỖI THỰC THI</div>
        <div className="flex flex-wrap items-center gap-2 text-xs">
          <span className="rounded-full bg-indigo-100 px-2.5 py-1 font-semibold text-indigo-900">
            Webhook: {currentScriptName} ({currentScriptKind.toUpperCase()})
          </span>
          {configOverwriteSummary && (
            <>
              <ArrowRight className="h-3.5 w-3.5 text-indigo-400" />
              <span className="rounded-full bg-amber-100 px-2.5 py-1 font-semibold text-amber-900">
                Ghi đè Config ({configOverwriteSummary})
              </span>
            </>
          )}
          {selectedNextFlows.map((flow) => (
            <span key={flow.id} className="contents">
              <ArrowRight className="h-3.5 w-3.5 text-indigo-400" />
              <span className="rounded-full bg-white border border-indigo-200 px-2.5 py-1 font-semibold text-indigo-900">
                {flow.name} ({flow.kind.toUpperCase()})
              </span>
            </span>
          ))}
          {selectedNextFlows.length === 0 && !configOverwriteSummary && (
            <span className="text-indigo-800/60 italic">Chưa chọn Flow kế tiếp — chỉ Flow này chạy.</span>
          )}
        </div>

        <div className="mt-4 border-t border-indigo-100 pt-3">
          <NextFlowSelector
            scripts={scripts}
            selectedIds={selectedNextFlowIds}
            currentScriptId={null}
            onToggle={(id) => onToggleNextFlow(id)}
          />
        </div>
      </div>
    </div>
  );
}
