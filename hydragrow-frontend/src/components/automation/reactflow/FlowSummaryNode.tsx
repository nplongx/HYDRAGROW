import { Handle, Position, type NodeProps, type Node } from '@xyflow/react';
import type { FlowNodeData } from '../../../hooks/useFlowCanvas';
import { countLeafConditions } from '../../../lib/automation/conditionTree';

const KIND_LABEL: Record<string, string> = {
  alert: 'Alert',
  recipe_override: 'Recipe',
  action_command: 'Action',
};

const KIND_COLOR: Record<string, string> = {
  alert: 'bg-red-100 text-red-700',
  recipe_override: 'bg-sky-100 text-sky-700',
  action_command: 'bg-amber-100 text-amber-700',
};

/** Thẻ hiển thị 1 Flow trên canvas tổng quan. `onNodeClick` (React Flow) do
 * component cha xử lý — component này chỉ render nội dung thẻ. */
export function FlowSummaryNode({ data }: NodeProps<Node<FlowNodeData>>) {
  const { script } = data;
  const conditionCount = script.ir_json ? countLeafConditions(script.ir_json.conditions) : 0;
  const actionCount = script.ir_json?.actions.length ?? 0;
  const badgeColor = KIND_COLOR[script.kind] ?? 'bg-emerald-50 text-emerald-800/70';

  return (
    <div
      className={`w-52 cursor-pointer rounded-lg border-2 bg-white p-3 shadow-sm transition-shadow hover:shadow-md ${
        script.enabled ? 'border-emerald-500' : 'border-gray-200 opacity-60'
      }`}
    >
      <Handle type="target" position={Position.Top} style={{ opacity: 0 }} />
      <div className="flex items-start justify-between gap-1">
        <span className="truncate text-sm font-semibold leading-tight">{script.name}</span>
        <span className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase ${badgeColor}`}>
          {KIND_LABEL[script.kind] ?? script.kind}
        </span>
      </div>
      <div className="mt-1.5 text-xs text-gray-400">
        {script.ir_json
          ? `${conditionCount} điều kiện → ${actionCount} hành động`
          : 'Script viết tay (Rhai)'}
      </div>
      {!script.enabled && (
        <div className="mt-1 text-[10px] text-gray-400 italic">Đã tắt</div>
      )}
      <Handle type="source" position={Position.Bottom} style={{ opacity: 0 }} />
    </div>
  );
}
