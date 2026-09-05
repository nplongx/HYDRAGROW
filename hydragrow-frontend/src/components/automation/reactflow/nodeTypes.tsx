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
  Database,
} from "lucide-react";

interface NodeProps {
  data: any;
  selected?: boolean;
}

export function TriggerNode({ data, selected }: NodeProps) {
  let Icon = Activity;
  let badge = "TRIGGER · SENSOR";
  let title = data.field ? `${data.field} (thời gian thực)` : "Cảm biến (thời gian thực)";
  let subtitle = data.intervalSec ? `Đọc mỗi ${data.intervalSec}s từ cảm biến` : "Đọc mỗi 30s từ cảm biến";
  let borderClass = "border-sky-300 bg-sky-50/40 text-sky-950";
  let badgeClass = "bg-sky-100 text-sky-800 border-sky-200";

  if (data.kind === "fsm") {
    Icon = Zap;
    badge = "TRIGGER · FSM";
    title = data.state ? `Giai đoạn: ${data.state}` : "Giai đoạn canh tác (FSM)";
    subtitle = "Theo dõi chuyển đổi trạng thái FSM";
    borderClass = "border-amber-300 bg-amber-50/40 text-amber-950";
    badgeClass = "bg-amber-100 text-amber-800 border-amber-200";
  } else if (data.kind === "cron") {
    Icon = Calendar;
    badge = "TRIGGER · CRON";
    title = data.expression ? `${data.expression}` : "07:00 mỗi ngày";
    subtitle = data.timezone || "Asia/Ho_Chi_Minh";
    borderClass = "border-purple-300 bg-purple-50/40 text-purple-950";
    badgeClass = "bg-purple-100 text-purple-800 border-purple-200";
  } else if (data.kind === "webhook") {
    Icon = Webhook;
    badge = "TRIGGER · WEBHOOK";
    title = "Nhận dữ liệu bên ngoài";
    subtitle = data.mode === "direct" ? "Xử lý trực tiếp" : "Kích hoạt Flow";
    borderClass = "border-indigo-300 bg-indigo-50/40 text-indigo-950";
    badgeClass = "bg-indigo-100 text-indigo-800 border-indigo-200";
  }

  return (
    <div
      className={`min-w-[190px] rounded-2xl border-2 px-3.5 py-2.5 shadow-sm bg-white transition-all cursor-pointer ${borderClass} ${
        selected ? "ring-2 ring-emerald-500 ring-offset-2 shadow-md" : ""
      }`}
    >
      <Handle type="target" position={Position.Left} className="opacity-0" />
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-1.5">
          <span className={`text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded border ${badgeClass}`}>
            {badge}
          </span>
          <Icon className="w-3.5 h-3.5 opacity-70" />
        </div>
        <div className="font-bold text-xs text-slate-900 mt-0.5 leading-tight">{title}</div>
        <div className="text-[10px] text-slate-500 leading-tight truncate max-w-[180px]">{subtitle}</div>
      </div>
      <Handle type="source" position={Position.Right} className="!w-2 !h-2 !bg-emerald-500" />
    </div>
  );
}

export function ConditionNode({ data, selected }: NodeProps) {
  const isGroup = data.type === "condition_group" || Array.isArray(data.conditions);
  const isTimeWindow = data.type === "time-window";
  
  let badge = "CONDITION";
  let title = "Điều kiện an toàn";
  let subtitle = "Kiểm tra biểu thức";

  if (isGroup) {
    const op = data.op?.toUpperCase() ?? "AND";
    badge = `CONDITION · NHÓM [${op}]`;
    title = op === "AND" ? "Tất cả đều đúng" : "Một trong số điều kiện đúng";
    subtitle = data.conditions?.length ? `${data.conditions.length} điều kiện con` : "Chưa có điều kiện con";
  } else if (isTimeWindow) {
    badge = "CONDITION · THỜI GIAN";
    title = data.field ? `${data.field} (${data.mode || "mean"})` : "Khung giờ điều kiện";
    subtitle = data.windowMin ? `Trong ${data.windowMin} phút` : "Cửa sổ thời gian";
  } else if (data.field) {
    title = `${data.field} ${data.op || ">"} ${data.value ?? ""}`;
    subtitle = "So sánh cảm biến tức thời";
  }

  return (
    <div
      className={`min-w-[190px] rounded-2xl border-2 border-amber-300 bg-white px-3.5 py-2.5 shadow-sm transition-all cursor-pointer ${
        selected ? "ring-2 ring-amber-500 ring-offset-2 shadow-md" : ""
      }`}
    >
      <Handle type="target" position={Position.Left} className="!w-2 !h-2 !bg-amber-500" />
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-1.5">
          <span className="text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded border bg-amber-100 text-amber-900 border-amber-200">
            {badge}
          </span>
          <Filter className="w-3.5 h-3.5 text-amber-600" />
        </div>
        <div className="font-bold text-xs text-slate-900 mt-0.5 leading-tight">{title}</div>
        <div className="text-[10px] text-slate-500 leading-tight truncate max-w-[180px]">{subtitle}</div>
      </div>
      <Handle type="source" position={Position.Right} className="!w-2 !h-2 !bg-amber-500" />
    </div>
  );
}

