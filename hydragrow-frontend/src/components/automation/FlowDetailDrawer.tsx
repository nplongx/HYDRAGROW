import { useMemo, useState } from 'react';
import toast from 'react-hot-toast';
import { BlockLogicEditor } from './BlockLogicEditor';
import { compileToRhai } from '../../lib/automation/compileToRhai';
import { AutomationIrSchema, type Action, type AutomationIr, type Condition } from '../../lib/automation/ir';
import type { UserScript } from '../../types/automation';
import {
  useCreateAutomationScript,
  useDeleteAutomationScript,
  useUpdateAutomationScript,
  useValidateAutomationScript,
} from '../../hooks/useAutomationScripts';

export interface FlowDetailDrawerProps {
  deviceId: string;
  /** 'new' khi tạo Flow mới; một `UserScript` khi mở chi tiết Flow đã lưu. */
  script: UserScript | 'new';
  onClose: () => void;
}

const TRIGGER_FOR_KIND: Record<AutomationIr['kind'], AutomationIr['trigger']> = {
  alert: { type: 'sensor' },
  recipe_override: { type: 'fsm' },
  action_command: { type: 'sensor' },
};

/** Pure — tách riêng để test không cần mount component hay mock react-query,
 * theo đúng pattern `canLoadIntoBuilder` đã có trong ScriptListPanel.tsx. */
export function buildAutomationIr(
  kind: AutomationIr['kind'],
  blocklyResult: { conditions: Condition[]; actions: Action[] },
): AutomationIr {
  return {
    kind,
    trigger: TRIGGER_FOR_KIND[kind],
    conditions: blocklyResult.conditions,
    actions: blocklyResult.actions,
    nodes: [],
    edges: [],
  };
}

export function FlowDetailDrawer({ deviceId, script, onClose }: FlowDetailDrawerProps) {
  const isNew = script === 'new';
  const [name, setName] = useState(isNew ? 'Flow mới' : script.name);
  const [kind, setKind] = useState<AutomationIr['kind']>(isNew ? 'alert' : script.kind);
  const [enabled, setEnabled] = useState(isNew ? true : script.enabled);
  const [blocklyResult, setBlocklyResult] = useState<{ conditions: Condition[]; actions: Action[] }>(
    !isNew && script.ir_json
      ? { conditions: script.ir_json.conditions, actions: script.ir_json.actions }
      : { conditions: [], actions: [] },
  );

  const validateScript = useValidateAutomationScript(deviceId);
  const createScript = useCreateAutomationScript(deviceId);
  // Luôn gọi hook (Rules of Hooks) — id rỗng vô hại vì nhánh isNew không dùng updateScript.
  const updateScript = useUpdateAutomationScript(deviceId, isNew ? '' : script.id);
  const deleteScript = useDeleteAutomationScript(deviceId);

  const hasLegacyGraph = !isNew && (script.ir_json?.nodes.length ?? 0) > 0;

  const initialConditions = useMemo(
    () => (!isNew && script.ir_json ? script.ir_json.conditions : []),
    [isNew, script],
  );
  const initialActions = useMemo(
    () => (!isNew && script.ir_json ? script.ir_json.actions : []),
    [isNew, script],
  );

  const handleSave = async () => {
    const ir = buildAutomationIr(kind, blocklyResult);
    const parsed = AutomationIrSchema.safeParse(ir);
    if (!parsed.success) {
      toast.error(`IR không hợp lệ: ${parsed.error.issues[0]?.message}`);
      return;
    }
    const source = compileToRhai(parsed.data);
    const validation = await validateScript.mutateAsync({
      kind: parsed.data.kind,
      name,
      source,
      ir_json: parsed.data,
    });
    if (!validation.valid) {
      toast.error(`Script không hợp lệ: ${validation.error}`);
      return;
    }
    if (isNew) {
      await createScript.mutateAsync({ kind: parsed.data.kind, name, source, enabled, ir_json: parsed.data });
    } else {
      await updateScript.mutateAsync({ kind: parsed.data.kind, name, source, enabled, ir_json: parsed.data });
    }
    toast.success('Đã lưu Flow');
    onClose();
  };

  const handleDelete = () => {
    if (isNew) return;
    if (!confirm(`Xóa Flow "${script.name}"?`)) return;
    deleteScript.mutate(script.id, { onSuccess: onClose });
  };

  return (
    <div className="fixed inset-y-0 right-0 z-20 flex w-[36rem] flex-col gap-2 border-l bg-white p-4 shadow-xl">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">{isNew ? 'Flow mới' : `Sửa: ${script.name}`}</h2>
        <button className="text-sm text-gray-500" onClick={onClose}>
          Đóng
        </button>
      </div>
      <div className="flex items-center gap-2">
        <input className="flex-1 rounded border px-2 py-1" value={name} onChange={(e) => setName(e.target.value)} />
        <select
          className="rounded border px-2 py-1 text-sm"
          value={kind}
          onChange={(e) => setKind(e.target.value as AutomationIr['kind'])}
        >
          <option value="alert">Alert</option>
          <option value="recipe_override">Recipe Override</option>
          <option value="action_command">Action Command</option>
        </select>
        <label className="flex items-center gap-1 text-xs">
          <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
          Bật
        </label>
      </div>
      {hasLegacyGraph && (
        <div className="rounded border border-amber-300 bg-amber-50 p-2 text-xs text-amber-800">
          Flow này được tạo bằng chế độ node-graph cũ. Lưu lại từ đây sẽ chuyển nó sang Blockly —
          bố cục node-graph gốc sẽ không còn dùng được sau khi lưu.
        </div>
      )}
      <div className="flex-1 rounded border p-2">
        <BlockLogicEditor
          kind={kind}
          onChange={setBlocklyResult}
          initialConditions={initialConditions}
          initialActions={initialActions}
          className="h-full w-full"
        />
      </div>
      <div className="flex justify-between">
        {!isNew ? (
          <button className="rounded bg-red-50 px-3 py-1 text-sm text-red-600" onClick={handleDelete}>
            Xóa Flow
          </button>
        ) : (
          <span />
        )}
        <button
          className="rounded bg-emerald-600 px-3 py-1 text-white disabled:opacity-50"
          disabled={createScript.isPending || updateScript.isPending}
          onClick={handleSave}
        >
          Lưu Flow
        </button>
      </div>
    </div>
  );
}
