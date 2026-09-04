import type { AutomationIr } from '../../lib/automation/ir';
import type { UserScript } from '../../types/automation';
import { useAutomationScripts, useDeleteAutomationScript, useUpdateAutomationScript } from '../../hooks/useAutomationScripts';

export interface ScriptListPanelProps {
  deviceId: string;
  onLoad: (ir: AutomationIr) => void;
}

/** Pure so it's unit-testable without mounting the panel. A hand-written
 * Rhai script has `ir_json: null` and can't be restored into the builder. */
export function canLoadIntoBuilder(script: Pick<UserScript, 'ir_json'>): boolean {
  return script.ir_json !== null;
}

function ScriptRow({ deviceId, script, onLoad }: { deviceId: string; script: UserScript; onLoad: (ir: AutomationIr) => void }) {
  const updateScript = useUpdateAutomationScript(deviceId, script.id);
  const deleteScript = useDeleteAutomationScript(deviceId);

  return (
    <li className="flex items-center justify-between gap-2 text-sm">
      <span>
        {script.name} <span className="text-xs text-emerald-700/60">({script.kind})</span>
      </span>
      <div className="flex gap-2">
        <button
          className="text-xs text-emerald-700 disabled:text-emerald-800/30"
          disabled={!canLoadIntoBuilder(script)}
          title={canLoadIntoBuilder(script) ? undefined : 'Script viết tay, không thể mở lại trong visual builder'}
          onClick={() => script.ir_json && onLoad(script.ir_json)}
        >
          Load
        </button>
        <button
          className="text-xs text-amber-700"
          disabled={updateScript.isPending}
          onClick={() =>
            updateScript.mutate({
              kind: script.kind,
              name: script.name,
              source: script.source,
              enabled: !script.enabled,
              ir_json: script.ir_json ?? undefined,
            })
          }
        >
          {script.enabled ? 'Tắt' : 'Bật'}
        </button>
        <button
          className="text-xs text-red-600"
          disabled={deleteScript.isPending}
          onClick={() => {
            if (confirm(`Xóa automation "${script.name}"?`)) deleteScript.mutate(script.id);
          }}
        >
          Xóa
        </button>
      </div>
    </li>
  );
}

export function ScriptListPanel({ deviceId, onLoad }: ScriptListPanelProps) {
  const { data: scripts } = useAutomationScripts(deviceId);

  if (!scripts || scripts.length === 0) {
    return <div className="border-t p-2 text-sm text-emerald-800/70">Chưa có automation nào.</div>;
  }

  return (
    <div className="border-t p-2">
      <h3 className="mb-2 text-sm font-semibold">Automations đã lưu</h3>
      <ul className="flex flex-col gap-1">
        {scripts.map((s) => (
          <ScriptRow key={s.id} deviceId={deviceId} script={s} onLoad={onLoad} />
        ))}
      </ul>
    </div>
  );
}
