import { useState } from 'react';
import type { Condition, ConditionGroup, ConditionOrGroup, ComparisonOperator } from '../../../lib/automation/ir';
import { VariableCombobox } from './VariableCombobox';
import { Segmented } from './ConfigPanelUI';

const OPERATORS: ComparisonOperator[] = ['>', '<', '>=', '<=', '==', '!='];

function isGroup(c: ConditionOrGroup): c is ConditionGroup {
  return 'op' in c;
}

const RANGE_MODES: { value: Condition['mode']; label: string }[] = [
  { value: 'instant', label: 'Tức thời' },
  { value: 'mean', label: 'Trung bình (mean)' },
  { value: 'min', label: 'Nhỏ nhất (min)' },
  { value: 'max', label: 'Lớn nhất (max)' },
];
const DEFAULT_WINDOW_SEC = 900; // 15 phút — khớp mock Figma frame 04

function LeafEditor({
  condition,
  fields,
  availableVariables,
  onChange,
  onRemove,
}: {
  condition: Condition;
  fields: readonly string[];
  availableVariables: readonly string[];
  onChange: (c: Condition) => void;
  onRemove: () => void;
}) {
  const mode = condition.mode ?? 'instant';
  const [localVariableMode, setLocalVariableMode] = useState<boolean | null>(null);
  const usesVariable = localVariableMode ?? (condition.valueVariable !== undefined);
  // The sensor combobox suggests the fixed fields for this automation kind
  // first, then any context variables discovered upstream on the canvas
  // (e.g. a Config·Read node's "Lưu vào biến"), de-duplicated.
  const sensorSuggestions = Array.from(new Set([...fields, ...availableVariables]));

  return (
    <div className="mb-2 flex flex-col gap-1 rounded border border-emerald-100 p-1.5">
      <div className="flex items-center gap-1">
        <VariableCombobox
          id={`sensor-${condition.sensor || 'new'}-${Math.random().toString(36).slice(2, 8)}`}
          ariaLabel="Cảm biến"
          value={condition.sensor}
          availableVariables={sensorSuggestions}
          onChange={(raw) => onChange({ ...condition, sensor: raw })}
          className="ui-input px-1 py-1 text-xs"
        />
        <select
          aria-label="Toán tử"
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
        {usesVariable ? (
          <VariableCombobox
            id={`value-var-${condition.sensor || 'new'}`}
            ariaLabel="Biến giá trị"
            value={condition.valueVariable ?? ''}
            availableVariables={availableVariables}
            onChange={(raw) => onChange({ ...condition, valueVariable: raw })}
            className="ui-input w-28 px-1 py-1 text-xs"
          />
        ) : (
          <input
            aria-label="Giá trị"
            type="number"
            className="ui-input w-20 px-1 py-1 text-xs"
            value={condition.value}
            onChange={(e) => onChange({ ...condition, value: Number(e.target.value) })}
          />
        )}
        <button
          type="button"
          className="p-1 text-[10px] font-semibold text-emerald-700 hover:text-emerald-900"
          onClick={() => {
            const nextUsesVariable = !usesVariable;
            setLocalVariableMode(nextUsesVariable);
            onChange({
              ...condition,
              valueVariable: nextUsesVariable ? (condition.valueVariable ?? availableVariables[0] ?? '') : undefined,
            });
          }}
        >
          {usesVariable ? 'Dùng số' : 'Dùng biến'}
        </button>
        <button
          type="button"
          className="p-1 text-xs font-bold text-red-600 hover:text-red-800"
          onClick={onRemove}
        >
          ✕
        </button>
      </div>
      <div className="flex items-center gap-1">
        <label htmlFor={`mode-${condition.sensor}`} className="sr-only">Chế độ đọc</label>
        <select
          id={`mode-${condition.sensor}`}
          aria-label="Chế độ đọc"
          className="ui-input px-1 py-1 text-xs"
          value={mode}
          onChange={(e) => {
            const nextMode = e.target.value as Condition['mode'];
            onChange({
              ...condition,
              mode: nextMode,
              windowSec: nextMode === 'instant' ? undefined : (condition.windowSec ?? DEFAULT_WINDOW_SEC),
            });
          }}
        >
          {RANGE_MODES.map((m) => (
            <option key={m.value} value={m.value}>{m.label}</option>
          ))}
        </select>
        {mode !== 'instant' && (
          <>
            <label htmlFor={`window-${condition.sensor}`} className="sr-only">Cửa sổ (phút)</label>
            <input
              id={`window-${condition.sensor}`}
              aria-label="Cửa sổ (phút)"
              type="number"
              min={1}
              className="ui-input w-16 px-1 py-1 text-xs"
              value={Math.round((condition.windowSec ?? DEFAULT_WINDOW_SEC) / 60)}
              onChange={(e) => onChange({ ...condition, windowSec: Math.max(1, Number(e.target.value)) * 60 })}
            />
            <span className="text-[11px] text-emerald-800/70">phút</span>
          </>
        )}
      </div>
    </div>
  );
}

export interface ConditionGroupEditorProps {
  group: ConditionGroup;
  fields: readonly string[];
  /** Variable names in scope at this node (trigger fields + upstream
   * Config·Read saveToVariable names). Defaults to `[]` so existing callers
   * that don't yet compute this (e.g. older tests) keep working unchanged. */
  availableVariables?: readonly string[];
  onChange: (next: ConditionGroup) => void;
  isRoot?: boolean;
}

export function ConditionGroupEditor({
  group,
  fields,
  availableVariables = [],
  onChange,
  isRoot,
}: ConditionGroupEditorProps) {
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
        <Segmented
          options={[
            { value: "and", label: "AND — tất cả đúng" },
            { value: "or", label: "OR — bất kỳ đúng" },
          ]}
          value={group.op}
          onChange={(op) => onChange({ ...group, op })}
        />
      </div>
      {group.children.map((child, i) =>
        isGroup(child) ? (
          <ConditionGroupEditor
            key={i}
            group={child}
            fields={fields}
            availableVariables={availableVariables}
            onChange={(next) => setChild(i, next)}
          />
        ) : (
          <LeafEditor
            key={i}
            condition={child}
            fields={fields}
            availableVariables={availableVariables}
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
