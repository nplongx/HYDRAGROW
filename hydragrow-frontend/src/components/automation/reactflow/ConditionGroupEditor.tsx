import type { Condition, ConditionGroup, ConditionOrGroup, ComparisonOperator } from '../../../lib/automation/ir';

const OPERATORS: ComparisonOperator[] = ['>', '<', '>=', '<=', '==', '!='];

function isGroup(c: ConditionOrGroup): c is ConditionGroup {
  return 'op' in c;
}

function LeafEditor({
  condition,
  fields,
  onChange,
  onRemove,
}: {
  condition: Condition;
  fields: readonly string[];
  onChange: (c: Condition) => void;
  onRemove: () => void;
}) {
  return (
    <div className="mb-2 flex items-center gap-1">
      <select
        className="ui-input px-1 py-1 text-xs"
        value={condition.sensor}
        onChange={(e) => onChange({ ...condition, sensor: e.target.value })}
      >
        {fields.map((f) => (
          <option key={f} value={f}>
            {f}
          </option>
        ))}
      </select>
      <select
        className="ui-input px-1 py-1 text-xs"
        value={condition.operator}
        onChange={(e) => onChange({ ...condition, operator: e.target.value as ComparisonOperator })}
      >
        {OPERATORS.map((op) => (
          <option key={op} value={op}>
            {op}
          </option>
        ))}
      </select>
      <input
        type="number"
        className="ui-input w-20 px-1 py-1 text-xs"
        value={condition.value}
        onChange={(e) => onChange({ ...condition, value: Number(e.target.value) })}
      />
      <button
        type="button"
        className="p-1 text-xs font-bold text-red-600 hover:text-red-800"
        onClick={onRemove}
      >
        ✕
      </button>
    </div>
  );
}

export interface ConditionGroupEditorProps {
  group: ConditionGroup;
  fields: readonly string[];
  onChange: (next: ConditionGroup) => void;
  isRoot?: boolean;
}

export function ConditionGroupEditor({ group, fields, onChange, isRoot }: ConditionGroupEditorProps) {
  const setChild = (index: number, next: ConditionOrGroup) => {
    onChange({ ...group, children: group.children.map((c, i) => (i === index ? next : c)) });
  };
  const removeChild = (index: number) => {
    onChange({ ...group, children: group.children.filter((_, i) => i !== index) });
  };
  const addLeaf = () => {
    onChange({ ...group, children: [...group.children, { sensor: fields[0], operator: '>', value: 0 }] });
  };
  const addGroup = () => {
    onChange({ ...group, children: [...group.children, { op: 'and', children: [] }] });
  };

  return (
    <div className={isRoot ? '' : 'ml-3 border-l border-emerald-200 pl-3 my-2'}>
      <div className="mb-2 flex items-center gap-2">
        {!isRoot && <span className="text-xs font-medium text-emerald-800/70">Nhóm con</span>}
        <div className="flex overflow-hidden rounded border border-emerald-200 text-xs">
          <button
            type="button"
            aria-pressed={group.op === 'and'}
            className={`px-2 py-1 font-semibold transition-colors ${
              group.op === 'and' ? 'bg-emerald-600 text-white' : 'bg-white text-emerald-800 hover:bg-emerald-50'
            }`}
            onClick={() => onChange({ ...group, op: 'and' })}
          >
            AND
          </button>
          <button
            type="button"
            aria-pressed={group.op === 'or'}
            className={`px-2 py-1 font-semibold transition-colors ${
              group.op === 'or' ? 'bg-emerald-600 text-white' : 'bg-white text-emerald-800 hover:bg-emerald-50'
            }`}
            onClick={() => onChange({ ...group, op: 'or' })}
          >
            OR
          </button>
        </div>
      </div>
      {group.children.map((child, i) =>
        isGroup(child) ? (
          <ConditionGroupEditor
            key={i}
            group={child}
            fields={fields}
            onChange={(next) => setChild(i, next)}
          />
        ) : (
          <LeafEditor
            key={i}
            condition={child}
            fields={fields}
            onChange={(next) => setChild(i, next)}
            onRemove={() => removeChild(i)}
          />
        ),
      )}
      <div className="flex gap-3 text-xs mt-2">
        <button type="button" className="font-medium text-emerald-700 hover:text-emerald-900" onClick={addLeaf}>
          + Thêm điều kiện
        </button>
        <button type="button" className="font-medium text-emerald-700 hover:text-emerald-900" onClick={addGroup}>
          + Thêm nhóm con (AND/OR)
        </button>
      </div>
    </div>
  );
}