export function ActionNode({ data, selected }: NodeProps) {
  let Icon = Zap;
  let badge = "ACTION";
  let title = "Hành động thực thi";
  let subtitle = "Kích hoạt thiết bị";
  let borderClass = "border-emerald-300";
  let badgeClass = "bg-emerald-100 text-emerald-800 border-emerald-200";

  if (data.type === "alert") {
    Icon = Bell;
    badge = "ACTION · ALERT";
    title = data.message ? `Cảnh báo: ${data.message}` : "Gửi thông báo & Alert";
    subtitle = data.level ? `Mức độ: ${data.level}` : "Gửi FCM / App / Email";
    borderClass = "border-rose-300";
    badgeClass = "bg-rose-100 text-rose-800 border-rose-200";
  } else if (data.type === "control") {
    Icon = Beaker;
    badge = "ACTION · DOSE/WATER";
    title = data.pump ? `Bơm ${data.pump}` : "Định lượng dinh dưỡng / Bơm";
    subtitle = data.duration ? `Thời gian: ${data.duration}s` : "Bơm A + B hoặc tưới";
    borderClass = "border-teal-300";
    badgeClass = "bg-teal-100 text-teal-800 border-teal-200";
  } else if (data.type === "delay") {
    Icon = Clock;
    badge = "ACTION · DELAY";
    title = `Chờ ${data.duration || 10}s`;
    subtitle = "Tạm dừng trước bước kế tiếp";
    borderClass = "border-amber-300";
    badgeClass = "bg-amber-100 text-amber-800 border-amber-200";
  } else if (data.type === "chain") {
    Icon = Link2;
    badge = "ACTION · CHAIN";
    title = "Chạy tiếp Flow khác";
    subtitle = data.targetFlowName || "Kích hoạt Flow liên kết";
    borderClass = "border-indigo-300";
    badgeClass = "bg-indigo-100 text-indigo-800 border-indigo-200";
  }

  return (
    <div
      className={`min-w-[190px] rounded-2xl border-2 ${borderClass} bg-white px-3.5 py-2.5 shadow-sm transition-all cursor-pointer ${
        selected ? "ring-2 ring-emerald-500 ring-offset-2 shadow-md" : ""
      }`}
    >
      <Handle type="target" position={Position.Left} className="!w-2 !h-2 !bg-emerald-500" />
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-1.5">
          <span className={`text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded border ${badgeClass}`}>
            {badge}
          </span>
          <Icon className="w-3.5 h-3.5 opacity-80" />
        </div>
        <div className="font-bold text-xs text-slate-900 mt-0.5 leading-tight">{title}</div>
        <div className="text-[10px] text-slate-500 leading-tight truncate max-w-[180px]">{subtitle}</div>
      </div>
      <Handle type="source" position={Position.Right} className="!w-2 !h-2 !bg-emerald-500" />
    </div>
  );
}

export function ConfigNode({ data, selected }: NodeProps) {
  const isOverwrite = data?.variant === "overwrite";
  const configKey = typeof data?.configKey === "string" ? data.configKey : "";
  const saveToVariable = typeof data?.saveToVariable === "string" ? data.saveToVariable : "";
  const overrideValue = data?.overrideValue !== undefined ? String(data.overrideValue) : "";

  const badge = isOverwrite ? "CONFIG · GHI ĐÈ" : "CONFIG · ĐỌC";
  const summary = isOverwrite
    ? configKey && overrideValue
      ? `${configKey} → ${overrideValue}`
      : "Chưa cấu hình"
    : configKey && saveToVariable
      ? `${configKey} → ${saveToVariable}`
      : "Chưa cấu hình";

  const subtitle = isOverwrite
    ? "Đọc giá trị gốc trước khi ghi · Tự động khôi phục"
    : "Lưu biến tạm vào ngữ cảnh thi hành";

  const borderClass = isOverwrite
    ? "border-indigo-500 border-2 bg-indigo-50/30"
    : "border-blue-300 bg-blue-50/20";
  const badgeClass = isOverwrite
    ? "bg-indigo-600 text-white border-indigo-600"
    : "bg-blue-100 text-blue-800 border-blue-200";

  return (
    <div
      className={`min-w-[210px] rounded-2xl border px-3.5 py-2.5 shadow-sm bg-white transition-all cursor-pointer ${borderClass} ${
        selected ? "ring-2 ring-indigo-500 ring-offset-2 shadow-md" : ""
      }`}
    >
      <Handle type="target" position={Position.Left} className="!w-2 !h-2 !bg-indigo-500" />
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-1.5">
          <span className={`text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded border ${badgeClass}`}>
            {badge}
          </span>
          <Database className="w-3.5 h-3.5 text-indigo-600" />
        </div>
        <div className="font-bold text-xs text-slate-900 mt-0.5 leading-tight">{summary}</div>
        <div className="text-[10px] text-slate-500 leading-tight truncate max-w-[200px]">{subtitle}</div>
      </div>
      <Handle type="source" position={Position.Right} className="!w-2 !h-2 !bg-indigo-500" />
    </div>
  );
}

export const AUTOMATION_NODE_TYPES = {
  trigger: TriggerNode,
  condition: ConditionNode,
  action: ActionNode,
  config: ConfigNode,
};
