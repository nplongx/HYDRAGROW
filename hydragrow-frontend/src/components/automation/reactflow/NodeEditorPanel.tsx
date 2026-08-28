import type { Action, AutomationIr, ComparisonOperator, Condition } from '../../../lib/automation/ir';
import { fieldsForKind } from '../../../hooks/useAutomationBuilder';

const OPERATORS: ComparisonOperator[] = ['>', '<', '>=', '<=', '==', '!='];

function summarizeConditions(conditions: Condition[]): string {
  if (conditions.length === 0) return 'Chưa cấu hình';
  return conditions.map((c) => `${c.sensor} ${c.operator} ${c.value}`).join(' và ');
}

function summarizeActions(actions: Action[]): string {
  if (actions.length === 0) return 'Chưa cấu hình';
  return actions
    .map((a) =>
      a.type === 'alert'
        ? `alert (${a.level}): ${a.message}`
        : `advance_stage ${a.targetStageOffset >= 0 ? '+' : ''}${a.targetStageOffset}: ${a.reason}`,
    )
    .join(', ');
}

export interface NodeEditorPanelProps {
  kind: AutomationIr['kind'];
  node: { id: string; type?: string; data: Record<string, unknown> };
  onChange: (nodeId: string, data: Record<string, unknown>) => void;
  onClose: () => void;
}

export function NodeEditorPanel({ kind, node, onChange, onClose }: NodeEditorPanelProps) {
  const fields = fieldsForKind(kind);

  if (node.type === 'condition') {
    const conditions = (node.data.conditions as Condition[] | undefined) ?? [];
    const update = (next: Condition[]) => onChange(node.id, { conditions: next, summary: summarizeConditions(next) });

    return (
      <div className="w-72 shrink-0 border-l p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold">Condition</h3>
          <button className="text-xs text-gray-500" onClick={onClose}>
            Đóng
          </button>
        </div>
        {conditions.map((c, i) => (
          <div key={i} className="mb-2 flex gap-1">
            <select
              className="rounded border px-1 text-sm"
              value={c.sensor}
              onChange={(e) => update(conditions.map((x, j) => (j === i ? { ...x, sensor: e.target.value } : x)))}
            >
              {fields.map((f) => (
                <option key={f} value={f}>
                  {f}
                </option>
              ))}
            </select>
            <select
              className="rounded border px-1 text-sm"
              value={c.operator}
              onChange={(e) =>
                update(conditions.map((x, j) => (j === i ? { ...x, operator: e.target.value as ComparisonOperator } : x)))
              }
            >
              {OPERATORS.map((op) => (
                <option key={op} value={op}>
                  {op}
                </option>
              ))}
            </select>
            <input
              type="number"
              className="w-20 rounded border px-1 text-sm"
              value={c.value}
              onChange={(e) => update(conditions.map((x, j) => (j === i ? { ...x, value: Number(e.target.value) } : x)))}
            />
            <button className="text-xs text-red-600" onClick={() => update(conditions.filter((_, j) => j !== i))}>
              ✕
            </button>
          </div>
        ))}
        <button
          className="text-xs text-emerald-700"
          onClick={() => update([...conditions, { sensor: fields[0], operator: '>', value: 0 }])}
        >
          + Thêm điều kiện
        </button>
      </div>
    );
  }

  if (node.type === 'action') {
    const actions = (node.data.actions as Action[] | undefined) ?? [];
    const current = actions[0];
    const setAction = (action: Action) => onChange(node.id, { actions: [action], summary: summarizeActions([action]) });

    if (kind === 'alert') {
      const a = current?.type === 'alert' ? current : { type: 'alert' as const, level: 'warning' as const, message: '' };
      return (
        <div className="w-72 shrink-0 border-l p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold">Action — Alert</h3>
            <button className="text-xs text-gray-500" onClick={onClose}>
              Đóng
            </button>
          </div>
          <label className="mb-2 block text-xs">
            Level
            <select
              className="mt-1 w-full rounded border px-1 text-sm"
              value={a.level}
              onChange={(e) => setAction({ ...a, level: e.target.value as typeof a.level })}
            >
              <option value="info">info</option>
              <option value="warning">warning</option>
              <option value="error">error</option>
            </select>
          </label>
          <label className="mb-2 block text-xs">
            Title (optional)
            <input
              className="mt-1 w-full rounded border px-1 text-sm"
              value={a.title ?? ''}
              onChange={(e) => setAction({ ...a, title: e.target.value })}
            />
          </label>
          <label className="block text-xs">
            Message
            <input
              className="mt-1 w-full rounded border px-1 text-sm"
              value={a.message}
              onChange={(e) => setAction({ ...a, message: e.target.value })}
            />
          </label>
        </div>
      );
    }

    const a =
      current?.type === 'advance_stage' ? current : { type: 'advance_stage' as const, targetStageOffset: 1, reason: '' };
    return (
      <div className="w-72 shrink-0 border-l p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold">Action — Advance Stage</h3>
          <button className="text-xs text-gray-500" onClick={onClose}>
            Đóng
          </button>
        </div>
        <label className="mb-2 block text-xs">
          Target stage offset
          <input
            type="number"
            className="mt-1 w-full rounded border px-1 text-sm"
            value={a.targetStageOffset}
            onChange={(e) => setAction({ ...a, targetStageOffset: Number(e.target.value) })}
          />
        </label>
        <label className="block text-xs">
          Reason
          <input
            className="mt-1 w-full rounded border px-1 text-sm"
            value={a.reason}
            onChange={(e) => setAction({ ...a, reason: e.target.value })}
          />
        </label>
      </div>
    );
  }

  return null;
}
