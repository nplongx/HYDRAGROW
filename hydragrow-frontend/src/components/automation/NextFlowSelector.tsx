import { UserScript } from "../../types/automation";
import { wouldCreateCycle } from "../../lib/automation/flowCycle";

interface NextFlowSelectorProps {
  scripts: UserScript[];
  selectedIds: string[];
  currentScriptId: string | null;
  onToggle: (id: string, selected: boolean) => void;
  // Make check optional, falling back to false if not provided, allowing parent to handle it if needed
  allScripts?: UserScript[];
  /** Whether child flows receive a COPY of this flow's execution context.
   * Stored in `AutomationIr.chainConfig.passContextVariables` — see
   * "Known scope boundary" in the frontend IR/UI implementation plan for
   * what this does and does not enforce yet. */
  passContextVariables?: boolean;
  onTogglePassContext?: (next: boolean) => void;
}

export function NextFlowSelector({
  scripts,
  selectedIds,
  currentScriptId,
  onToggle,
  allScripts = scripts,
  passContextVariables = false,
  onTogglePassContext,
}: NextFlowSelectorProps) {
  return (
    <div className="rounded-lg border border-emerald-100 p-2 mt-4">
      <p className="mb-1 text-xs font-semibold text-emerald-950">
        Flow kế tiếp sau khi chạy xong
      </p>
      {scripts.map((s) => {
        const isChecked = selectedIds.includes(s.id);

        // If currentScriptId is null, it's a new script, cycle is only possible if there's self-reference
        // which would require knowing its future ID, so we pass a dummy ID for new scripts
        const effectiveScriptId = currentScriptId || "__new__";

        const isCycle =
          !isChecked &&
          wouldCreateCycle(effectiveScriptId, selectedIds, s.id, allScripts);

        return (
          <label
            key={s.id}
            className="flex items-center gap-1 text-xs text-emerald-800/75 cursor-pointer"
          >
            <input
              type="checkbox"
              checked={isChecked}
              disabled={isCycle}
              onChange={(e) => onToggle(s.id, e.target.checked)}
              className="mr-2"
            />
            <span>{s.name}</span>
            {isCycle && (
              <span className="ml-1 text-[11px] font-medium text-amber-600">
                không cho phép — sẽ tạo vòng lặp
              </span>
            )}
          </label>
        );
      })}
      <label className="mt-2 flex items-center gap-2 border-t border-emerald-100 pt-2 text-xs text-emerald-800/75 cursor-pointer">
        <input
          type="checkbox"
          aria-label="Truyền biến ngữ cảnh sang flow tiếp theo"
          checked={passContextVariables}
          onChange={(e) => onTogglePassContext?.(e.target.checked)}
        />
        Truyền biến ngữ cảnh sang flow tiếp theo
      </label>
    </div>
  );
}
