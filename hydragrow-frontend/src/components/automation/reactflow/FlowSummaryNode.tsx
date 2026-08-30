import { Handle, Position, type NodeProps, type Node } from '@xyflow/react';
import type { FlowNodeData } from '../../../hooks/useFlowCanvas';

/** Thẻ hiển thị 1 Flow trên canvas tổng quan. `onNodeClick` (React Flow) do
 * component cha xử lý — component này chỉ render nội dung thẻ. */
export function FlowSummaryNode({ data }: NodeProps<Node<FlowNodeData>>) {
  const { script } = data;
  const conditionCount = script.ir_json?.conditions.length ?? 0;
  const actionCount = script.ir_json?.actions.length ?? 0;

  return (
    <div
      className={`w-48 cursor-pointer rounded border-2 bg-white p-2 shadow-sm ${
        script.enabled ? 'border-emerald-500' : 'border-gray-300 opacity-60'
      }`}
    >
      <Handle type="target" position={Position.Top} style={{ opacity: 0 }} />
      <div className="flex items-center justify-between gap-1">
        <span className="truncate text-sm font-semibold">{script.name}</span>
        <span className="shrink-0 rounded bg-slate-100 px-1 text-[10px] uppercase text-slate-600">
          {script.kind}
        </span>
      </div>
      <div className="mt-1 text-xs text-gray-500">
        {script.ir_json ? `${conditionCount} điều kiện → ${actionCount} hành động` : 'Script viết tay'}
      </div>
      <Handle type="source" position={Position.Bottom} style={{ opacity: 0 }} />
    </div>
  );
}

export const AUTOMATION_FLOW_NODE_TYPES = { flowSummary: FlowSummaryNode };
