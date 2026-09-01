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
    .map((a) => {
      switch (a.type) {
        case 'alert':
          return `alert (${a.level}): ${a.message}`;
        case 'advance_stage':
          return `advance_stage ${a.targetStageOffset >= 0 ? '+' : ''}${a.targetStageOffset}: ${a.reason}`;
        case 'end_season':
          return `end_season: ${a.reason}`;
        case 'dose':
          return `dose ${a.doseMl}ml (${a.pump})`;
        case 'water_on':
          return `water_on ${a.durationSec}s (${a.pump})`;
        case 'water_off':
          return `water_off (${a.pump})`;
        case 'emergency_stop':
          return 'emergency_stop';
        default:
          return 'unknown_action';
      }
    })
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
      <div className="w-72 shrink-0 border-l border-emerald-100 bg-white p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-emerald-950">Condition</h3>
          <button className="text-xs text-emerald-700/70" onClick={onClose}>
            Đóng
          </button>
        </div>
        {conditions.map((c, i) => (
          <div key={i} className="mb-2 flex gap-1">
            <select
              className="ui-input px-1 text-sm"
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
              className="ui-input px-1 text-sm"
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
              className="ui-input w-20 px-1 text-sm"
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
        <div className="w-72 shrink-0 border-l border-emerald-100 bg-white p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Action — Alert</h3>
            <button className="text-xs text-emerald-700/70" onClick={onClose}>
              Đóng
            </button>
          </div>
          <label className="mb-2 block text-xs text-emerald-800/75">
            Level
            <select
              className="ui-input mt-1"
              value={a.level}
              onChange={(e) => setAction({ ...a, level: e.target.value as typeof a.level })}
            >
              <option value="info">info</option>
              <option value="warning">warning</option>
              <option value="error">error</option>
            </select>
          </label>
          <label className="mb-2 block text-xs text-emerald-800/75">
            Title (optional)
            <input
              className="ui-input mt-1"
              value={a.title ?? ''}
              onChange={(e) => setAction({ ...a, title: e.target.value })}
            />
          </label>
          <label className="block text-xs text-emerald-800/75">
            Message
            <input className="ui-input mt-1" value={a.message} onChange={(e) => setAction({ ...a, message: e.target.value })} />
          </label>
        </div>
      );
    }

    if (kind === 'recipe_override') {
      const actionType = current?.type === 'end_season' ? 'end_season' : 'advance_stage';
      return (
        <div className="w-72 shrink-0 border-l border-emerald-100 bg-white p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold text-emerald-950">Action — Recipe</h3>
            <button className="text-xs text-emerald-700/70" onClick={onClose}>
              Đóng
            </button>
          </div>
          <label className="mb-2 block text-xs text-emerald-800/75">
            Loại hành động
            <select
              className="ui-input mt-1"
              value={actionType}
              onChange={(e) =>
                setAction(
                  e.target.value === 'end_season'
                    ? { type: 'end_season', reason: current?.type === 'end_season' ? current.reason : '' }
                    : { type: 'advance_stage', targetStageOffset: 1, reason: '' },
                )
              }
            >
              <option value="advance_stage">advance_stage</option>
              <option value="end_season">end_season</option>
            </select>
          </label>
          {actionType === 'advance_stage' ? (
            <>
              <label className="mb-2 block text-xs text-emerald-800/75">
                Target stage offset
                <input
                  type="number"
                  className="ui-input mt-1"
                  value={current?.type === 'advance_stage' ? current.targetStageOffset : 1}
                  onChange={(e) =>
                    setAction({
                      type: 'advance_stage',
                      targetStageOffset: Number(e.target.value),
                      reason: current?.type === 'advance_stage' ? current.reason : '',
                    })
                  }
                />
              </label>
              <label className="block text-xs text-emerald-800/75">
                Reason
                <input
                  className="ui-input mt-1"
                  value={current?.type === 'advance_stage' ? current.reason : ''}
                  onChange={(e) =>
                    setAction({
                      type: 'advance_stage',
                      targetStageOffset: current?.type === 'advance_stage' ? current.targetStageOffset : 1,
                      reason: e.target.value,
                    })
                  }
                />
              </label>
            </>
          ) : (
            <label className="block text-xs text-emerald-800/75">
              Reason
              <input
                className="ui-input mt-1"
                value={current?.type === 'end_season' ? current.reason : ''}
                onChange={(e) => setAction({ type: 'end_season', reason: e.target.value })}
              />
            </label>
          )}
        </div>
      );
    }

    // kind === 'action_command'
    const actionType: 'dose' | 'water_on' | 'water_off' | 'emergency_stop' =
      current?.type === 'dose' || current?.type === 'water_on' || current?.type === 'water_off' || current?.type === 'emergency_stop'
        ? current.type
        : 'dose';

    return (
      <div className="w-72 shrink-0 border-l border-emerald-100 bg-white p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-emerald-950">Action — Điều khiển</h3>
          <button className="text-xs text-emerald-700/70" onClick={onClose}>
            Đóng
          </button>
        </div>
        <label className="mb-2 block text-xs text-emerald-800/75">
          Loại hành động
          <select
            className="ui-input mt-1"
            value={actionType}
            onChange={(e) => {
              const next = e.target.value as typeof actionType;
              if (next === 'dose') setAction({ type: 'dose', pump: 'PUMP_A', doseMl: 1, pwm: 100 });
              else if (next === 'water_on') setAction({ type: 'water_on', pump: 'WATER_PUMP_IN', durationSec: 10 });
              else if (next === 'water_off') setAction({ type: 'water_off', pump: 'WATER_PUMP_IN' });
              else setAction({ type: 'emergency_stop' });
            }}
          >
            <option value="dose">dose</option>
            <option value="water_on">water_on</option>
            <option value="water_off">water_off</option>
            <option value="emergency_stop">emergency_stop</option>
          </select>
        </label>
        {actionType === 'dose' && (
          <>
            <label className="mb-2 block text-xs text-emerald-800/75">
              Bơm
              <select
                className="ui-input mt-1"
                value={current?.type === 'dose' ? current.pump : 'PUMP_A'}
                onChange={(e) =>
                  setAction({
                    type: 'dose',
                    pump: e.target.value as 'PUMP_A' | 'PUMP_B' | 'PH_UP' | 'PH_DOWN',
                    doseMl: current?.type === 'dose' ? current.doseMl : 1,
                    pwm: current?.type === 'dose' ? current.pwm : 100,
                  })
                }
              >
                <option value="PUMP_A">PUMP_A</option>
                <option value="PUMP_B">PUMP_B</option>
                <option value="PH_UP">PH_UP</option>
                <option value="PH_DOWN">PH_DOWN</option>
              </select>
            </label>
            <label className="mb-2 block text-xs text-emerald-800/75">
              Liều (ml)
              <input
                type="number"
                className="ui-input mt-1"
                value={current?.type === 'dose' ? current.doseMl : 1}
                onChange={(e) =>
                  setAction({
                    type: 'dose',
                    pump: current?.type === 'dose' ? current.pump : 'PUMP_A',
                    doseMl: Number(e.target.value),
                    pwm: current?.type === 'dose' ? current.pwm : 100,
                  })
                }
              />
            </label>
            <label className="block text-xs text-emerald-800/75">
              PWM (%)
              <input
                type="number"
                className="ui-input mt-1"
                value={current?.type === 'dose' ? current.pwm : 100}
                onChange={(e) =>
                  setAction({
                    type: 'dose',
                    pump: current?.type === 'dose' ? current.pump : 'PUMP_A',
                    doseMl: current?.type === 'dose' ? current.doseMl : 1,
                    pwm: Number(e.target.value),
                  })
                }
              />
            </label>
          </>
        )}
        {(actionType === 'water_on' || actionType === 'water_off') && (
          <>
            <label className="mb-2 block text-xs text-emerald-800/75">
              Bơm/van
              <select
                className="ui-input mt-1"
                value={current?.type === 'water_on' || current?.type === 'water_off' ? current.pump : 'WATER_PUMP_IN'}
                onChange={(e) => {
                  const pump = e.target.value as 'WATER_PUMP_IN' | 'WATER_PUMP_OUT' | 'MIST_VALVE' | 'OSAKA_PUMP';
                  if (actionType === 'water_on') {
                    setAction({ type: 'water_on', pump, durationSec: current?.type === 'water_on' ? current.durationSec : 10 });
                  } else {
                    setAction({ type: 'water_off', pump });
                  }
                }}
              >
                <option value="WATER_PUMP_IN">WATER_PUMP_IN</option>
                <option value="WATER_PUMP_OUT">WATER_PUMP_OUT</option>
                <option value="MIST_VALVE">MIST_VALVE</option>
                <option value="OSAKA_PUMP">OSAKA_PUMP</option>
              </select>
            </label>
            {actionType === 'water_on' && (
              <label className="block text-xs text-emerald-800/75">
                Thời gian (giây)
                <input
                  type="number"
                  className="ui-input mt-1"
                  value={current?.type === 'water_on' ? current.durationSec : 10}
                  onChange={(e) =>
                    setAction({
                      type: 'water_on',
                      pump: current?.type === 'water_on' ? current.pump : 'WATER_PUMP_IN',
                      durationSec: Number(e.target.value),
                    })
                  }
                />
              </label>
            )}
          </>
        )}
        {actionType === 'emergency_stop' && (
          <p className="text-xs text-emerald-800/75">Dừng toàn bộ actor ngay khi điều kiện phía trên đúng — không có tham số thêm.</p>
        )}
      </div>
    );
  }

  return null;
}
