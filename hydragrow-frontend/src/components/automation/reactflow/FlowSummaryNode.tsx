import { Handle, Position } from "@xyflow/react";
import type { UserScript } from "../../../types/automation";
import { Activity, Zap, Calendar, Webhook } from "lucide-react";

export interface FlowSummaryNodeProps {
  data: {
    script: UserScript;
    onClick?: () => void;
  };
}

export function FlowSummaryNode({
  data: { script, onClick },
}: FlowSummaryNodeProps) {
  const { name, kind, enabled, ir_json } = script;

  // Derive summary
  const getTriggerSummary = () => {
    if (!ir_json || !ir_json.nodes)
      return {
        label: "No trigger",
        Icon: Activity,
        badge: "bg-gray-100 text-gray-700",
      };
    const trigger = ir_json.nodes.find((n) => n.id === "trigger");
    if (!trigger)
      return {
        label: "No trigger",
        Icon: Activity,
        badge: "bg-gray-100 text-gray-700",
      };

    if (trigger.data.kind === "cron")
      return {
        label: "CRON",
        Icon: Calendar,
        badge: "bg-purple-100 text-purple-700",
      };
    if (trigger.data.kind === "webhook")
      return {
        label: "WEBHOOK",
        Icon: Webhook,
        badge: "bg-indigo-100 text-indigo-700",
      };
    if (trigger.data.kind === "sensor")
      return {
        label: "SENSOR",
        Icon: Activity,
        badge: "bg-blue-100 text-blue-700",
      };
    if (trigger.data.kind === "fsm")
      return {
        label: "FSM",
        Icon: Zap,
        badge: "bg-orange-100 text-orange-700",
      };
    return {
      label: "TRIGGER",
      Icon: Activity,
      badge: "bg-gray-100 text-gray-700",
    };
  };

  const triggerInfo = getTriggerSummary();
  const kindBadge = kind === "alert" ? "Cảnh báo" : "Hành động";
  const kindColors =
    kind === "alert"
      ? "bg-amber-100 text-amber-800"
      : "bg-emerald-100 text-emerald-800";

  return (
    <div
      className={`ui-card p-3 w-64 ${!enabled ? "opacity-50 grayscale" : "hover:shadow-md cursor-pointer transition-shadow"} border-2 ${enabled ? "border-emerald-200" : "border-gray-200"}`}
      onClick={onClick}
    >
      <Handle type="target" position={Position.Top} className="opacity-0" />

      <div className="flex justify-between items-start mb-2">
        <h3 className="font-semibold text-sm truncate max-w-[150px]">{name}</h3>
        <span
          className={`text-[10px] font-bold px-1.5 py-0.5 rounded ${kindColors}`}
        >
          {kindBadge}
        </span>
      </div>

      <div className="flex gap-2 items-center mb-1">
        <div
          className={`flex items-center gap-1 text-[10px] font-semibold px-1.5 py-0.5 rounded-full ${triggerInfo.badge}`}
        >
          <triggerInfo.Icon className="w-3 h-3" /> {triggerInfo.label}
        </div>
      </div>

      <div className="text-xs text-gray-500 truncate">
        {ir_json?.nodes?.length ? `${ir_json.nodes.length} nodes` : "No nodes"}
      </div>

      {!enabled && (
        <div className="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-gray-800 text-white text-xs px-2 py-1 rounded font-bold uppercase tracking-wider opacity-90">
          Đã tắt
        </div>
      )}

      <Handle type="source" position={Position.Bottom} className="opacity-0" />
    </div>
  );
}
