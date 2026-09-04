import { Handle, Position } from "@xyflow/react";
import {
  Activity,
  Beaker,
  Bell,
  Link2,
  Zap,
  Clock,
  Filter,
  Calendar,
  Webhook,
} from "lucide-react";

interface NodeProps {
  data: any;
  selected?: boolean;
}

export function TriggerNode({ data, selected }: NodeProps) {
  let Icon = Activity;
  let label = "Trigger (sensor)";
  let color = "text-blue-600 bg-blue-50 border-blue-200";

  if (data.kind === "fsm") {
    Icon = Zap;
    label = "Trigger (FSM)";
    color = "text-amber-600 bg-amber-50 border-amber-200";
  } else if (data.kind === "cron") {
    Icon = Calendar;
    label = "Trigger (Lịch)";
    color = "text-purple-600 bg-purple-50 border-purple-200";
  } else if (data.kind === "webhook") {
    Icon = Webhook;
    label = "Trigger (Webhook)";
    color = "text-indigo-600 bg-indigo-50 border-indigo-200";
  }

  return (
    <div
      className={`rounded-xl border-2 px-4 py-3 shadow-sm ${color} ${selected ? "ring-2 ring-blue-500 ring-offset-2" : ""}`}
    >
      <Handle type="target" position={Position.Top} className="opacity-0" />
      <div className="flex items-center gap-2">
        <Icon className="h-5 w-5" />
        <span className="font-semibold">{label}</span>
      </div>
      <Handle type="source" position={Position.Bottom} />
    </div>
  );
}

export function ConditionNode({ data, selected }: NodeProps) {
  const isGroup = data.type === "condition_group";
  const label = isGroup
    ? `Group (${data.op})`
    : data.field
      ? `${data.field} ${data.op} ${data.value}`
      : "Điều kiện mới";

  return (
    <div
      className={`rounded-xl border-2 px-4 py-3 shadow-sm bg-white border-amber-200 text-amber-900 ${selected ? "ring-2 ring-amber-500 ring-offset-2" : ""}`}
    >
      <Handle type="target" position={Position.Top} />
      <div className="flex items-center gap-2">
        <Filter className="h-4 w-4 text-amber-500" />
        <span className="font-semibold text-sm">{label}</span>
      </div>
      <Handle type="source" position={Position.Bottom} />
    </div>
  );
}

export function ActionNode({ data, selected }: NodeProps) {
  let Icon = Zap;
  let label = "Hành động mới";
  let color = "text-emerald-700 bg-emerald-50 border-emerald-200";

  if (data.type === "alert") {
    Icon = Bell;
    label = `Cảnh báo: ${data.message || "..."}`;
    color = "text-rose-600 bg-rose-50 border-rose-200";
  } else if (data.type === "control") {
    Icon = Beaker;
    label = `Bơm: ${data.duration ? data.duration + "s" : "..."}`;
  } else if (data.type === "delay") {
    Icon = Clock;
    label = `Chờ: ${data.duration ? data.duration + "s" : "..."}`;
    color = "text-slate-600 bg-slate-50 border-slate-200";
  } else if (data.type === "fsm") {
    Icon = Zap;
    label = `Chuyển giai đoạn`;
  } else if (data.type === "chain") {
    Icon = Link2;
    label = `Kích hoạt Flow khác`;
  }

  return (
    <div
      className={`rounded-xl border-2 px-4 py-3 shadow-sm ${color} ${selected ? "ring-2 ring-emerald-500 ring-offset-2" : ""}`}
    >
      <Handle type="target" position={Position.Top} />
      <div className="flex items-center gap-2">
        <Icon className="h-5 w-5" />
        <span className="font-semibold">{label}</span>
      </div>
      <Handle type="source" position={Position.Bottom} className="opacity-0" />
    </div>
  );
}

export const AUTOMATION_NODE_TYPES = {
  trigger: TriggerNode,
  condition: ConditionNode,
  action: ActionNode,
};
