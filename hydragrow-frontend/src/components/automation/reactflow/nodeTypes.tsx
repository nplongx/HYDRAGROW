import { Handle, Position, type NodeProps } from '@xyflow/react';

function BaseNode({ label, color, children }: { label: string; color: string; children?: React.ReactNode }) {
  return (
    <div className="rounded-lg border-2 bg-white px-3 py-2 shadow-sm shadow-emerald-950/5" style={{ borderColor: color }}>
      <Handle type="target" position={Position.Top} />
      <div className="text-xs font-semibold" style={{ color }}>
        {label}
      </div>
      {children}
      <Handle type="source" position={Position.Bottom} />
    </div>
  );
}

export function SensorNode() {
  return <BaseNode label="Sensor Input" color="#0284c7" />;
}

export function ConditionNode({ data }: NodeProps) {
  const summary = (data as { summary?: string }).summary ?? 'Chưa cấu hình điều kiện';
  return (
    <BaseNode label="Condition" color="#d97706">
      <div className="text-sm">{summary}</div>
    </BaseNode>
  );
}

export function DelayNode({ data }: NodeProps) {
  const seconds = (data as { seconds?: number }).seconds ?? 0;
  return (
    <BaseNode label="Delay" color="#7c3aed">
      <div className="text-sm">{seconds}s</div>
    </BaseNode>
  );
}

export function ActionNode({ data }: NodeProps) {
  const summary = (data as { summary?: string }).summary ?? 'Chưa cấu hình action';
  return (
    <BaseNode label="Action" color="#dc2626">
      <div className="text-sm">{summary}</div>
    </BaseNode>
  );
}

export const AUTOMATION_NODE_TYPES = {
  sensor: SensorNode,
  condition: ConditionNode,
  delay: DelayNode,
  action: ActionNode,
};
