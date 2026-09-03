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

export function FlowSummaryNode({ data }: NodeProps<Node<FlowNodeData>>) {
  const { script } = data;
  const conditionCount = script.ir_json ? countLeafConditions(script.ir_json.conditions) : 0;
  const actionCount = script.ir_json?.actions?.length ?? 0;
  const badgeColor = KIND_COLOR[script.kind] ?? 'bg-emerald-50 text-emerald-800/70';

  let triggerBadge = null;
  // trigger type is constrained, cast to any to allow 'cron' since the IR schema might not include it explicitly if it was typed earlier.
  if ((script.ir_json?.trigger?.type as any) === 'cron') {
    triggerBadge = (
      <span className="ml-2 rounded-full bg-teal-100 px-2 py-0.5 text-[10px] font-bold uppercase text-teal-800">
        CRON
      </span>
    );
  } else if ((script.ir_json?.trigger?.type as any) === 'webhook') {
    triggerBadge = (
      <span className="ml-2 rounded-full bg-indigo-100 px-2 py-0.5 text-[10px] font-bold uppercase text-indigo-800">
        WEBHOOK
      </span>
    );
  }

  return (
    <div
      className={`w-52 cursor-pointer rounded-lg border-2 bg-white p-3 shadow-sm transition-shadow hover:shadow-md ${
        script.enabled ? 'border-emerald-500' : 'border-gray-200 opacity-60'
      }`}
    >
      <Handle type="target" position={Position.Top} style={{ opacity: 0 }} />
      <div className="flex items-start justify-between gap-1">
        <span className="truncate text-sm font-semibold leading-tight">
          {script.name}
        </span>
        <span className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase ${badgeColor}`}>
          {KIND_LABEL[script.kind] ?? script.kind}
        </span>
      </div>
      <div className="mt-1.5 flex flex-wrap items-center text-xs text-gray-400">
        <span className="truncate">
          {script.ir_json
            ? `${conditionCount} điều kiện → ${actionCount} hành động`
            : 'Script viết tay (Rhai)'}
        </span>
        {triggerBadge}
      </div>
      {!script.enabled && (
        <div className="mt-1 text-[10px] text-gray-400 italic">Đã tắt</div>
      )}
      <Handle type="source" position={Position.Bottom} style={{ opacity: 0 }} />
    </div>
  );
}
